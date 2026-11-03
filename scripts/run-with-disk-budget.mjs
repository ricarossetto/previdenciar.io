#!/usr/bin/env node
/**
 * Wrapper serial de orçamento de disco.
 *
 * Ele NÃO é uma cota de filesystem. Faz pré-checagem, executa UM comando sem
 * shell e amostra o crescimento das raízes declaradas durante a execução. A
 * parada é fail-closed: erro de medição, ledger, lock ou persistência encerra
 * o próprio filho e nunca devolve sucesso sem a prova final.
 *
 * Uso:
 *   node scripts/run-with-disk-budget.mjs --ledger <json> --baseline <json> \
 *     --root target=<dir> --root repo=<dir> --root saga=<dir> \
 *     [--root temp=<dir>] [--root pg=<dir>] [--cache cargo=<dir>] \
 *     [--volume <dir>] [--cwd <dir>] [--timeout-seconds N] \
 *     [--init-ledger] -- <comando> [args...]
 *
 * O primeiro uso exige --init-ledger. O ledger fica fora do Git, sob a saga,
 * e nunca é reiniciado por medição nova. O baseline antigo não é alterado.
 */
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { execFileSync, spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { BudgetError, freeBytesForPath, parseBaselineObject } from './disk-budget.mjs';

const GIB = 1024 ** 3;
const MIB = 1024 ** 2;
const SCHEMA = 'previdenciar-run-disk-budget/v1';
const AUTHORIZED_CAP_BYTES = 2 * GIB;
const AUTHORIZED_FLOOR_BYTES = 10 * GIB;
const AUTHORIZED_RESERVE_BYTES = 256 * MIB;
const MAX_ENTRIES = 2_000_000;
const MAX_HISTORY = 50;
const MAX_DECREASES = 50;
const ROOT_NAMES = new Set(['target', 'repo', 'saga', 'temp', 'pg']);
const CACHE_NAME = /^[a-z][a-z0-9_-]{0,31}$/;
const DEFAULT_ROOT_SAMPLE_MS = 5_000;
const DEFAULT_FREE_SAMPLE_MS = 1_000;
const DEFAULT_STOP_GRACE_MS = 3_000;

export const POLICY = Object.freeze({
  totalCapBytes: AUTHORIZED_CAP_BYTES,
  floorBytes: AUTHORIZED_FLOOR_BYTES,
  reserveBytes: AUTHORIZED_RESERVE_BYTES,
  maxEntries: MAX_ENTRIES,
});

const USAGE = `Orçamento serial com parada fail-closed

Uso:
  node scripts/run-with-disk-budget.mjs --ledger <json> --baseline <json> \\
    --root target=<dir> --root repo=<dir> --root saga=<dir> \\
    [--root temp=<dir>] [--root pg=<dir>] [--cache cargo=<dir>] \\
    [--volume <dir>] [--cwd <dir>] [--timeout-seconds N] \\
    [--init-ledger] -- <comando> [args...]

Política fixa: 2 GiB de crescimento total desde a retomada, piso de 10 GiB
livres e reserva de 256 MiB para interrupção/relatórios. Não há flag para
afrouxar. O wrapper não usa shell, não lê conteúdo de arquivos e não mata
processos que não sejam o próprio filho.

Primeiro uso: --init-ledger. O ledger não pode ser sobrescrito, trocado de
caminho, apagado ou recriado por medição nova. O baseline antigo permanece
como está; a diminuição externa do target é registrada, não creditada.

Códigos: 0 comando ok; código do filho se ele falhar; 3 bloqueio de orçamento;
4 erro de configuração/medição/ledger; 130 cancelamento.`;

export class RunBudgetError extends Error {
  constructor(code, message) {
    super(message);
    this.name = 'RunBudgetError';
    this.code = code;
  }
}

function fail(code, message) {
  throw new RunBudgetError(code, message);
}

function nowIso(now = Date.now()) {
  return new Date(now).toISOString();
}

function normalize(value) {
  const resolved = path.resolve(String(value));
  return process.platform === 'win32' ? resolved.toLowerCase() : resolved;
}

function samePath(left, right) {
  return normalize(left) === normalize(right);
}

function isInside(child, parent) {
  const relative = path.relative(path.resolve(parent), path.resolve(child));
  return Boolean(relative) && !relative.startsWith('..') && !path.isAbsolute(relative);
}

function isSafeInteger(value) {
  return Number.isSafeInteger(value) && value >= 0;
}

function parsePositiveInteger(value, label) {
  const number = Number(value);
  if (!Number.isSafeInteger(number) || number <= 0) fail('config', `${label} inválido`);
  return number;
}

function sha256File(filePath) {
  try {
    return crypto.createHash('sha256').update(fs.readFileSync(filePath)).digest('hex');
  } catch {
    fail('config', 'não foi possível ler o baseline');
  }
}

function readJsonFile(filePath, label) {
  let text;
  try {
    text = fs.readFileSync(filePath, 'utf8');
  } catch {
    fail('config', `${label} não pôde ser lido`);
  }
  try {
    return JSON.parse(text);
  } catch {
    fail('config', `${label} não é JSON válido`);
  }
}

function parseRootSpec(value, label) {
  const separator = String(value).indexOf('=');
  if (separator <= 0 || separator === String(value).length - 1) {
    fail('config', `${label} deve ser nome=path`);
  }
  const name = String(value).slice(0, separator).trim().toLowerCase();
  const raw = String(value).slice(separator + 1).trim();
  if (!/^[a-z][a-z0-9_-]{0,31}$/.test(name) || !raw) {
    fail('config', `${label} inválido`);
  }
  return { name, path: path.resolve(raw) };
}

export function parseArgs(argv) {
  const parsed = {
    help: false,
    initLedger: false,
    ledger: null,
    baseline: null,
    roots: [],
    caches: [],
    volumes: [],
    cwd: process.cwd(),
    timeoutSeconds: 0,
    command: null,
    args: [],
  };
  let index = 0;
  while (index < argv.length) {
    const arg = argv[index];
    if (arg === '--') {
      parsed.command = argv[index + 1] ?? null;
      parsed.args = argv.slice(index + 2);
      index = argv.length;
      break;
    }
    if (arg === '--help' || arg === '-h') {
      parsed.help = true;
      index += 1;
      continue;
    }
    if (arg === '--init-ledger') {
      parsed.initLedger = true;
      index += 1;
      continue;
    }
    const value = argv[index + 1];
    if (value === undefined) fail('config', `${arg} exige um valor`);
    if (arg === '--ledger') parsed.ledger = path.resolve(value);
    else if (arg === '--baseline') parsed.baseline = path.resolve(value);
    else if (arg === '--root') parsed.roots.push(parseRootSpec(value, '--root'));
    else if (arg === '--cache') parsed.caches.push(parseRootSpec(value, '--cache'));
    else if (arg === '--volume') parsed.volumes.push(path.resolve(value));
    else if (arg === '--cwd') parsed.cwd = path.resolve(value);
    else if (arg === '--timeout-seconds') parsed.timeoutSeconds = parsePositiveInteger(value, '--timeout-seconds');
    else fail('config', 'argumento desconhecido');
    index += 2;
  }
  if (parsed.help) return parsed;
  if (!parsed.ledger) fail('config', '--ledger é obrigatório');
  if (!parsed.baseline) fail('config', '--baseline é obrigatório');
  if (!parsed.command) fail('config', 'informe o comando depois de --');
  if (!parsed.roots.some((root) => root.name === 'target')) fail('config', '--root target é obrigatório');
  if (!parsed.roots.some((root) => root.name === 'repo')) fail('config', '--root repo é obrigatório');
  if (!parsed.roots.some((root) => root.name === 'saga')) fail('config', '--root saga é obrigatório');
  const names = new Set();
  for (const root of parsed.roots) {
    if (!ROOT_NAMES.has(root.name)) fail('config', 'raiz não permitida');
    if (names.has(root.name)) fail('config', 'raiz duplicada');
    names.add(root.name);
  }
  for (const cache of parsed.caches) {
    if (!CACHE_NAME.test(cache.name)) fail('config', 'nome de cache inválido');
    if (names.has(cache.name)) fail('config', 'cache/raiz duplicados');
    names.add(cache.name);
  }
  return parsed;
}

export function validatePolicy(policy) {
  if (!policy || typeof policy !== 'object') fail('config', 'política inválida');
  if (!isSafeInteger(policy.totalCapBytes) || policy.totalCapBytes <= 0) fail('config', 'cap inválido');
  if (!isSafeInteger(policy.floorBytes) || policy.floorBytes <= 0) fail('config', 'piso inválido');
  if (!isSafeInteger(policy.reserveBytes) || policy.reserveBytes <= 0) fail('config', 'reserva inválida');
  if (policy.reserveBytes >= policy.totalCapBytes) fail('config', 'reserva consome o cap');
  if (!isSafeInteger(policy.maxEntries) || policy.maxEntries <= 0) fail('config', 'limite de entradas inválido');
  return policy;
}

function assertNoSymlink(value) {
  if (value.isSymbolicLink()) fail('measurement', 'symlink/reparsepoint recusado');
}

/**
 * Mede metadados de uma raiz, sem ler conteúdo. Raízes aninhadas podem ser
 * excluídas para não contar duas vezes o mesmo arquivo. Links são recusados.
 */
export function measureTree(root, { excludes = [], maxEntries = MAX_ENTRIES, io = fs } = {}) {
  const absolute = path.resolve(root);
  let rootStat;
  try {
    rootStat = io.lstatSync(absolute);
  } catch (error) {
    if (error.code === 'ENOENT') return { bytes: 0, entries: 0, root: absolute, exists: false };
    fail('measurement', 'raiz não pôde ser medida');
  }
  assertNoSymlink(rootStat);
  if (!rootStat.isDirectory()) fail('measurement', 'raiz não é diretório');
  let realRoot;
  try {
    realRoot = io.realpathSync.native(absolute);
  } catch {
    fail('measurement', 'raiz não pôde ser resolvida');
  }
  if (normalize(realRoot) !== normalize(absolute)) fail('measurement', 'raiz resolve por link');
  const excluded = new Set(excludes.map((value) => normalize(value)));
  let bytes = 0;
  let entries = 0;
  const pending = [absolute];
  while (pending.length) {
    const directory = pending.pop();
    let children;
    try {
      children = io.readdirSync(directory, { withFileTypes: true });
    } catch {
      fail('measurement', 'falha ao ler uma raiz');
    }
    for (const child of children) {
      entries += 1;
      if (entries > maxEntries) fail('measurement', 'limite de entradas excedido');
      const full = path.join(directory, child.name);
      if (excluded.has(normalize(full))) continue;
      let stat;
      try {
        stat = io.lstatSync(full);
      } catch {
        fail('measurement', 'falha ao medir uma entrada');
      }
      assertNoSymlink(stat);
      if (stat.isDirectory()) {
        pending.push(full);
      } else if (stat.isFile()) {
        if (!isSafeInteger(stat.size)) fail('measurement', 'tamanho inválido');
        bytes += stat.size;
        if (!Number.isSafeInteger(bytes)) fail('measurement', 'soma de bytes excedeu o inteiro seguro');
      } else {
        fail('measurement', 'entrada não regular');
      }
    }
  }
  return { bytes, entries, root: absolute, exists: true };
}

function nestedExcludes(root, allPaths) {
  return allPaths.filter((candidate) => !samePath(candidate, root) && isInside(candidate, root));
}

function measureConfig(config, { policy = POLICY, measure = measureTree } = {}) {
  const segments = [
    ...config.roots.map((root) => ({ ...root, kind: 'root' })),
    ...config.caches.map((cache) => ({ ...cache, kind: 'cache' })),
  ];
  const allPaths = segments.map((segment) => segment.path);
  const measured = new Map();
  for (const segment of segments) {
    const optional = segment.kind === 'root' && segment.name === 'temp';
    const result = measure(segment.path, {
      excludes: nestedExcludes(segment.path, allPaths),
      maxEntries: policy.maxEntries,
    });
    if (!result.exists && !optional) fail('config', 'raiz declarada não existe');
    measured.set(segment.name, { ...result, kind: segment.kind, path: segment.path });
  }
  return measured;
}

function readBaseline(input, targetPath) {
  const filePath = path.resolve(input);
  const raw = readJsonFile(filePath, 'baseline');
  let parsed;
  try {
    parsed = parseBaselineObject(raw, { targetPath });
  } catch (error) {
    if (error instanceof BudgetError) fail('config', 'baseline inválido');
    throw error;
  }
  const hash = sha256File(filePath);
  if (parsed.floorGb < 10) fail('config', 'baseline enfraquece o piso');
  if (parsed.growthCapGb > 2) fail('config', 'baseline enfraquece o teto');
  const uncertaintyBytes = parsed.approx ? Math.round(parsed.uncertaintyGb * GIB) : 0;
  const effectiveBytes = Math.max(0, Math.round(parsed.baselineBytes) - uncertaintyBytes);
  return {
    path: filePath,
    hash,
    bytes: Math.round(parsed.baselineBytes),
    approx: parsed.approx,
    uncertaintyBytes,
    effectiveBytes,
  };
}

function buildSegments(config, measured) {
  const configs = [
    ...config.roots.map((root) => ({ ...root, kind: 'root' })),
    ...config.caches.map((cache) => ({ ...cache, kind: 'cache' })),
  ];
  return configs.map((item) => {
    const value = measured.get(item.name);
    return {
      name: item.name,
      kind: item.kind,
      path: item.path,
      start_bytes: value.bytes,
      high_water_bytes: value.bytes,
      charged_bytes: 0,
      last_observed_bytes: value.bytes,
    };
  });
}

function uniqueExternalDecrease(baseline, targetBytes) {
  const decrease = Math.max(0, baseline.effectiveBytes - targetBytes);
  return decrease;
}

export function buildLedger({ config, baseline, measured, policy = POLICY, now = Date.now() }) {
  validatePolicy(policy);
  const segments = buildSegments(config, measured);
  const target = segments.find((segment) => segment.name === 'target');
  if (!target) fail('config', 'segmento target ausente');
  const external = uniqueExternalDecrease(baseline, target.start_bytes);
  const created = nowIso(now);
  return {
    schema: SCHEMA,
    created_at: created,
    updated_at: created,
    resumption_id: `2026-11-03:${crypto.randomUUID()}`,
    ledger_path: config.ledger,
    repo_root: config.repoRoot,
    target_root: config.targetRoot,
    baseline_path: baseline.path,
    baseline_sha256: baseline.hash,
    baseline_bytes: baseline.bytes,
    baseline_approx: baseline.approx,
    baseline_uncertainty_bytes: baseline.uncertaintyBytes,
    external_decrease_bytes: external,
    external_decrease_approx: baseline.approx,
    external_decrease_at: created,
    external_decreases: external > 0 ? [{
      at: created,
      kind: 'initial-baseline-above-target',
      before_bytes: baseline.effectiveBytes,
      after_bytes: target.start_bytes,
      bytes: external,
    }] : [],
    total_cap_bytes: policy.totalCapBytes,
    floor_bytes: policy.floorBytes,
    reserve_bytes: policy.reserveBytes,
    used_bytes: 0,
    reserved_bytes: 0,
    remaining_bytes: policy.totalCapBytes,
    status: 'ready',
    segments,
    history: [],
    last_run: null,
  };
}

export function validateLedger(ledger, { config, baseline, policy = POLICY, now = Date.now() }) {
  validatePolicy(policy);
  if (!ledger || typeof ledger !== 'object' || Array.isArray(ledger)) fail('ledger', 'ledger inválido');
  if (ledger.schema !== SCHEMA) fail('ledger', 'schema do ledger desconhecido');
  if (!ledger.ledger_path || !samePath(ledger.ledger_path, config.ledger)) fail('ledger', 'ledger em caminho diferente');
  if (!ledger.repo_root || !samePath(ledger.repo_root, config.repoRoot)) fail('ledger', 'repo diferente');
  if (!ledger.target_root || !samePath(ledger.target_root, config.targetRoot)) fail('ledger', 'target diferente');
  if (!ledger.baseline_path || !samePath(ledger.baseline_path, baseline.path)) fail('ledger', 'baseline diferente');
  if (ledger.baseline_sha256 !== baseline.hash) fail('ledger', 'baseline mudou');
  if (ledger.baseline_bytes !== baseline.bytes) fail('ledger', 'baseline bytes divergem');
  if (ledger.baseline_approx !== baseline.approx) fail('ledger', 'baseline aproximação diverge');
  if (ledger.baseline_uncertainty_bytes !== baseline.uncertaintyBytes) fail('ledger', 'incerteza diverge');
  if (ledger.total_cap_bytes !== policy.totalCapBytes) fail('ledger', 'cap foi alterado');
  if (ledger.floor_bytes !== policy.floorBytes) fail('ledger', 'piso foi alterado');
  if (ledger.reserve_bytes !== policy.reserveBytes) fail('ledger', 'reserva foi alterada');
  if (typeof ledger.resumption_id !== 'string' || !ledger.resumption_id.trim()) fail('ledger', 'resumption_id ausente');
  if (!isSafeInteger(ledger.external_decrease_bytes)) fail('ledger', 'diminuição externa inválida');
  if (typeof ledger.external_decrease_approx !== 'boolean') fail('ledger', 'aproximação externa inválida');
  if (Number.isNaN(Date.parse(ledger.external_decrease_at))) fail('ledger', 'data de diminuição inválida');
  if (!isSafeInteger(ledger.used_bytes) || !isSafeInteger(ledger.reserved_bytes)) fail('ledger', 'saldos inválidos');
  if (ledger.reserved_bytes > policy.reserveBytes) fail('ledger', 'reserva acima do autorizado');
  if (!Number.isSafeInteger(ledger.remaining_bytes)) fail('ledger', 'saldo restante inválido');
  const created = Date.parse(ledger.created_at);
  const updated = Date.parse(ledger.updated_at);
  if (Number.isNaN(created) || Number.isNaN(updated) || created > updated) fail('ledger', 'timestamps contraditórios');
  if (updated > now + 5 * 60_000) fail('ledger', 'timestamp futuro');
  if (!Array.isArray(ledger.segments) || ledger.segments.length === 0) fail('ledger', 'segmentos ausentes');
  const names = new Set();
  let used = 0;
  for (const segment of ledger.segments) {
    if (!segment || typeof segment.name !== 'string' || names.has(segment.name)) fail('ledger', 'segmento duplicado');
    names.add(segment.name);
    if (typeof segment.path !== 'string' || !path.isAbsolute(segment.path)) fail('ledger', 'path de segmento inválido');
    for (const key of ['start_bytes', 'high_water_bytes', 'charged_bytes', 'last_observed_bytes']) {
      if (!isSafeInteger(segment[key])) fail('ledger', 'bytes de segmento inválidos');
    }
    if (segment.high_water_bytes < segment.start_bytes) fail('ledger', 'highwater abaixo do início');
    if (segment.charged_bytes !== segment.high_water_bytes - segment.start_bytes) fail('ledger', 'cobrança divergente');
    used += segment.charged_bytes;
    if (!Number.isSafeInteger(used)) fail('ledger', 'soma de uso excedeu o inteiro seguro');
  }
  const expected = new Map([
    ...config.roots.map((root) => [root.name, { path: root.path, kind: 'root' }]),
    ...config.caches.map((cache) => [cache.name, { path: cache.path, kind: 'cache' }]),
  ]);
  if (ledger.segments.length !== expected.size) fail('ledger', 'conjunto de segmentos mudou');
  for (const segment of ledger.segments) {
    const wanted = expected.get(segment.name);
    if (!wanted || !samePath(segment.path, wanted.path) || segment.kind !== wanted.kind) {
      fail('ledger', 'segmento não corresponde à configuração');
    }
  }
  if (used !== ledger.used_bytes) fail('ledger', 'uso não reconcilia');
  if (ledger.remaining_bytes !== policy.totalCapBytes - ledger.used_bytes - ledger.reserved_bytes) {
    fail('ledger', 'saldo não reconcilia');
  }
  if (!['ready', 'ok', 'command-failed', 'blocked', 'error', 'cancelled'].includes(ledger.status)) {
    fail('ledger', 'status do ledger inválido');
  }
  if (ledger.status === 'running') fail('ledger', 'ledger em execução; lock/falha anterior exige revisão');
  if (!Array.isArray(ledger.history) || ledger.history.length > MAX_HISTORY) fail('ledger', 'histórico inválido');
  if (!Array.isArray(ledger.external_decreases)) fail('ledger', 'segmentos externos ausentes');
  if (ledger.last_run && ledger.last_run.started_at) {
    const started = Date.parse(ledger.last_run.started_at);
    if (Number.isNaN(started) || started < created) fail('ledger', 'execução anterior contraditória');
  }
  return ledger;
}

export function updateSegments(ledger, measured, { policy = POLICY, now = Date.now() } = {}) {
  const byName = new Map(measured);
  let used = 0;
  for (const segment of ledger.segments) {
    const value = byName.get(segment.name);
    if (!value || !samePath(value.path, segment.path)) fail('measurement', 'segmento medido não confere');
    const previous = segment.last_observed_bytes ?? segment.high_water_bytes;
    if (value.bytes < previous) {
      ledger.external_decreases.push({
        at: nowIso(now),
        kind: 'observed-decrease',
        segment: segment.name,
        before_bytes: previous,
        after_bytes: value.bytes,
        bytes: previous - value.bytes,
      });
      if (ledger.external_decreases.length > MAX_DECREASES) ledger.external_decreases.shift();
    }
    segment.last_observed_bytes = value.bytes;
    segment.high_water_bytes = Math.max(segment.high_water_bytes, value.bytes);
    segment.charged_bytes = segment.high_water_bytes - segment.start_bytes;
    used += segment.charged_bytes;
    if (!Number.isSafeInteger(used)) fail('measurement', 'uso excedeu inteiro seguro');
  }
  ledger.used_bytes = used;
  ledger.reserved_bytes = Math.min(ledger.reserved_bytes, policy.reserveBytes);
  ledger.remaining_bytes = policy.totalCapBytes - used - ledger.reserved_bytes;
  ledger.updated_at = nowIso(now);
  return ledger;
}

function writeLedgerAtomic(ledgerPath, ledger) {
  const directory = path.dirname(ledgerPath);
  let directoryStat;
  try {
    directoryStat = fs.lstatSync(directory);
  } catch {
    fail('ledger', 'diretório do ledger não existe');
  }
  if (!directoryStat.isDirectory() || directoryStat.isSymbolicLink()) fail('ledger', 'diretório do ledger inválido');
  const temp = path.join(directory, `.${path.basename(ledgerPath)}.tmp-${process.pid}-${crypto.randomUUID()}`);
  const text = `${JSON.stringify(ledger, null, 2)}\n`;
  let fd;
  try {
    fd = fs.openSync(temp, 'wx', 0o600);
    fs.writeFileSync(fd, text, 'utf8');
    fs.fsyncSync(fd);
    fs.closeSync(fd);
    fd = undefined;
    fs.renameSync(temp, ledgerPath);
  } catch (error) {
    if (fd !== undefined) {
      try { fs.closeSync(fd); } catch { /* falha já reportada */ }
    }
    fail('ledger', `não foi possível persistir o ledger (${error.code ?? 'erro'})`);
  }
}

function acquireLock(lockPath) {
  try {
    const fd = fs.openSync(lockPath, 'wx', 0o600);
    try {
      fs.writeFileSync(fd, JSON.stringify({ pid: process.pid, started_at: nowIso() }) + '\n', 'utf8');
      fs.fsyncSync(fd);
    } finally {
      fs.closeSync(fd);
    }
  } catch (error) {
    if (error.code === 'EEXIST') fail('lock', 'lock já existe; execução anterior exige revisão');
    fail('lock', 'não foi possível criar o lock');
  }
}

function releaseLock(lockPath) {
  try {
    fs.unlinkSync(lockPath);
  } catch {
    fail('lock', 'lock não pôde ser removido após o ledger');
  }
}

function ledgerLockPath(ledgerPath) {
  return `${ledgerPath}.lock`;
}

function loadLedger(ledgerPath, config, baseline, policy, now) {
  let stat;
  try {
    stat = fs.lstatSync(ledgerPath);
  } catch (error) {
    if (error.code === 'ENOENT') fail('ledger', 'ledger ausente; use --init-ledger no primeiro uso');
    fail('ledger', 'ledger não pôde ser lido');
  }
  if (stat.isSymbolicLink() || !stat.isFile()) fail('ledger', 'ledger não é arquivo regular');
  return validateLedger(readJsonFile(ledgerPath, 'ledger'), { config, baseline, policy, now });
}

function ensurePaths(config) {
  if (!fs.existsSync(config.sagaRoot)) fail('config', 'saga não existe');
  if (!isInside(config.ledger, config.sagaRoot)) fail('config', 'ledger deve ficar sob a saga');
  if (isInside(config.ledger, config.repoRoot) || samePath(config.ledger, config.repoRoot)) {
    fail('config', 'ledger não pode ficar no repositório');
  }
  if (!fs.existsSync(path.dirname(config.ledger))) fail('config', 'diretório do ledger não existe');
  if (!fs.existsSync(config.cwd)) fail('config', 'cwd não existe');
  for (const volume of config.volumes) {
    if (!fs.existsSync(volume)) fail('config', 'volume declarado não existe');
  }
}

function checkFree({ config, policy, freeBytes = freeBytesForPath }) {
  const volumePaths = [config.targetRoot, ...config.volumes];
  const pg = config.roots.find((root) => root.name === 'pg');
  if (pg) volumePaths.push(pg.path);
  const seen = new Set();
  const minimum = policy.floorBytes + policy.reserveBytes;
  const readFree = typeof freeBytes === 'function' ? freeBytes : () => freeBytes;
  for (const value of volumePaths) {
    const root = path.parse(path.resolve(value)).root;
    const key = normalize(root);
    if (seen.has(key)) continue;
    seen.add(key);
    let free;
    try {
      free = readFree(root);
    } catch {
      fail('measurement', 'espaço livre não pôde ser medido');
    }
    if (!isSafeInteger(free) || free < minimum) {
      fail('blocked', 'piso de espaço livre insuficiente');
    }
  }
}

function policyBlocked(ledger, policy) {
  if (ledger.used_bytes + policy.reserveBytes > policy.totalCapBytes) return 'cap';
  if (ledger.remaining_bytes < 0) return 'cap';
  return null;
}

function summary(ledger, { status, exitCode, reason, childExitCode = null, childSignal = null }) {
  return {
    status,
    exitCode,
    reason: reason ?? null,
    runId: ledger.last_run?.run_id ?? null,
    usedBytes: ledger.used_bytes,
    reservedBytes: ledger.reserved_bytes,
    remainingBytes: ledger.remaining_bytes,
    totalCapBytes: ledger.total_cap_bytes,
    floorBytes: ledger.floor_bytes,
    externalDecreaseBytes: ledger.external_decrease_bytes,
    externalDecreaseApprox: ledger.external_decrease_approx,
    childExitCode,
    childSignal,
    updatedAt: ledger.updated_at,
  };
}

function statusExit(status, childCode, childSignal = null) {
  if (status === 'ok') return 0;
  if (status === 'command-failed') {
    if (Number.isInteger(childCode) && childCode >= 0) return childCode;
    const signals = { SIGINT: 2, SIGKILL: 9, SIGTERM: 15 };
    return childSignal ? 128 + (signals[childSignal] ?? 0) : 4;
  }
  if (status === 'blocked') return 3;
  if (status === 'cancelled') return 130;
  return 4;
}

function rememberRun(ledger, { status, exitCode, reason, childExitCode, childSignal, startedAt, runId }) {
  ledger.status = status;
  ledger.reserved_bytes = 0;
  ledger.remaining_bytes = ledger.total_cap_bytes - ledger.used_bytes;
  ledger.updated_at = nowIso();
  ledger.last_run = {
    run_id: runId,
    started_at: startedAt,
    ended_at: ledger.updated_at,
    status,
    exit_code: exitCode,
    reason: reason ?? null,
    child_exit_code: childExitCode,
    child_signal: childSignal,
    used_bytes: ledger.used_bytes,
    remaining_bytes: ledger.remaining_bytes,
  };
  ledger.history.push({ ...ledger.last_run });
  if (ledger.history.length > MAX_HISTORY) ledger.history.shift();
}

function waitForExit(child, timeoutMs) {
  if (!child || child.exitCode !== null || child.signalCode !== null) return Promise.resolve(true);
  return new Promise((resolve) => {
    let settled = false;
    const timer = setTimeout(() => {
      if (settled) return;
      settled = true;
      child.removeListener('exit', onExit);
      resolve(false);
    }, timeoutMs);
    function onExit() {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      resolve(true);
    }
    child.once('exit', onExit);
  });
}

export async function stopOwnedChild(child, { graceMs = DEFAULT_STOP_GRACE_MS } = {}) {
  if (!child || child.exitCode !== null || child.signalCode !== null) return true;
  if (process.platform === 'win32') {
    if (!Number.isInteger(child.pid) || child.pid <= 0) return false;
    try {
      execFileSync('taskkill', ['/PID', String(child.pid), '/T', '/F'], {
        windowsHide: true,
        stdio: 'ignore',
        timeout: 10_000,
      });
      return await waitForExit(child, 1_000);
    } catch {
      try { child.kill('SIGKILL'); } catch { /* sem fallback */ }
      return waitForExit(child, 1_000);
    }
  }
  try {
    process.kill(-child.pid, 'SIGTERM');
  } catch {
    try { child.kill('SIGTERM'); } catch { /* sem fallback */ }
  }
  if (await waitForExit(child, graceMs)) return true;
  try {
    process.kill(-child.pid, 'SIGKILL');
  } catch {
    try { child.kill('SIGKILL'); } catch { /* sem fallback */ }
  }
  return waitForExit(child, 2_000);
}

function runCommand({ config, onFreeSample, onRootSample, timeoutSeconds, sampleMs, rootSampleMs }) {
  return new Promise((resolve) => {
    let child;
    try {
      child = spawn(config.command, config.args, {
        cwd: config.cwd,
        env: process.env,
        shell: false,
        windowsHide: true,
        stdio: 'inherit',
        detached: process.platform !== 'win32',
      });
    } catch {
      resolve({ status: 'error', reason: 'spawn', childExitCode: null, childSignal: null });
      return;
    }
    let settled = false;
    let aborting = false;
    let sampling = false;
    let timeoutTimer = null;
    let freeTimer = null;
    let rootTimer = null;
    let onSignal = null;
    const cleanupTimers = () => {
      if (timeoutTimer) clearTimeout(timeoutTimer);
      if (freeTimer) clearInterval(freeTimer);
      if (rootTimer) clearInterval(rootTimer);
    };
    const cleanupSignals = () => {
      if (!onSignal) return;
      process.removeListener('SIGINT', onSignal);
      process.removeListener('SIGTERM', onSignal);
    };
    const finish = (value) => {
      if (settled) return;
      settled = true;
      cleanupTimers();
      cleanupSignals();
      resolve(value);
    };
    const abort = async (status, reason, childExitCode = null, childSignal = null) => {
      if (settled || aborting) return;
      aborting = true;
      cleanupTimers();
      const stopped = await stopOwnedChild(child);
      if (settled) return;
      settled = true;
      cleanupSignals();
      resolve({
        status: stopped ? status : 'error',
        reason: stopped ? reason : 'stop',
        childExitCode: child.exitCode ?? childExitCode,
        childSignal: child.signalCode ?? childSignal,
      });
    };
    const sampleFree = () => {
      if (settled || sampling) return;
      try {
        onFreeSample();
      } catch (error) {
        if (error?.code === 'blocked') abort('blocked', 'floor');
        else abort('error', 'measurement');
      }
    };
    const sampleRoots = () => {
      if (settled || sampling) return;
      sampling = true;
      try {
        onRootSample();
        sampling = false;
      } catch (error) {
        sampling = false;
        if (error?.code === 'blocked') abort('blocked', 'cap');
        else abort('error', 'measurement');
      }
    };
    if (timeoutSeconds > 0) {
      timeoutTimer = setTimeout(() => { void abort('blocked', 'timeout'); }, timeoutSeconds * 1_000);
    }
    freeTimer = setInterval(sampleFree, sampleMs);
    rootTimer = setInterval(sampleRoots, rootSampleMs);
    onSignal = (signal) => { void abort('cancelled', signal === 'SIGTERM' ? 'sigterm' : 'sigint'); };
    process.once('SIGINT', onSignal);
    process.once('SIGTERM', onSignal);
    child.once('error', () => {
      if (settled || aborting) return;
      finish({ status: 'error', reason: 'spawn', childExitCode: null, childSignal: null });
    });
    child.once('exit', (code, signal) => {
      cleanupTimers();
      if (settled || aborting) return;
      settled = true;
      cleanupSignals();
      resolve({
        status: code === 0 && !signal ? 'ok' : 'command-failed',
        reason: signal ? 'signal' : (code === 0 ? null : 'child'),
        childExitCode: code,
        childSignal: signal,
      });
    });
  });
}

function assertRootConfig(config) {
  const roots = new Map(config.roots.map((root) => [root.name, root]));
  for (const name of ['target', 'repo', 'saga']) if (!roots.has(name)) fail('config', `--root ${name} é obrigatório`);
  if (!isInside(roots.get('target').path, roots.get('repo').path)) fail('config', 'target deve estar sob repo');
  if (!isInside(config.ledger, roots.get('saga').path)) fail('config', 'ledger deve ficar sob a saga');
  const paths = [...config.roots, ...config.caches].map((item) => normalize(item.path));
  if (new Set(paths).size !== paths.length) fail('config', 'raízes/caches com path duplicado');
}

export function makeConfig(parsed, policy = POLICY) {
  validatePolicy(policy);
  const roots = parsed.roots;
  const target = roots.find((root) => root.name === 'target');
  const repo = roots.find((root) => root.name === 'repo');
  const saga = roots.find((root) => root.name === 'saga');
  if (!target || !repo || !saga) fail('config', 'roots obrigatórias ausentes');
  const config = {
    ledger: parsed.ledger,
    baselinePath: parsed.baseline,
    roots,
    caches: parsed.caches,
    volumes: parsed.volumes,
    cwd: parsed.cwd,
    timeoutSeconds: parsed.timeoutSeconds,
    command: parsed.command,
    args: parsed.args,
    targetRoot: target.path,
    repoRoot: repo.path,
    sagaRoot: saga.path,
  };
  assertRootConfig(config);
  ensurePaths(config);
  return config;
}

export async function runWithBudget(parsed, deps = {}) {
  const runId = crypto.randomUUID();
  const startedAt = nowIso();
  let policy;
  let config;
  let baseline;
  try {
    policy = validatePolicy(deps.policy ?? POLICY);
    config = deps.config ?? makeConfig(parsed, policy);
    baseline = deps.baseline ?? readBaseline(config.baselinePath, config.targetRoot);
  } catch (error) {
    return { status: 'error', exitCode: 4, reason: error?.code ?? 'error', runId, updatedAt: nowIso(), ledger: null };
  }
  const lockPath = ledgerLockPath(config.ledger);
  let ledger;
  let lockHeld = false;
  try {
    acquireLock(lockPath);
    lockHeld = true;
    const measured = measureConfig(config, { policy, measure: deps.measureTree });
    if (parsed.initLedger) {
      let exists = true;
      try { fs.lstatSync(config.ledger); } catch (error) { if (error.code === 'ENOENT') exists = false; else fail('ledger', 'ledger não pôde ser verificado'); }
      if (exists) fail('ledger', '--init-ledger não sobrescreve ledger existente');
      ledger = buildLedger({ config, baseline, measured, policy, now: Date.parse(startedAt) });
    } else {
      ledger = loadLedger(config.ledger, config, baseline, policy, Date.now());
      updateSegments(ledger, measured, { policy });
    }
    const blocked = policyBlocked(ledger, policy);
    if (blocked) {
      rememberRun(ledger, {
        status: 'blocked',
        exitCode: statusExit('blocked'),
        reason: blocked,
        childExitCode: null,
        childSignal: null,
        startedAt,
        runId,
      });
      writeLedgerAtomic(config.ledger, ledger);
      if (lockHeld) releaseLock(lockPath);
      return { ...summary(ledger, { status: 'blocked', exitCode: 3, reason: blocked }), ledger };
    }
    checkFree({ config, policy, freeBytes: deps.freeBytes });
    ledger.reserved_bytes = policy.reserveBytes;
    ledger.remaining_bytes = ledger.total_cap_bytes - ledger.used_bytes - ledger.reserved_bytes;
    ledger.status = 'running';
    ledger.last_run = { run_id: runId, started_at: startedAt, status: 'running' };
    ledger.updated_at = nowIso();
    writeLedgerAtomic(config.ledger, ledger);
    const onFreeSample = () => {
      checkFree({ config, policy, freeBytes: deps.freeBytes });
    };
    const onRootSample = () => {
      const current = measureConfig(config, { policy, measure: deps.measureTree });
      updateSegments(ledger, current, { policy });
      if (ledger.used_bytes + policy.reserveBytes > policy.totalCapBytes) fail('blocked', 'cap');
    };
    const childResult = await runCommand({
      config,
      onFreeSample,
      onRootSample,
      timeoutSeconds: config.timeoutSeconds,
      sampleMs: deps.freeSampleMs ?? DEFAULT_FREE_SAMPLE_MS,
      rootSampleMs: deps.rootSampleMs ?? DEFAULT_ROOT_SAMPLE_MS,
    });
    let finalMeasurement = null;
    try {
      finalMeasurement = measureConfig(config, { policy, measure: deps.measureTree });
      updateSegments(ledger, finalMeasurement, { policy });
    } catch {
      ledger.reserved_bytes = 0;
      ledger.remaining_bytes = ledger.total_cap_bytes - ledger.used_bytes;
      rememberRun(ledger, {
        status: 'error',
        exitCode: 4,
        reason: 'measurement',
        childExitCode: childResult.childExitCode,
        childSignal: childResult.childSignal,
        startedAt,
        runId,
      });
      writeLedgerAtomic(config.ledger, ledger);
      if (lockHeld) releaseLock(lockPath);
      return { ...summary(ledger, { status: 'error', exitCode: 4, reason: 'measurement', childExitCode: childResult.childExitCode, childSignal: childResult.childSignal }), ledger };
    }
    let status = childResult.status;
    let reason = childResult.reason;
    if (ledger.used_bytes + 0 > policy.totalCapBytes) {
      status = 'blocked';
      reason = 'cap';
    } else if (status === 'blocked' || status === 'error') {
      // preserva o motivo do monitor
    }
    const exitCode = statusExit(status, childResult.childExitCode, childResult.childSignal);
    rememberRun(ledger, {
      status,
      exitCode,
      reason,
      childExitCode: childResult.childExitCode,
      childSignal: childResult.childSignal,
      startedAt,
      runId,
    });
    writeLedgerAtomic(config.ledger, ledger);
    if (lockHeld) releaseLock(lockPath);
    return { ...summary(ledger, { status, exitCode, reason, childExitCode: childResult.childExitCode, childSignal: childResult.childSignal }), ledger };
  } catch (error) {
    if (ledger) {
      try {
        ledger.reserved_bytes = 0;
        ledger.remaining_bytes = ledger.total_cap_bytes - ledger.used_bytes;
        const status = error?.code === 'blocked' ? 'blocked' : 'error';
        const exitCode = status === 'blocked' ? 3 : 4;
        const reason = error?.code === 'blocked'
          ? (String(error.message).includes('piso') ? 'floor' : 'cap')
          : 'error';
        rememberRun(ledger, {
          status,
          exitCode,
          reason,
          childExitCode: null,
          childSignal: null,
          startedAt,
          runId,
        });
        writeLedgerAtomic(config.ledger, ledger);
        if (lockHeld) releaseLock(lockPath);
        return { ...summary(ledger, { status, exitCode, reason }), ledger };
      } catch {
        // mantém o lock e devolve erro sem alegar sucesso
      }
    }
    return {
      status: 'error',
      exitCode: 4,
      reason: error?.code ?? 'error',
      runId,
      updatedAt: nowIso(),
      ledger: null,
    };
  }
}

export async function runCli(argv, deps = {}) {
  let parsed;
  try {
    parsed = parseArgs(argv);
    if (parsed.help) return { code: 0, stdout: `${USAGE}\n`, stderr: '' };
    const result = await runWithBudget(parsed, deps);
    return {
      code: result.exitCode,
      stdout: '',
      stderr: `${JSON.stringify(result, (key, value) => key === 'ledger' ? undefined : value)}\n`,
    };
  } catch (error) {
    if (error instanceof RunBudgetError) {
      return { code: 4, stdout: '', stderr: `run-with-disk-budget: ${error.message}\n` };
    }
    return { code: 4, stdout: '', stderr: 'run-with-disk-budget: erro inesperado\n' };
  }
}

function isDirectExecution() {
  const entry = process.argv[1];
  if (!entry) return false;
  try {
    return path.resolve(entry) === path.resolve(fileURLToPath(import.meta.url));
  } catch {
    return false;
  }
}

if (isDirectExecution()) {
  const result = await runCli(process.argv.slice(2));
  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  process.exitCode = result.code;
}
