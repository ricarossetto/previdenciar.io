#!/usr/bin/env node
/**
 * Monitor de orçamento de disco para builds seriais.
 *
 * Uso:
 *   node scripts/disk-budget.mjs --check --baseline <arquivo-ou-gb> \
 *     [--floor-gb 10] [--growth-cap-gb 2] [--target <pasta>]
 *
 * Saída: JSON com freeGb, targetGb, growthGb, status e motivos.
 * Código de saída: 0 = ok; 2 = blocked; 1 = erro de leitura/configuração.
 *
 * Política autorizada: piso de 10 GiB livres e crescimento acumulado de no
 * máximo 2 GiB desde a liberação do target. O script não recalcula o baseline:
 * ele lê o arquivo indicado ou o valor numérico explícito e recusa baseline
 * inválido, alvo ausente, link/reparsepoint ou política mais fraca.
 *
 * Medição:
 * - espaço livre: fs.statfsSync na raiz do volume; fallback de leitura via
 *   Get-PSDrive no Windows quando statfs não estiver disponível.
 * - target: soma de metadados (tamanho de arquivos), sem ler conteúdo. Não usa
 *   tamanho antigo/cache para liberar build; cada --check mede o estado atual.
 *
 * Limitação TOCTOU: a medição é um retrato. Entre o --check e o build, outro
 * processo pode consumir espaço ou crescer o target. O build deve rodar logo
 * após o check e o wrapper que executa o build deve repetir a medição. Este
 * script não monitora um comando durante a execução.
 */
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const GIB = 1024 ** 3;
const MIB = 1024 ** 2;
const AUTHORIZED_FLOOR_GB = 10;
const AUTHORIZED_GROWTH_CAP_GB = 2;
const DEFAULT_UNCERTAINTY_GB = 0.01;
const MAX_TARGET_ENTRIES = 2_000_000;
const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const DEFAULT_TARGET = path.resolve(SCRIPT_DIR, '..', 'target');

const USAGE = `Monitor de orçamento de disco

Uso:
  node scripts/disk-budget.mjs --check --baseline <arquivo-ou-gb> \\
    [--floor-gb ${AUTHORIZED_FLOOR_GB}] [--growth-cap-gb ${AUTHORIZED_GROWTH_CAP_GB}] [--target <pasta>]

Saída: JSON com freeGb, targetGb, growthGb, status e motivos.
Código: 0 ok, 2 blocked, 1 erro.

Baseline em arquivo JSON: target_path, created_at, resumption_id,
baseline_source, baseline_bytes (exato) ou baseline_gb + baseline_approx,
floor_gb, growth_cap_gb.
Números são aproximados; para prova exata use baseline_bytes.
O script não escreve, não move, não apaga e não cria cache.`;

export class BudgetError extends Error {
  constructor(message) {
    super(message);
    this.name = 'BudgetError';
  }
}

function fail(message) {
  throw new BudgetError(message);
}

function isFiniteNonNegative(value) {
  return typeof value === 'number' && Number.isFinite(value) && value >= 0;
}

function normalizePath(value) {
  const resolved = path.resolve(String(value));
  return process.platform === 'win32' ? resolved.toLowerCase() : resolved;
}

function round(value, digits = 3) {
  return Number(value.toFixed(digits));
}

function parsePositiveNumber(value, label) {
  const number = Number(value);
  if (!Number.isFinite(number) || number <= 0) {
    fail(`${label} deve ser um número positivo`);
  }
  return number;
}

function readBaselineFile(filePath) {
  let text;
  try {
    text = fs.readFileSync(filePath, 'utf8');
  } catch (error) {
    fail(`não foi possível ler o baseline ${filePath}: ${error.message}`);
  }
  try {
    return JSON.parse(text);
  } catch (error) {
    fail(`baseline ${filePath} não é JSON válido: ${error.message}`);
  }
}

/**
 * Valida um objeto de baseline em memória. Exportado para testes, sem I/O.
 */
