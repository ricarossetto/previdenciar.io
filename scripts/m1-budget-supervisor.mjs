#!/usr/bin/env node
/**
 * Supervisor operacional da autorização M1 de 4 GiB.
 *
 * Não substitui o run-with-disk-budget/ledger antigo e nunca o altera.
 * Este ledger é separado, inicia na medição da autorização registrada em
 * AUTORIZACAO-ORCAMENTO-M1-20261103.md e conta apenas aumentos positivos
 * (high-water), sem crédito por reduções.
 */
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import process from 'node:process';
import { spawn, spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { measureTree } from './run-with-disk-budget.mjs';

export const MIB = 1024 * 1024;
export const GIB = 1024 * MIB;
export const CAP_BYTES = 4 * GIB;
export const RESERVE_BYTES = 256 * MIB;
export const FLOOR_BYTES = 10 * GIB;
export const SCHEMA = 'm1-budget-supervisor/v1';

const fail = (code, message) => {
  const error = new Error(message);
  error.code = code;
  throw error;
};

const expand = (value) => String(value).replace(/%([^%]+)%/g, (_, name) => process.env[name] ?? `%${name}%`);
const sha256 = (data) => crypto.createHash('sha256').update(data).digest('hex');
const readJson = (file) => JSON.parse(fs.readFileSync(file, 'utf8'));

function inside(child, parent) {
  const rel = path.relative(path.resolve(parent), path.resolve(child));
  return rel !== '' && !rel.startsWith('..') && !path.isAbsolute(rel);
}

function rootsFromBaseline(baseline) {
  if (!Array.isArray(baseline.roots) || baseline.roots.length === 0) fail('baseline', 'roots ausentes');
  return baseline.roots.map((root) => {
    if (!root || typeof root.name !== 'string' || typeof root.path !== 'string' || !Number.isSafeInteger(root.start_bytes)) {
      fail('baseline', 'raiz inválida');
    }
    return { name: root.name, path: path.resolve(expand(root.path)), start_bytes: root.start_bytes };
  });
}

export function measureRoots(roots) {
  const paths = roots.map((root) => root.path);
  return roots.map((root) => {
    const excludes = paths.filter((candidate) => inside(candidate, root.path));
    let result;
    try {
      result = measureTree(root.path, { excludes });
    } catch (error) {
      fail('source', `${root.name}: ${error.message}`);
    }
    if (!result.exists) fail('source', `${root.name}: caminho ausente`);
    return { name: root.name, path: root.path, bytes: result.bytes };
  });
}

export function measureVhd(vhd) {
  if (!Array.isArray(vhd) || vhd.length === 0) fail('baseline', 'VHD ausente');
  return vhd.map((item) => {
    const file = path.resolve(expand(item.path));
    let stat;
    try { stat = fs.statSync(file); } catch { fail('source', `VHD ausente: ${item.path}`); }
    if (!stat.isFile()) fail('source', `VHD não é arquivo: ${item.path}`);
    return { name: item.name, path: file, bytes: stat.size };
  });
}

export function measurePagefilePowerShell() {
  const command = "$p=Get-CimInstance Win32_PageFileUsage -ErrorAction SilentlyContinue | Select-Object -First 1; if($p){[pscustomobject]@{alloc=$p.AllocatedBaseSize}|ConvertTo-Json -Compress}else{'{}'}";
  const result = spawnSync('powershell', ['-NoProfile', '-Command', command], { encoding: 'utf8', timeout: 30_000 });
  let value;
  try { value = JSON.parse(result.stdout || '{}'); } catch { fail('source', 'pagefile ilegível'); }
  if (!Number.isSafeInteger(value.alloc) || value.alloc <= 0) fail('source', 'pagefile indisponível');
  return value.alloc * MIB;
}

export function measureFree() {
  const stat = fs.statfsSync('C:\\');
  const bytes = stat.bavail * stat.bsize;
  if (!Number.isSafeInteger(bytes) || bytes < 0) fail('source', 'C: livre inválido');
  return bytes;
}

export function measureAll(baseline, deps = {}) {
  const pagefile = (deps.measurePagefile ?? measurePagefilePowerShell)();
  return {
    free: (deps.measureFree ?? measureFree)(),
    roots: (deps.measureRoots ?? measureRoots)(rootsFromBaseline(baseline)),
    vhd: (deps.measureVhd ?? measureVhd)(baseline.vhd),
    pagefile_alloc_bytes: pagefile,
  };
}

function newLedger(baseline, baselineBytes) {
  const roots = rootsFromBaseline(baseline);
  const vhd = Array.isArray(baseline.vhd) ? baseline.vhd : [];
  if (!baseline.authorization_id || !baseline.authorized_at) fail('baseline', 'identidade da autorização ausente');
  const rootState = Object.fromEntries(roots.map((root) => [root.name, { path: root.path, start_bytes: root.start_bytes, high_water_bytes: root.start_bytes, charged_bytes: 0 }]));
  const vhdState = Object.fromEntries(vhd.map((item) => [item.name, { path: path.resolve(expand(item.path)), start_bytes: item.start_bytes, high_water_bytes: item.start_bytes, charged_bytes: 0 }]));
  return {
    schema: SCHEMA,
    authorization_id: baseline.authorization_id,
    authorized_at: baseline.authorized_at,
    baseline_sha256: sha256(baselineBytes),
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    status: 'ready',
    roots: rootState,
    vhd: vhdState,
    pagefile: { start_bytes: baseline.pagefile.start_alloc_bytes, high_water_bytes: baseline.pagefile.start_alloc_bytes, charged_bytes: 0 },
    charged_bytes: 0,
    history: [],
  };
}

export function loadOrCreateLedger(baselinePath, ledgerPath) {
  const baselineBytes = fs.readFileSync(baselinePath);
  const baseline = JSON.parse(baselineBytes.toString('utf8'));
  const baselineHash = sha256(baselineBytes);
  if (!baseline.pagefile || !Number.isSafeInteger(baseline.pagefile.start_alloc_bytes)) fail('baseline', 'pagefile inicial ausente');
  if (baseline.old_ledger_path && path.resolve(expand(baseline.old_ledger_path)) === path.resolve(ledgerPath)) {
    fail('identity', 'o ledger antigo de 2 GiB não pode ser reutilizado como ledger novo');
  }
  if (!fs.existsSync(ledgerPath)) {
    const ledger = newLedger(baseline, baselineBytes);
    persistLedger(ledgerPath, ledger);
    return { baseline, ledger };
  }
  const ledger = readJson(ledgerPath);
  if (ledger.schema !== SCHEMA) fail('identity', 'schema do ledger novo difere');
  if (ledger.authorization_id !== baseline.authorization_id || ledger.authorized_at !== baseline.authorized_at) fail('identity', 'autorização difere');
  if (ledger.baseline_sha256 !== baselineHash) fail('identity', 'baseline difere da autorização');
  for (const root of rootsFromBaseline(baseline)) {
    if (ledger.roots?.[root.name]?.path !== root.path || ledger.roots[root.name].start_bytes !== root.start_bytes) fail('identity', `raiz ${root.name} difere`);
  }
  for (const item of baseline.vhd ?? []) {
    const file = path.resolve(expand(item.path));
    if (ledger.vhd?.[item.name]?.path !== file || ledger.vhd[item.name].start_bytes !== item.start_bytes) fail('identity', `VHD ${item.name} difere`);
  }
  if (ledger.pagefile?.start_bytes !== baseline.pagefile.start_alloc_bytes) fail('identity', 'pagefile inicial difere');
  return { baseline, ledger };
}

export function accumulate(ledger, sample) {
  let delta = 0;
  for (const root of sample.roots) {
    const state = ledger.roots[root.name];
    if (!state) fail('identity', `raiz nova: ${root.name}`);
    if (root.bytes > state.high_water_bytes) {
      delta += root.bytes - state.high_water_bytes;
      state.charged_bytes += root.bytes - state.high_water_bytes;
      state.high_water_bytes = root.bytes;
    }
  }
  for (const item of sample.vhd) {
    const state = ledger.vhd[item.name];
    if (!state) fail('identity', `VHD novo: ${item.name}`);
    if (item.bytes > state.high_water_bytes) {
      delta += item.bytes - state.high_water_bytes;
      state.charged_bytes += item.bytes - state.high_water_bytes;
      state.high_water_bytes = item.bytes;
    }
  }
  if (sample.pagefile_alloc_bytes > ledger.pagefile.high_water_bytes) {
    delta += sample.pagefile_alloc_bytes - ledger.pagefile.high_water_bytes;
    ledger.pagefile.charged_bytes += sample.pagefile_alloc_bytes - ledger.pagefile.high_water_bytes;
    ledger.pagefile.high_water_bytes = sample.pagefile_alloc_bytes;
  }
  ledger.charged_bytes += delta;
  ledger.updated_at = new Date().toISOString();
  return delta;
}

export function checkLimits(ledger, freeBytes) {
  if (ledger.charged_bytes + RESERVE_BYTES > CAP_BYTES) return 'cap';
  if (freeBytes < FLOOR_BYTES + RESERVE_BYTES) return 'floor';
  return null;
}

export function persistLedger(file, ledger) {
  const tmp = `${file}.tmp-${process.pid}-${crypto.randomBytes(4).toString('hex')}`;
  fs.writeFileSync(tmp, `${JSON.stringify(ledger, null, 2)}\n`, 'utf8');
  try {
    fs.renameSync(tmp, file);
  } catch (error) {
    try {
      fs.copyFileSync(tmp, file);
      fs.unlinkSync(tmp);
    } catch {
      throw error;
    }
  }
}

export function cancelOwnChild(child) {
  if (!child || typeof child.kill !== 'function') return false;
  child.kill();
  return true;
}

function usage() {
  return `Uso: node scripts/m1-budget-supervisor.mjs --baseline <json> --ledger <json> [--check] [--interval-seconds 5] [--timeout-seconds 3600] -- <comando> [args...]

Mede roots/VHD/pagefile e conta apenas aumentos positivos (high-water).
Teto: 4 GiB - 256 MiB de reserva. Piso: C: 10 GiB + 256 MiB.
O ledger antigo de 2 GiB não é lido nem alterado.`;
}

function parseArgs(argv) {
  const parsed = { baseline: null, ledger: null, check: false, interval: 5, timeout: 3600, command: null, args: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--help' || arg === '-h') { parsed.help = true; continue; }
    if (arg === '--check') { parsed.check = true; continue; }
    if (arg === '--') { parsed.command = argv[i + 1] ?? null; parsed.args = argv.slice(i + 2); break; }
    const value = argv[i + 1];
    if (value === undefined) fail('config', `${arg} exige valor`);
    if (arg === '--baseline') parsed.baseline = path.resolve(value);
    else if (arg === '--ledger') parsed.ledger = path.resolve(value);
    else if (arg === '--interval-seconds') parsed.interval = Number(value);
    else if (arg === '--timeout-seconds') parsed.timeout = Number(value);
    else fail('config', `argumento desconhecido: ${arg}`);
    i += 1;
  }
  if (!parsed.help && (!parsed.baseline || !parsed.ledger)) fail('config', '--baseline e --ledger são obrigatórios');
  if (!Number.isInteger(parsed.interval) || parsed.interval < 1) fail('config', 'interval inválido');
  if (!Number.isInteger(parsed.timeout) || parsed.timeout < 1) fail('config', 'timeout inválido');
  if (!parsed.help && !parsed.check && !parsed.command) fail('config', 'informe o comando após --');
  return parsed;
}

function persistSample(ledgerPath, ledger, sample, status = 'running', lastError = null) {
  accumulate(ledger, sample);
  ledger.status = status;
  ledger.updated_at = new Date().toISOString();
  ledger.history.push({
    at: ledger.updated_at,
    free_bytes: sample.free,
    charged_bytes: ledger.charged_bytes,
    status,
    ...(lastError ? { error: lastError } : {}),
  });
  if (ledger.history.length > 200) ledger.history = ledger.history.slice(-200);
  persistLedger(ledgerPath, ledger);
}

export async function runSupervisor(argv, deps = {}) {
  const parsed = parseArgs(argv);
  if (parsed.help) return { code: 0, stdout: `${usage()}\n` };
  const { baseline, ledger } = loadOrCreateLedger(parsed.baseline, parsed.ledger);
  const sample = (deps.measureAll ?? ((b) => measureAll(b, deps)))(baseline);
  const initialLimit = checkLimits(ledger, sample.free);
  if (initialLimit) {
    persistSample(parsed.ledger, ledger, sample, 'blocked', initialLimit);
    return { code: 3, stderr: `bloqueado: ${initialLimit}\n` };
  }
  if (parsed.check) {
    persistSample(parsed.ledger, ledger, sample, 'ok');
    return { code: 0, stdout: `${JSON.stringify({ status: 'ok', charged_bytes: ledger.charged_bytes, free_bytes: sample.free })}\n` };
  }

  const child = (deps.spawn ?? spawn)(parsed.command, parsed.args, { stdio: 'inherit', windowsHide: true });
  let stopped = false;
  let timer = null;
  let interval = null;
  let finalCode = 0;
  let failure = null;
  const stop = (reason) => {
    if (stopped) return;
    stopped = true;
    finalCode = 3;
    failure = reason;
    try { persistSample(parsed.ledger, ledger, (deps.measureAll ?? ((b) => measureAll(b, deps)))(baseline), 'blocked', reason); } catch {}
    cancelOwnChild(child);
  };
  await new Promise((resolve) => {
    interval = setInterval(() => {
      try {
        const current = (deps.measureAll ?? ((b) => measureAll(b, deps)))(baseline);
        const limit = checkLimits(ledger, current.free);
        if (limit) { persistSample(parsed.ledger, ledger, current, 'blocked', limit); stop(limit); return; }
        persistSample(parsed.ledger, ledger, current, 'running');
      } catch (error) {
        stop(error.message || 'measure');
      }
    }, parsed.interval * 1000);
    timer = setTimeout(() => stop('timeout'), parsed.timeout * 1000);
    child.once('error', (error) => { failure = error.message; finalCode = 4; resolve(); });
    child.once('exit', (code, signal) => {
      if (!stopped) finalCode = signal ? 4 : (code ?? 4);
      resolve();
    });
  });
  clearInterval(interval);
  clearTimeout(timer);
  try {
    const final = (deps.measureAll ?? ((b) => measureAll(b, deps)))(baseline);
    const finalLimit = checkLimits(ledger, final.free);
    persistSample(parsed.ledger, ledger, final, finalLimit || stopped ? 'blocked' : (finalCode === 0 ? 'ok' : 'command-failed'), finalLimit || failure);
    if (finalLimit && !stopped) finalCode = 3;
  } catch (error) {
    ledger.status = 'error';
    ledger.updated_at = new Date().toISOString();
    persistLedger(parsed.ledger, ledger);
    finalCode = 4;
    failure = error.message;
  }
  return { code: finalCode, stderr: failure ? `supervisor: ${failure}\n` : '' };
}

if (process.argv[1] && path.resolve(process.argv[1]) === path.resolve(fileURLToPath(import.meta.url))) {
  try {
    const result = await runSupervisor(process.argv.slice(2));
    if (result.stdout) process.stdout.write(result.stdout);
    if (result.stderr) process.stderr.write(result.stderr);
    process.exitCode = result.code;
  } catch (error) {
    process.stderr.write(`m1-budget-supervisor: ${error.code ?? 'error'}: ${error.message}\n`);
    process.exitCode = 4;
  }
}
