#!/usr/bin/env node
/**
 * Supervisor operacional da autorização M1 de 4 GiB.
 *
 * Não substitui o run-with-disk-budget/ledger antigo e nunca o altera.
 * Este ledger é separado, inicia na medição da autorização registrada em
 * AUTORIZACAO-ORCAMENTO-M1-20261103.md e cobra apenas o crescimento positivo
 * desde a última amostra, sem crédito por reduções; high-water é metadado.
 */
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import process from 'node:process';
import { execFile, spawn, spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { Worker, isMainThread } from 'node:worker_threads';
import { measureTree } from './run-with-disk-budget.mjs';

export const MIB = 1024 * 1024;
export const GIB = 1024 * MIB;
export const CAP_BYTES = 4 * GIB;
export const RESERVE_BYTES = 256 * MIB;
export const FLOOR_BYTES = 10 * GIB;
export const SCHEMA = 'm1-budget-supervisor/v2';
export const LEGACY_SCHEMA = 'm1-budget-supervisor/v1';

const fail = (code, message) => {
  const error = new Error(message);
  error.code = code;
  throw error;
};

const hasOwn = (value, key) => Object.prototype.hasOwnProperty.call(value, key);

/** Cap explícito: inteiro seguro, com headroom para a reserva e sem ampliar o teto legado. */
function validateCapBytes(value, code = 'config') {
  if (!Number.isSafeInteger(value) || value <= RESERVE_BYTES || value > CAP_BYTES) {
    fail(code, 'cap deve ser inteiro seguro, maior que a reserva e no máximo 4 GiB');
  }
  return value;
}

function parseCapBytes(value) {
  if (typeof value !== 'string' || !/^[0-9]+$/.test(value)) {
    fail('config', '--cap-bytes deve ser inteiro decimal positivo');
  }
  return validateCapBytes(Number(value), 'config');
}

const expand = (value) => String(value).replace(/%([^%]+)%/g, (_, name) => process.env[name] ?? `%${name}%`);
const sha256 = (data) => crypto.createHash('sha256').update(data).digest('hex');
const readJson = (file) => JSON.parse(fs.readFileSync(file, 'utf8'));

function inside(child, parent) {
  const rel = path.relative(path.resolve(parent), path.resolve(child));
  return rel !== '' && !rel.startsWith('..') && !path.isAbsolute(rel);
}

export function rootsFromBaseline(baseline) {
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

const PAGEFILE_COMMAND = "$p=Get-CimInstance Win32_PageFileUsage -ErrorAction SilentlyContinue | Select-Object -First 1; if($p){[pscustomobject]@{alloc=$p.AllocatedBaseSize}|ConvertTo-Json -Compress}else{'{}'}";

export function measurePagefilePowerShellAsync(timeoutMs = 2500) {
  return new Promise((resolve, reject) => {
    execFile('powershell', ['-NoProfile', '-Command', PAGEFILE_COMMAND], { encoding: 'utf8', timeout: timeoutMs, windowsHide: true }, (error, stdout) => {
      if (error) {
        const failure = new Error('pagefile: medição não concluída');
        failure.code = error.killed || error.signal ? 'measure-timeout' : 'source';
        reject(failure);
        return;
      }
      let value;
      try { value = JSON.parse(stdout || '{}'); } catch { value = null; }
      if (!value || !Number.isSafeInteger(value.alloc) || value.alloc <= 0) {
        const failure = new Error('pagefile indisponível');
        failure.code = 'source';
        reject(failure);
        return;
      }
      resolve(value.alloc * MIB);
    });
  });
}

export function measureFastDefault(deps = {}) {
  return Promise.resolve((deps.measureFree ?? measureFree)()).then(async free => ({
    free,
    pagefile_alloc_bytes: await (deps.measurePagefileAsync ?? measurePagefilePowerShellAsync)(),
  }));
}

export function measureSlowDefault(baseline, deps = {}) {
  const timeoutMs = deps.measureTimeoutMs ?? 5000;
  let worker;
  let timer;
  const promise = new Promise((resolve, reject) => {
    const createWorker = deps.createWorker ?? ((url, options) => new Worker(url, options));
    worker = createWorker(new URL('./m1-budget-measure-worker.mjs', import.meta.url), { workerData: { baseline } });
    timer = (deps.setTimeout ?? setTimeout)(() => {
      const failure = new Error('varredura de árvores excedeu o tempo');
      failure.code = 'measure-timeout';
      worker.terminate();
      reject(failure);
    }, timeoutMs);
    worker.once('message', message => {
      clearTimeout(timer);
      worker.terminate();
      if (message.ok) resolve({ roots: message.roots, vhd: message.vhd });
      else {
        const failure = new Error(message.message);
        failure.code = message.code ?? 'source';
        reject(failure);
      }
    });
    worker.once('error', error => {
      clearTimeout(timer);
      const failure = new Error(error.message || 'worker de medição falhou');
      failure.code = 'source';
      reject(failure);
    });
  });
  promise.cancel = () => worker?.terminate();
  return promise;
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

function newLedger(baseline, baselineBytes, capBytes) {
  const roots = rootsFromBaseline(baseline);
  const vhd = Array.isArray(baseline.vhd) ? baseline.vhd : [];
  if (!baseline.authorization_id || !baseline.authorized_at) fail('baseline', 'identidade da autorização ausente');
  const rootState = Object.fromEntries(roots.map((root) => [root.name, { path: root.path, start_bytes: root.start_bytes, last_observed_bytes: root.start_bytes, high_water_bytes: root.start_bytes, charged_bytes: 0 }]));
  const vhdState = Object.fromEntries(vhd.map((item) => [item.name, { path: path.resolve(expand(item.path)), start_bytes: item.start_bytes, last_observed_bytes: item.start_bytes, high_water_bytes: item.start_bytes, charged_bytes: 0 }]));
  return {
    schema: SCHEMA,
    authorization_id: baseline.authorization_id,
    authorized_at: baseline.authorized_at,
    cap_bytes: capBytes,
    baseline_sha256: sha256(baselineBytes),
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    status: 'ready',
    roots: rootState,
    vhd: vhdState,
    pagefile: { start_bytes: baseline.pagefile.start_alloc_bytes, last_observed_bytes: baseline.pagefile.start_alloc_bytes, high_water_bytes: baseline.pagefile.start_alloc_bytes, charged_bytes: 0 },
    charged_bytes: 0,
    history: [],
  };
}

function migrateLedgerV1(ledger) {
  const migrated = JSON.parse(JSON.stringify(ledger));
  migrated.schema = SCHEMA;
  migrated.migrated_from = LEGACY_SCHEMA;
  migrated.migration = {
    at: new Date().toISOString(),
    note: 'last_observed_bytes inicializado do high_water do ledger v1; charged preservado; primeira amostra atualiza o último sem crédito.',
  };
  for (const group of [migrated.roots, migrated.vhd]) {
    for (const state of Object.values(group ?? {})) {
      if (!Number.isSafeInteger(state.last_observed_bytes)) {
        state.last_observed_bytes = state.high_water_bytes ?? state.start_bytes;
      }
    }
  }
  if (migrated.pagefile && !Number.isSafeInteger(migrated.pagefile.last_observed_bytes)) {
    migrated.pagefile.last_observed_bytes = migrated.pagefile.high_water_bytes ?? migrated.pagefile.start_bytes;
  }
  return migrated;
}

export function loadOrCreateLedger(baselinePath, ledgerPath, requestedCapBytes) {
  const baselineBytes = fs.readFileSync(baselinePath);
  const baseline = JSON.parse(baselineBytes.toString('utf8'));
  const baselineHash = sha256(baselineBytes);
  if (path.resolve(baselinePath) === path.resolve(ledgerPath)) fail('identity', 'baseline e ledger devem usar caminhos distintos');
  if (!baseline.pagefile || !Number.isSafeInteger(baseline.pagefile.start_alloc_bytes)) fail('baseline', 'pagefile inicial ausente');
  const oldLedger = baseline.old_ledger_path ? path.resolve(expand(baseline.old_ledger_path)) : null;
  if (oldLedger && oldLedger === path.resolve(ledgerPath)) {
    fail('identity', 'o ledger antigo de 2 GiB não pode ser reutilizado como ledger novo');
  }
  if (oldLedger && oldLedger === path.resolve(baselinePath)) fail('identity', 'baseline não pode ser o ledger antigo');
  const baselineHasCap = hasOwn(baseline, 'cap_bytes');
  if (hasOwn(baseline, 'task_id') && !baselineHasCap) fail('baseline', 'baseline com task_id exige cap_bytes');
  const baselineCap = baselineHasCap ? validateCapBytes(baseline.cap_bytes, 'baseline') : null;
  const requestedCap = requestedCapBytes === undefined ? null : validateCapBytes(requestedCapBytes, 'config');
  if (requestedCap !== null) {
    if (!baselineHasCap) fail('cap', '--cap-bytes exige cap_bytes na baseline');
    if (requestedCap !== baselineCap) fail('cap', 'cap do CLI difere da baseline');
  }
  const capBytes = baselineHasCap ? baselineCap : CAP_BYTES;
  if (!fs.existsSync(ledgerPath)) {
    const ledger = newLedger(baseline, baselineBytes, capBytes);
    persistLedger(ledgerPath, ledger);
    return { baseline, ledger, capBytes };
  }
  let ledger = readJson(ledgerPath);
  if (ledger.schema === LEGACY_SCHEMA) {
    ledger = migrateLedgerV1(ledger);
    persistLedger(ledgerPath, ledger);
  }
  if (ledger.schema !== SCHEMA) fail('identity', 'schema do ledger novo difere');
  if (hasOwn(ledger, 'cap_bytes')) {
    const ledgerCap = validateCapBytes(ledger.cap_bytes, 'ledger');
    if (ledgerCap !== capBytes) fail('cap', 'cap do ledger difere da baseline');
  } else if (capBytes !== CAP_BYTES) {
    fail('cap', 'ledger legado sem cap não pode ser reutilizado com cap explícito');
  }
  for (const group of [ledger.roots, ledger.vhd]) {
    for (const [name, state] of Object.entries(group ?? {})) {
      if (!Number.isSafeInteger(state.last_observed_bytes) || !Number.isSafeInteger(state.high_water_bytes)) fail('identity', `estado sem last/high-water: ${name}`);
    }
  }
  if (!ledger.pagefile || !Number.isSafeInteger(ledger.pagefile.last_observed_bytes)) fail('identity', 'pagefile sem último observado');
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
  return { baseline, ledger, capBytes };
}

function advanceSegment(state, bytes, name) {
  if (!state) fail('identity', `segmento novo: ${name}`);
  const last = Number.isSafeInteger(state.last_observed_bytes)
    ? state.last_observed_bytes
    : state.start_bytes;
  const delta = bytes - last;
  if (delta > 0) {
    state.charged_bytes += delta;
  }
  state.last_observed_bytes = bytes;
  if (!Number.isSafeInteger(state.high_water_bytes) || bytes > state.high_water_bytes) {
    state.high_water_bytes = bytes;
  }
  return delta > 0 ? delta : 0;
}

export function accumulate(ledger, sample) {
  let delta = 0;
  for (const root of sample.roots) {
    delta += advanceSegment(ledger.roots[root.name], root.bytes, `raiz ${root.name}`);
  }
  for (const item of sample.vhd) {
    delta += advanceSegment(ledger.vhd[item.name], item.bytes, `VHD ${item.name}`);
  }
  delta += advanceSegment(ledger.pagefile, sample.pagefile_alloc_bytes, 'pagefile');
  ledger.charged_bytes += delta;
  ledger.updated_at = new Date().toISOString();
  return delta;
}

export function accumulateFast(ledger, sample) {
  const delta = advanceSegment(ledger.pagefile, sample.pagefile_alloc_bytes, 'pagefile');
  ledger.charged_bytes += delta;
  ledger.updated_at = new Date().toISOString();
  return delta;
}

export function accumulateSlow(ledger, sample) {
  let delta = 0;
  for (const root of sample.roots) delta += advanceSegment(ledger.roots[root.name], root.bytes, `raiz ${root.name}`);
  for (const item of sample.vhd) delta += advanceSegment(ledger.vhd[item.name], item.bytes, `VHD ${item.name}`);
  ledger.charged_bytes += delta;
  ledger.updated_at = new Date().toISOString();
  return delta;
}

function rootsBytes(sample) {
  return sample.roots.reduce((sum, root) => sum + root.bytes, 0) + sample.vhd.reduce((sum, item) => sum + item.bytes, 0);
}

/** Observed positive peaks per segment; legacy history falls back to charged deltas. */
export function observedHeadroomBytes(ledger) {
  const history = Array.isArray(ledger.history) ? ledger.history : [];
  let maxPagefile = 0, maxRoots = 0, legacy = 0, hasSegments = false;
  for (let index = 1; index < history.length; index += 1) {
    const previous = history[index - 1], current = history[index];
    if ([previous.pagefile_bytes, current.pagefile_bytes, previous.roots_bytes, current.roots_bytes].every(Number.isSafeInteger)) {
      hasSegments = true;
      maxPagefile = Math.max(maxPagefile, current.pagefile_bytes - previous.pagefile_bytes);
      maxRoots = Math.max(maxRoots, current.roots_bytes - previous.roots_bytes);
    } else if (Number.isSafeInteger(previous.charged_bytes) && Number.isSafeInteger(current.charged_bytes)) {
      legacy = Math.max(legacy, current.charged_bytes - previous.charged_bytes);
    }
  }
  return hasSegments ? Math.max(0, maxPagefile) + Math.max(0, maxRoots) : Math.max(0, legacy);
}

export function startHeadroomIssue(ledger) {
  const cap = ledger.cap_bytes ?? CAP_BYTES;
  const useful = cap - ledger.charged_bytes - RESERVE_BYTES;
  const observed = observedHeadroomBytes(ledger);
  return useful < observed ? 'headroom' : null;
}

export function checkLimits(ledger, freeBytes, capBytes = hasOwn(ledger, 'cap_bytes') ? ledger.cap_bytes : CAP_BYTES) {
  const cap = validateCapBytes(capBytes, hasOwn(ledger, 'cap_bytes') ? 'ledger' : 'config');
  if (ledger.charged_bytes + RESERVE_BYTES > cap) return 'cap';
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

export function cancelOwnChild(child, deps = {}) {
  if (!child) return false;
  if (child.exitCode !== null && child.exitCode !== undefined) return false;
  if (child.signalCode !== null && child.signalCode !== undefined) return false;

  const platform = deps.platform ?? process.platform;
  const pid = child.pid;
  if (platform === 'win32' && Number.isInteger(pid) && pid > 0) {
    const taskkill = deps.taskkill ?? ((childPid) => spawnSync(
      'taskkill',
      ['/PID', String(childPid), '/T', '/F'],
      { windowsHide: true, timeout: 5000 },
    ));
    const result = taskkill(pid);
    if (result && result.status === 0) return true;
  }
  if (typeof child.kill === 'function') {
    child.kill();
    return true;
  }
  return false;
}

function usage() {
  return `Uso: node scripts/m1-budget-supervisor.mjs --baseline <json> --ledger <json> [--cap-bytes N] [--check] [--interval-seconds 5] [--timeout-seconds 3600] -- <comando> [args...]

Mede roots/VHD/pagefile e cobra apenas o crescimento positivo desde a última amostra; high-water é metadado.
Teto padrão: 4 GiB; cap explícito exige cap_bytes na baseline e no ledger e não pode ampliar 4 GiB.
Reserva: 256 MiB. Piso: C: 10 GiB + 256 MiB.
O ledger antigo de 2 GiB não é lido nem alterado.`;
}

function parseArgs(argv) {
  const parsed = { baseline: null, ledger: null, capBytes: null, check: false, interval: 5, timeout: 3600, command: null, args: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--help' || arg === '-h') { parsed.help = true; continue; }
    if (arg === '--check') { parsed.check = true; continue; }
    if (arg === '--') { parsed.command = argv[i + 1] ?? null; parsed.args = argv.slice(i + 2); break; }
    const value = argv[i + 1];
    if (value === undefined) fail('config', `${arg} exige valor`);
    if (arg === '--baseline') parsed.baseline = path.resolve(value);
    else if (arg === '--ledger') parsed.ledger = path.resolve(value);
    else if (arg === '--cap-bytes') parsed.capBytes = parseCapBytes(value);
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

export function evaluateSample(ledger, sample) {
  accumulate(ledger, sample);
  return checkLimits(ledger, sample.free);
}

function persistSample(ledgerPath, ledger, sample, status = 'running', lastError = null) {
  ledger.status = status;
  ledger.updated_at = new Date().toISOString();
  ledger.history.push({
    at: ledger.updated_at,
    free_bytes: sample.free,
    charged_bytes: ledger.charged_bytes,
    pagefile_bytes: sample.pagefile_alloc_bytes,
    roots_bytes: rootsBytes(sample),
    status,
    ...(lastError ? { error: lastError } : {}),
  });
  persistLedger(ledgerPath, ledger);
}

function persistMeasureError(ledgerPath, ledger, error) {
  ledger.status = 'error';
  ledger.updated_at = new Date().toISOString();
  ledger.history.push({ at: ledger.updated_at, charged_bytes: ledger.charged_bytes, status: 'error', error: error.message || 'measure' });
  persistLedger(ledgerPath, ledger);
}

async function runSupervisorAsync(parsed, deps = {}) {
  const { baseline, ledger } = loadOrCreateLedger(parsed.baseline, parsed.ledger, parsed.capBytes ?? undefined);
  const measureFast = deps.measureFast ?? (base => measureFastDefault(deps));
  const measureSlow = deps.measureSlow ?? (base => measureSlowDefault(base, deps));
  const combined = () => ({ ...lastFast, roots: lastSlow.roots, vhd: lastSlow.vhd });
  let lastFast;
  let lastSlow;
  try {
    lastFast = await measureFast(baseline);
  } catch (error) {
    persistMeasureError(parsed.ledger, ledger, error);
    return { code: 4, stderr: `supervisor: ${error.code ?? 'measure'}\n` };
  }
  accumulateFast(ledger, lastFast);
  const fastLimit = checkLimits(ledger, lastFast.free);
  if (fastLimit) {
    persistSample(parsed.ledger, ledger, { ...lastFast, roots: [], vhd: [] }, 'blocked', fastLimit);
    return { code: 3, stderr: `bloqueado: ${fastLimit}\n` };
  }
  try {
    lastSlow = await measureSlow(baseline);
  } catch (error) {
    persistMeasureError(parsed.ledger, ledger, error);
    return { code: 4, stderr: `supervisor: ${error.code ?? 'measure'}\n` };
  }
  accumulateSlow(ledger, lastSlow);
  const slowLimit = checkLimits(ledger, lastFast.free);
  if (slowLimit) {
    persistSample(parsed.ledger, ledger, combined(), 'blocked', slowLimit);
    return { code: 3, stderr: `bloqueado: ${slowLimit}\n` };
  }
  const headroom = startHeadroomIssue(ledger);
  if (headroom) {
    persistSample(parsed.ledger, ledger, combined(), 'blocked', headroom);
    return { code: 3, stderr: `bloqueado: ${headroom}\n` };
  }
  if (parsed.check) {
    persistSample(parsed.ledger, ledger, combined(), 'ok');
    return { code: 0, stdout: `${JSON.stringify({ status: 'ok', charged_bytes: ledger.charged_bytes, free_bytes: lastFast.free })}\n` };
  }

  const scheduleInterval = deps.setInterval ?? setInterval;
  const clearScheduledInterval = deps.clearInterval ?? clearInterval;
  const scheduleTimeout = deps.setTimeout ?? setTimeout;
  const clearScheduledTimeout = deps.clearTimeout ?? clearTimeout;
  const child = (deps.spawn ?? spawn)(parsed.command, parsed.args, { stdio: 'inherit', windowsHide: true });
  let stopped = false;
  let fastPending = false;
  let slowPending = null;
  let interval = null;
  let timer = null;
  let finalCode = 0;
  let failure = null;
  let finish;
  const stoppedPromise = new Promise(resolve => { finish = resolve; });
  const stop = (reason, alreadyRecorded = false) => {
    if (stopped) return;
    stopped = true;
    finalCode = 3;
    failure = reason;
    if (slowPending?.cancel) slowPending.cancel();
    if (!alreadyRecorded) {
      try { persistSample(parsed.ledger, ledger, combined(), 'blocked', reason); } catch {}
    }
    (deps.cancelOwnChild ?? cancelOwnChild)(child, deps);
  };
  const runFast = async () => {
    if (stopped || fastPending) return;
    fastPending = true;
    try {
      const fast = await measureFast(baseline);
      if (stopped) return;
      lastFast = fast;
      accumulateFast(ledger, fast);
      const limit = checkLimits(ledger, fast.free);
      if (limit) {
        persistSample(parsed.ledger, ledger, combined(), 'blocked', limit);
        stop(limit, true);
        return;
      }
      persistSample(parsed.ledger, ledger, combined(), 'running');
    } catch (error) {
      stop(error.code === 'measure-timeout' ? 'measure-timeout' : (error.message || 'measure'));
    } finally {
      fastPending = false;
    }
  };
  const runSlow = () => {
    if (stopped || slowPending) return;
    const pending = Promise.resolve(measureSlow(baseline));
    slowPending = pending;
    pending.then(slow => {
      if (stopped) return;
      lastSlow = slow;
      accumulateSlow(ledger, slow);
      const limit = checkLimits(ledger, lastFast.free);
      if (limit) {
        persistSample(parsed.ledger, ledger, combined(), 'blocked', limit);
        stop(limit, true);
        return;
      }
      persistSample(parsed.ledger, ledger, combined(), 'running');
    }).catch(error => {
      if (!stopped) stop(error.code === 'measure-timeout' ? 'measure-timeout' : (error.message || 'measure'));
    }).finally(() => { if (slowPending === pending) slowPending = null; });
  };
  interval = scheduleInterval(() => { void runFast(); runSlow(); }, parsed.interval * 1000);
  timer = scheduleTimeout(() => stop('timeout'), parsed.timeout * 1000);
  child.once('error', error => { failure = error.message; finalCode = 4; finish(); });
  child.once('exit', (code, signal) => {
    if (!stopped) finalCode = signal ? 4 : (code ?? 4);
    finish();
  });
  await stoppedPromise;
  clearScheduledInterval(interval);
  clearScheduledTimeout(timer);
  if (slowPending?.cancel) slowPending.cancel();
  try {
    const finalLimit = checkLimits(ledger, lastFast.free);
    persistSample(parsed.ledger, ledger, combined(), finalLimit || stopped ? 'blocked' : (finalCode === 0 ? 'ok' : 'command-failed'), finalLimit || failure);
    if (finalLimit && !stopped) finalCode = 3;
  } catch (error) {
    persistMeasureError(parsed.ledger, ledger, error);
    finalCode = 4;
  }
  return { code: finalCode, stderr: failure ? `supervisor: ${failure}\n` : '' };
}

export async function runSupervisor(argv, deps = {}) {
  const parsed = parseArgs(argv);
  if (parsed.help) return { code: 0, stdout: `${usage()}\n` };
  if (deps.measureAll && !deps.measureFast && !deps.measureSlow) return runSupervisorLegacy(argv, deps);
  return runSupervisorAsync(parsed, deps);
}

async function runSupervisorLegacy(argv, deps = {}) {
  const parsed = parseArgs(argv);
  if (parsed.help) return { code: 0, stdout: `${usage()}\n` };
  const { baseline, ledger } = loadOrCreateLedger(parsed.baseline, parsed.ledger, parsed.capBytes ?? undefined);
  const sample = (deps.measureAll ?? ((b) => measureAll(b, deps)))(baseline);
  const initialLimit = evaluateSample(ledger, sample);
  if (initialLimit) {
    persistSample(parsed.ledger, ledger, sample, 'blocked', initialLimit);
    return { code: 3, stderr: `bloqueado: ${initialLimit}\n` };
  }
  if (parsed.check) {
    persistSample(parsed.ledger, ledger, sample, 'ok');
    return { code: 0, stdout: `${JSON.stringify({ status: 'ok', charged_bytes: ledger.charged_bytes, free_bytes: sample.free })}\n` };
  }

  const scheduleInterval = deps.setInterval ?? setInterval;
  const clearScheduledInterval = deps.clearInterval ?? clearInterval;
  const scheduleTimeout = deps.setTimeout ?? setTimeout;
  const clearScheduledTimeout = deps.clearTimeout ?? clearTimeout;
  const child = (deps.spawn ?? spawn)(parsed.command, parsed.args, { stdio: 'inherit', windowsHide: true });
  let stopped = false;
  let timer = null;
  let interval = null;
  let finalCode = 0;
  let failure = null;
  const stop = (reason, alreadyRecorded = false) => {
    if (stopped) return;
    stopped = true;
    finalCode = 3;
    failure = reason;
    if (!alreadyRecorded) {
      try {
        const current = (deps.measureAll ?? ((b) => measureAll(b, deps)))(baseline);
        const limit = evaluateSample(ledger, current);
        persistSample(parsed.ledger, ledger, current, 'blocked', limit ?? reason);
      } catch {}
    }
    (deps.cancelOwnChild ?? cancelOwnChild)(child, deps);
  };
  await new Promise((resolve) => {
    interval = scheduleInterval(() => {
      try {
        const current = (deps.measureAll ?? ((b) => measureAll(b, deps)))(baseline);
        const limit = evaluateSample(ledger, current);
        if (limit) {
          // A amostra que cruzou o cap/piso já foi acumulada e é persistida
          // como blocked nesta mesma passagem; não espera a próxima amostra.
          persistSample(parsed.ledger, ledger, current, 'blocked', limit);
          stop(limit, true);
          return;
        }
        persistSample(parsed.ledger, ledger, current, 'running');
      } catch (error) {
        stop(error.message || 'measure');
      }
    }, parsed.interval * 1000);
    timer = scheduleTimeout(() => stop('timeout'), parsed.timeout * 1000);
    child.once('error', (error) => { failure = error.message; finalCode = 4; resolve(); });
    child.once('exit', (code, signal) => {
      if (!stopped) finalCode = signal ? 4 : (code ?? 4);
      resolve();
    });
  });
  clearScheduledInterval(interval);
  clearScheduledTimeout(timer);
  try {
    const final = (deps.measureAll ?? ((b) => measureAll(b, deps)))(baseline);
    const finalLimit = evaluateSample(ledger, final);
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

if (isMainThread && process.argv[1] && path.resolve(process.argv[1]) === path.resolve(fileURLToPath(import.meta.url))) {
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