export function parseBaselineObject(raw, { targetPath } = {}) {
  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) {
    fail('baseline deve ser um objeto JSON');
  }
  if (!targetPath) fail('targetPath é obrigatório para validar o baseline');
  if (typeof raw.target_path !== 'string' || !raw.target_path.trim()) {
    fail('baseline.target_path é obrigatório');
  }
  if (normalizePath(raw.target_path) !== normalizePath(targetPath)) {
    fail(`baseline.target_path (${raw.target_path}) não corresponde ao target (${targetPath})`);
  }
  if (typeof raw.created_at !== 'string' || Number.isNaN(Date.parse(raw.created_at))) {
    fail('baseline.created_at é obrigatório e deve ser uma data válida');
  }
  if (typeof raw.resumption_id !== 'string' || !raw.resumption_id.trim()) {
    fail('baseline.resumption_id é obrigatório');
  }
  if (typeof raw.baseline_source !== 'string' || !raw.baseline_source.trim()) {
    fail('baseline.baseline_source é obrigatório');
  }

  let bytes = null;
  let gb = null;
  if (raw.baseline_bytes !== undefined) {
    if (!Number.isSafeInteger(raw.baseline_bytes) || raw.baseline_bytes <= 0) {
      fail('baseline.baseline_bytes deve ser inteiro positivo em bytes');
    }
    bytes = raw.baseline_bytes;
  }
  if (raw.baseline_gb !== undefined) {
    gb = parsePositiveNumber(raw.baseline_gb, 'baseline.baseline_gb');
  }
  if (bytes === null && gb === null) {
    fail('baseline precisa de baseline_bytes ou baseline_gb');
  }
  if (bytes !== null && gb !== null && Math.abs(bytes - gb * GIB) > MIB) {
    fail('baseline_bytes e baseline_gb divergem; baseline inconsistente');
  }

  const approx = raw.baseline_approx === true;
  if (bytes === null && !approx) {
    fail('baseline_gb sem baseline_bytes exige baseline_approx=true');
  }
  if (approx && !raw.baseline_source) {
    fail('baseline aproximado exige baseline_source');
  }
  const uncertaintyGb = raw.baseline_uncertainty_gb === undefined
    ? (approx ? DEFAULT_UNCERTAINTY_GB : 0)
    : Number(raw.baseline_uncertainty_gb);
  if (!isFiniteNonNegative(uncertaintyGb) || uncertaintyGb > 1) {
    fail('baseline.baseline_uncertainty_gb deve estar entre 0 e 1 GiB');
  }

  const floorGb = Number(raw.floor_gb);
  const growthCapGb = Number(raw.growth_cap_gb);
  if (!Number.isFinite(floorGb) || floorGb <= 0) fail('baseline.floor_gb inválido');
  if (!Number.isFinite(growthCapGb) || growthCapGb <= 0) fail('baseline.growth_cap_gb inválido');
  if (floorGb < AUTHORIZED_FLOOR_GB) {
    fail(`baseline.floor_gb ${floorGb} enfraquece o piso autorizado de ${AUTHORIZED_FLOOR_GB} GiB`);
  }
  if (growthCapGb > AUTHORIZED_GROWTH_CAP_GB) {
    fail(`baseline.growth_cap_gb ${growthCapGb} enfraquece o teto autorizado de ${AUTHORIZED_GROWTH_CAP_GB} GiB`);
  }

  const baselineBytes = bytes ?? gb * GIB;
  if (!Number.isFinite(baselineBytes) || baselineBytes <= 0) {
    fail('baseline sem tamanho positivo');
  }
  if (approx && uncertaintyGb * GIB >= baselineBytes) {
    fail('incerteza do baseline maior que o próprio baseline');
  }

  return {
    baselineBytes,
    baselineGb: baselineBytes / GIB,
    approx,
    uncertaintyGb,
    source: raw.baseline_source,
    createdAt: raw.created_at,
    resumptionId: raw.resumption_id,
    targetPath: path.resolve(raw.target_path),
    floorGb,
    growthCapGb,
  };
}

function parseBaseline(input, { targetPath }) {
  const text = String(input ?? '').trim();
  if (!text) fail('--baseline é obrigatório');
  if (/^\d+(?:\.\d+)?$/.test(text)) {
    const gb = parsePositiveNumber(text, '--baseline');
    return {
      baselineBytes: gb * GIB,
      baselineGb: gb,
      approx: true,
      uncertaintyGb: DEFAULT_UNCERTAINTY_GB,
      source: 'CLI numérico (aproximado; sem baseline exato)',
      createdAt: new Date().toISOString(),
      resumptionId: 'cli',
      targetPath,
      floorGb: AUTHORIZED_FLOOR_GB,
      growthCapGb: AUTHORIZED_GROWTH_CAP_GB,
    };
  }
  const filePath = path.resolve(text);
  return parseBaselineObject(readBaselineFile(filePath), { targetPath });
}

function resolvePolicy(cliFloorGb, cliGrowthCapGb, baseline) {
  const baselineFloor = baseline?.floorGb ?? AUTHORIZED_FLOOR_GB;
  const baselineCap = baseline?.growthCapGb ?? AUTHORIZED_GROWTH_CAP_GB;
  const floorGb = cliFloorGb === null ? baselineFloor : cliFloorGb;
  const growthCapGb = cliGrowthCapGb === null ? baselineCap : cliGrowthCapGb;

  if (!Number.isFinite(floorGb) || floorGb <= 0) fail('--floor-gb inválido');
  if (!Number.isFinite(growthCapGb) || growthCapGb <= 0) fail('--growth-cap-gb inválido');
  if (floorGb < AUTHORIZED_FLOOR_GB) {
    fail(`--floor-gb ${floorGb} enfraquece o piso autorizado de ${AUTHORIZED_FLOOR_GB} GiB; decisão explícita necessária`);
  }
  if (growthCapGb > AUTHORIZED_GROWTH_CAP_GB) {
    fail(`--growth-cap-gb ${growthCapGb} enfraquece o teto autorizado de ${AUTHORIZED_GROWTH_CAP_GB} GiB; decisão explícita necessária`);
  }
  if (floorGb < baselineFloor) {
    fail(`--floor-gb ${floorGb} enfraquece o baseline (${baselineFloor} GiB)`);
  }
  if (growthCapGb > baselineCap) {
    fail(`--growth-cap-gb ${growthCapGb} enfraquece o baseline (${baselineCap} GiB)`);
  }
  return { floorGb, growthCapGb };
}

function assertNoLink(targetPath, stat) {
  if (stat.isSymbolicLink()) {
    fail(`target ${targetPath} é symlink/reparsepoint; não seguir`);
  }
}

/**
 * Soma metadados do target. Não lê conteúdo, mas percorre a árvore; por isso
 * não usa cache antigo para liberar build. Links são recusados.
 */
export function measureTargetBytes(targetPath) {
  const root = path.resolve(targetPath);
  let rootStat;
  try {
    rootStat = fs.lstatSync(root);
  } catch (error) {
    fail(`target ${root} não pôde ser medido: ${error.message}`);
  }
  assertNoLink(root, rootStat);
  if (!rootStat.isDirectory()) fail(`target ${root} não é diretório`);

  let realRoot;
  try {
    realRoot = fs.realpathSync.native(root);
  } catch (error) {
    fail(`target ${root} não pôde ser resolvido: ${error.message}`);
  }
  if (normalizePath(realRoot) !== normalizePath(root)) {
    fail(`target ${root} resolve por link/reparsepoint (${realRoot}); não seguir`);
  }

  let bytes = 0;
  let entries = 0;
  const pending = [root];
  while (pending.length) {
    const directory = pending.pop();
    let children;
    try {
      children = fs.readdirSync(directory, { withFileTypes: true });
    } catch (error) {
      fail(`falha ao ler ${directory}: ${error.message}`);
    }
    for (const child of children) {
      entries += 1;
      if (entries > MAX_TARGET_ENTRIES) {
        fail(`target excedeu ${MAX_TARGET_ENTRIES} entradas; revisar antes de medir`);
      }
      const full = path.join(directory, child.name);
      let stat;
      try {
        stat = fs.lstatSync(full);
      } catch (error) {
        fail(`falha ao medir ${full}: ${error.message}`);
      }
      assertNoLink(full, stat);
      if (stat.isDirectory()) {
        pending.push(full);
      } else if (stat.isFile()) {
        if (!Number.isSafeInteger(stat.size) || stat.size < 0) {
          fail(`tamanho inválido em ${full}`);
        }
        bytes += stat.size;
        if (!Number.isSafeInteger(bytes)) fail('soma de bytes do target excedeu o inteiro seguro');
      } else {
        fail(`entrada não regular no target: ${full}`);
      }
    }
  }
  return { bytes, entries, root };
}

function freeBytesViaPowerShell(root) {
  const drive = root.replace(/[\\/:]/g, '');
  if (!/^[A-Za-z]$/.test(drive)) fail(`não foi possível identificar a unidade de ${root}`);
  let output;
  try {
    output = execFileSync(
      'powershell.exe',
      ['-NoProfile', '-NonInteractive', '-Command', `(Get-PSDrive -Name '${drive}').Free`],
      { encoding: 'utf8', timeout: 15_000, windowsHide: true },
    ).trim();
  } catch (error) {
    fail(`Get-PSDrive falhou para ${root}: ${error.message}`);
  }
  if (!/^\d+$/.test(output)) fail(`Get-PSDrive devolveu valor inválido para ${root}`);
  const bytes = Number(output);
  if (!Number.isSafeInteger(bytes) || bytes < 0) fail(`Get-PSDrive devolveu bytes inválidos para ${root}`);
  return bytes;
}

/** Espaço livre do volume que contém o target; somente leitura. */
export function freeBytesForPath(targetPath) {
  const root = path.parse(path.resolve(targetPath)).root;
  try {
    const stat = fs.statfsSync(root);
    const bytes = stat.bavail * stat.bsize;
    if (!Number.isSafeInteger(bytes) || bytes < 0) {
      fail(`fs.statfs devolveu bytes inválidos para ${root}`);
    }
    return bytes;
  } catch (error) {
    if (process.platform === 'win32') return freeBytesViaPowerShell(root);
    fail(`fs.statfs falhou para ${root}: ${error.message}`);
  }
}

/**
 * Decide com valores já medidos. Exportado para testes; não mede nem escreve.
 */
export function evaluateBudget({
  freeBytes,
  targetBytes,
  baselineBytes,
  baselineApprox = false,
  uncertaintyGb = 0,
  floorGb = AUTHORIZED_FLOOR_GB,
  growthCapGb = AUTHORIZED_GROWTH_CAP_GB,
}) {
  for (const [label, value] of [
    ['freeBytes', freeBytes],
    ['targetBytes', targetBytes],
    ['baselineBytes', baselineBytes],
  ]) {
    if (!isFiniteNonNegative(value)) fail(`${label} inválido`);
  }
  if (!isFiniteNonNegative(uncertaintyGb)) fail('uncertaintyGb inválido');
  if (!Number.isFinite(floorGb) || floorGb < 0) fail('floorGb inválido');
  if (!Number.isFinite(growthCapGb) || growthCapGb < 0) fail('growthCapGb inválido');

  const uncertaintyBytes = baselineApprox ? uncertaintyGb * GIB : 0;
  const effectiveBaselineBytes = baselineBytes - uncertaintyBytes;
  if (effectiveBaselineBytes < 0) fail('baseline efetivo negativo');

  const freeGb = freeBytes / GIB;
  const targetGb = targetBytes / GIB;
  const growthBytes = Math.max(0, targetBytes - effectiveBaselineBytes);
  const growthGb = growthBytes / GIB;
  const reasons = [];
  if (freeGb < floorGb) {
    reasons.push(`freeGb ${round(freeGb)} abaixo do piso ${floorGb}`);
  }
  if (growthGb > growthCapGb) {
    reasons.push(`growthGb ${round(growthGb)} acima do teto ${growthCapGb}`);
  }
  const status = reasons.length ? 'blocked' : 'ok';
  return {
    status,
    exitCode: status === 'blocked' ? 2 : 0,
    freeGb,
    targetGb,
    baselineGb: baselineBytes / GIB,
    baselineEffectiveGb: effectiveBaselineBytes / GIB,
    baselineApprox,
    growthGb,
    floorGb,
    growthCapGb,
    reasons,
    freeBytes,
    targetBytes,
    baselineBytes,
    growthBytes,
  };
}

function parseArgs(argv) {
  const parsed = {
    check: false,
    help: false,
    baseline: null,
    target: null,
    floorGb: null,
    growthCapGb: null,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === '--check') {
      parsed.check = true;
    } else if (arg === '--help' || arg === '-h') {
      parsed.help = true;
    } else if (arg === '--baseline') {
      parsed.baseline = argv[index + 1];
      index += 1;
      if (parsed.baseline === undefined) fail('--baseline exige um valor');
    } else if (arg === '--target') {
      parsed.target = argv[index + 1];
      index += 1;
      if (parsed.target === undefined) fail('--target exige um valor');
    } else if (arg === '--floor-gb') {
      parsed.floorGb = parsePositiveNumber(argv[index + 1], '--floor-gb');
      index += 1;
    } else if (arg === '--growth-cap-gb') {
      parsed.growthCapGb = parsePositiveNumber(argv[index + 1], '--growth-cap-gb');
      index += 1;
    } else {
      fail(`argumento desconhecido: ${arg}`);
    }
  }
  return parsed;
}

/**
 * Executa a checagem sem escrever. `deps.freeBytes` e `deps.targetBytes` são
 * injeções de teste; a CLI pública não as expõe.
 */
export function runCli(argv, deps = {}) {
  try {
    const parsed = parseArgs(argv);
    if (parsed.help) return { code: 0, stdout: `${USAGE}\n`, stderr: '' };
    if (!parsed.check) fail('use --check');
    if (!parsed.baseline) fail('--baseline é obrigatório');

    const target = path.resolve(parsed.target ?? DEFAULT_TARGET);
    const baseline = parseBaseline(parsed.baseline, { targetPath: target });
    const policy = resolvePolicy(parsed.floorGb, parsed.growthCapGb, baseline);
    const freeBytes = deps.freeBytes ?? freeBytesForPath(target);
    const targetBytes = deps.targetBytes ?? measureTargetBytes(target).bytes;
    const result = evaluateBudget({
      freeBytes,
      targetBytes,
      baselineBytes: baseline.baselineBytes,
      baselineApprox: baseline.approx,
      uncertaintyGb: baseline.uncertaintyGb,
      floorGb: policy.floorGb,
      growthCapGb: policy.growthCapGb,
    });
    const report = {
      status: result.status,
      exitCode: result.exitCode,
      freeGb: round(result.freeGb),
      targetGb: round(result.targetGb),
      baselineGb: round(result.baselineGb),
      baselineApprox: result.baselineApprox,
      baselineSource: baseline.source,
      baselineEffectiveGb: round(result.baselineEffectiveGb),
      growthGb: round(result.growthGb),
      floorGb: result.floorGb,
      growthCapGb: result.growthCapGb,
      reasons: result.reasons,
      targetPath: target,
      measuredAt: new Date().toISOString(),
      freeBytes: result.freeBytes,
      targetBytes: result.targetBytes,
      baselineBytes: result.baselineBytes,
      growthBytes: result.growthBytes,
    };
    return { code: result.exitCode, stdout: `${JSON.stringify(report, null, 2)}\n`, stderr: '' };
  } catch (error) {
    const message = error instanceof BudgetError ? error.message : `erro inesperado: ${error.message}`;
    return { code: 1, stdout: '', stderr: `disk-budget: ${message}\n` };
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
  const result = runCli(process.argv.slice(2));
  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  process.exitCode = result.code;
}
