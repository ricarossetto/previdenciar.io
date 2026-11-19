import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import crypto from 'node:crypto';
import { spawn } from 'node:child_process';
import {
  POLICY,
  RunBudgetError,
  buildLedger,
  measureTree,
  parseArgs,
  runCli,
  runWithBudget,
  updateSegments,
  validateLedger,
} from './run-with-disk-budget.mjs';

const GIB = 1024 ** 3;
const MIB = 1024 ** 2;
const SAGA_ROOT = process.env.PREVIDENCIAR_SAGA_DIR
  ?? path.join(os.homedir(), '.sagas', 'previdenciar-fila-integral-20261102');
const SMALL_POLICY = {
  totalCapBytes: 128 * 1024,
  floorBytes: 1024,
  reserveBytes: 16 * 1024,
  initialChargeBytes: 4096,
  maxEntries: 100_000,
};
const FREE = 1 * 1024 * 1024;

function fixture() {
  const base = fs.mkdtempSync(path.join(SAGA_ROOT, 'tmp-orcamento-test-'));
  const repo = path.join(base, 'repo');
  const target = path.join(repo, 'target');
  const saga = path.join(base, 'saga');
  fs.mkdirSync(target, { recursive: true });
  fs.mkdirSync(path.join(repo, 'src'), { recursive: true });
  fs.mkdirSync(saga, { recursive: true });
  fs.writeFileSync(path.join(repo, 'src', 'tiny.txt'), 'x'.repeat(32));
  const baseline = path.join(saga, 'baseline.json');
  fs.writeFileSync(baseline, JSON.stringify({
    target_path: target,
    created_at: '2026-11-03T00:00:00.000Z',
    resumption_id: '2026-11-03',
    baseline_source: 'fixture sintetica',
    baseline_gb: 46.66,
    baseline_approx: true,
    baseline_uncertainty_gb: 0.01,
    floor_gb: 10,
    growth_cap_gb: 2,
  }, null, 2));
  return {
    base,
    repo,
    target,
    saga,
    baseline,
    ledger: path.join(saga, 'ledger.json'),
  };
}

function argvFor(f, { init = true, command = process.execPath, args = ['-e', 'process.exit(0)'], timeoutSeconds = 0 } = {}) {
  const argv = [
    '--ledger', f.ledger,
    '--baseline', f.baseline,
    '--root', `target=${f.target}`,
    '--root', `repo=${f.repo}`,
    '--root', `saga=${f.saga}`,
    '--cwd', f.repo,
  ];
  if (init) argv.push('--init-ledger');
  if (timeoutSeconds) argv.push('--timeout-seconds', String(timeoutSeconds));
  argv.push('--', command, ...args);
  return argv;
}

async function run(f, options = {}) {
  const parsed = parseArgs(argvFor(f, options));
  return runWithBudget(parsed, {
    policy: options.policy ?? SMALL_POLICY,
    freeBytes: options.freeBytes ?? FREE,
    rootSampleMs: options.rootSampleMs ?? 20,
    freeSampleMs: options.freeSampleMs ?? 10,
    measureTree: options.measureTree,
    volumeKey: options.volumeKey,
  });
}

function sentinelCommand(f, name = 'sentinel.txt') {
  return {
    command: process.execPath,
    args: ['-e', `require('fs').writeFileSync(${JSON.stringify(path.join(f.repo, name))}, 'x')`],
  };
}

test('sem ledger e sem init falha fechado e não executa o comando', async () => {
  const f = fixture();
  const sentinel = sentinelCommand(f);
  const parsed = parseArgs(argvFor(f, { init: false, ...sentinel }));
  const result = await runWithBudget(parsed, { policy: SMALL_POLICY, freeBytes: FREE });
  assert.equal(result.exitCode, 4);
  assert.equal(result.status, 'error');
  assert.equal(fs.existsSync(path.join(f.repo, 'sentinel.txt')), false);
});

test('ledger malformado falha fechado sem executar o comando', async () => {
  const f = fixture();
  fs.writeFileSync(f.ledger, '{ledger quebrado');
  const sentinel = sentinelCommand(f);
  const result = await run(f, { ...sentinel, init: false });
  assert.equal(result.exitCode, 4);
  assert.equal(result.status, 'error');
  assert.equal(fs.existsSync(path.join(f.repo, 'sentinel.txt')), false);
});

test('baseline inválido/forjado falha fechado sem executar o comando', async () => {
  const f = fixture();
  const baseline = JSON.parse(fs.readFileSync(f.baseline, 'utf8'));
  fs.writeFileSync(f.baseline, JSON.stringify({ ...baseline, baseline_approx: false }));
  const sentinel = sentinelCommand(f);
  const result = await run(f, { ...sentinel });
  assert.equal(result.exitCode, 4);
  assert.equal(fs.existsSync(path.join(f.repo, 'sentinel.txt')), false);
});

test('ledger existente com cap forjado falha fechado e preserva o lock para revisão', async () => {
  const f = fixture();
  const first = await run(f);
  assert.equal(first.exitCode, 0);
  const ledger = JSON.parse(fs.readFileSync(f.ledger, 'utf8'));
  ledger.total_cap_bytes = 4 * GIB;
  fs.writeFileSync(f.ledger, JSON.stringify(ledger, null, 2));
  const sentinel = sentinelCommand(f);
  const result = await run(f, { ...sentinel, init: false });
  assert.equal(result.exitCode, 4);
  assert.equal(fs.existsSync(path.join(f.repo, 'sentinel.txt')), false);
  assert.equal(fs.existsSync(`${f.ledger}.lock`), true);
});

test('piso livre bloqueia antes de criar o filho', async () => {
  const f = fixture();
  const sentinel = sentinelCommand(f);
  const result = await run(f, { ...sentinel, freeBytes: 0 });
  assert.equal(result.exitCode, 3);
  assert.equal(result.status, 'blocked');
  assert.equal(fs.existsSync(path.join(f.repo, 'sentinel.txt')), false);
  const ledger = JSON.parse(fs.readFileSync(f.ledger, 'utf8'));
  assert.equal(ledger.status, 'blocked');
  assert.equal(ledger.last_run.reason, 'floor');
});

test('crescimento acima do cap bloqueia antes do próximo comando', async () => {
  const f = fixture();
  const first = await run(f);
  assert.equal(first.exitCode, 0);
  fs.writeFileSync(path.join(f.target, 'large.bin'), Buffer.alloc(200 * 1024));
  const sentinel = sentinelCommand(f);
  const result = await run(f, { ...sentinel, init: false });
  assert.equal(result.exitCode, 3);
  assert.equal(fs.existsSync(path.join(f.repo, 'sentinel.txt')), false);
  const ledger = JSON.parse(fs.readFileSync(f.ledger, 'utf8'));
  assert.equal(ledger.status, 'blocked');
  assert.equal(ledger.last_run.reason, 'cap');
});

test('target dentro do repo não é contado duas vezes', async () => {
  const f = fixture();
  const first = await run(f);
  assert.equal(first.exitCode, 0);
  fs.writeFileSync(path.join(f.repo, 'src', 'tiny.txt'), 'y'.repeat(2048));
  fs.writeFileSync(path.join(f.target, 'artifact.bin'), Buffer.alloc(4096));
  const second = await run(f, { init: false });
  assert.equal(second.exitCode, 0);
  const ledger = JSON.parse(fs.readFileSync(f.ledger, 'utf8'));
  const target = ledger.segments.find((segment) => segment.name === 'target');
  const repo = ledger.segments.find((segment) => segment.name === 'repo');
  assert.equal(target.charged_bytes, 4096);
  assert.equal(repo.charged_bytes, 2048 - 32);
});

test('monitor aborta o próprio filho e não mata um processo externo', async () => {
  const f = fixture();
  const foreign = spawn(process.execPath, ['-e', 'setTimeout(() => {}, 5000)'], {
    stdio: 'ignore',
    windowsHide: true,
  });
  await new Promise((resolve) => setTimeout(resolve, 100));
  const file = path.join(f.target, 'during.bin');
  const script = [
    "const fs=require('fs');",
    'const file=process.argv[1];',
    'let n=0;',
    "const timer=setInterval(()=>{fs.appendFileSync(file,'x'.repeat(8192)); if(++n>100) clearInterval(timer);},20);",
  ].join('');
  const result = await run(f, {
    command: process.execPath,
    args: ['-e', script, file],
    policy: { ...SMALL_POLICY, totalCapBytes: 64 * 1024, reserveBytes: 16 * 1024 },
    rootSampleMs: 5,
    freeSampleMs: 5,
  });
  assert.equal(result.status, 'blocked');
  assert.equal(result.exitCode, 3);
  assert.equal(foreign.exitCode, null);
  foreign.kill();
  await new Promise((resolve) => foreign.once('exit', resolve));
  const ledger = JSON.parse(fs.readFileSync(f.ledger, 'utf8'));
  assert.equal(ledger.status, 'blocked');
});

test('timeout encerra o próprio filho e devolve bloqueio', async () => {
  const f = fixture();
  const result = await run(f, {
    command: process.execPath,
    args: ['-e', 'setTimeout(() => {}, 5000)'],
    timeoutSeconds: 1,
  });
  assert.equal(result.status, 'blocked');
  assert.equal(result.exitCode, 3);
  const ledger = JSON.parse(fs.readFileSync(f.ledger, 'utf8'));
  assert.equal(ledger.last_run.reason, 'timeout');
});

test('código não zero do filho é preservado e o ledger não guarda argv', async () => {
  const f = fixture();
  const marker = 'segredo-nao-deve-aparecer-no-ledger';
  const result = await run(f, {
    command: process.execPath,
    args: ['-e', 'process.exit(7)', marker],
  });
  assert.equal(result.exitCode, 7);
  assert.equal(result.status, 'command-failed');
  const raw = fs.readFileSync(f.ledger, 'utf8');
  assert.equal(raw.includes(marker), false);
  assert.equal(raw.includes('process.exit'), false);
  const ledger = JSON.parse(raw);
  assert.equal(ledger.last_run.child_exit_code, 7);
  assert.equal(ledger.status, 'command-failed');
});

test('execuções seriais preservam saldo, created_at e cap', async () => {
  const f = fixture();
  const first = await run(f);
  assert.equal(first.exitCode, 0);
  const before = JSON.parse(fs.readFileSync(f.ledger, 'utf8'));
  fs.writeFileSync(path.join(f.target, 'growth.bin'), Buffer.alloc(4096));
  const second = await run(f, { init: false });
  assert.equal(second.exitCode, 0);
  const after = JSON.parse(fs.readFileSync(f.ledger, 'utf8'));
  assert.equal(after.created_at, before.created_at);
  assert.equal(after.total_cap_bytes, SMALL_POLICY.totalCapBytes);
  assert.ok(after.used_bytes >= 4096);
  assert.ok(after.remaining_bytes <= SMALL_POLICY.totalCapBytes - 4096);
  const third = await run(f, { init: false });
  assert.equal(third.exitCode, 0);
  const final = JSON.parse(fs.readFileSync(f.ledger, 'utf8'));
  assert.ok(final.used_bytes >= after.used_bytes);
  assert.equal(final.created_at, before.created_at);
  assert.equal(final.total_cap_bytes, SMALL_POLICY.totalCapBytes);
});

test('soma de duas raízes e diminuição externa não creditam exclusão', () => {
  const config = {
    ledger: path.join('C:', 'fake-ledger.json'),
    repoRoot: path.join('C:', 'repo'),
    targetRoot: path.join('C:', 'repo', 'target'),
    roots: [
      { name: 'target', path: path.join('C:', 'repo', 'target') },
      { name: 'repo', path: path.join('C:', 'repo') },
    ],
    caches: [],
  };
  const baseline = {
    path: path.join('C:', 'baseline.json'),
    hash: 'a'.repeat(64),
    bytes: 50 * GIB,
    approx: true,
    uncertaintyBytes: 10 * MIB,
    effectiveBytes: 50 * GIB - 10 * MIB,
  };
  const measured = new Map([
    ['target', { bytes: 1000, path: config.targetRoot }],
    ['repo', { bytes: 1000, path: config.repoRoot }],
  ]);
  const ledger = buildLedger({ config, baseline, measured, policy: SMALL_POLICY });
  updateSegments(ledger, new Map([
    ['target', { bytes: 1500, path: config.targetRoot }],
    ['repo', { bytes: 2000, path: config.repoRoot }],
  ]), { policy: SMALL_POLICY });
  assert.equal(ledger.used_bytes, SMALL_POLICY.initialChargeBytes + 1500);
  const high = ledger.used_bytes;
  updateSegments(ledger, new Map([
    ['target', { bytes: 500, path: config.targetRoot }],
    ['repo', { bytes: 500, path: config.repoRoot }],
  ]), { policy: SMALL_POLICY });
  assert.equal(ledger.used_bytes, high);
  assert.ok(ledger.external_decreases.length >= 1);
  assert.equal(ledger.total_cap_bytes, SMALL_POLICY.totalCapBytes);
  assert.equal(ledger.remaining_bytes, SMALL_POLICY.totalCapBytes - high);
});

test('re crescimento após queda é cobrado cumulativamente: 100→120→80→100 = 40', () => {
  const config = {
    ledger: path.join('C:', 'fake-ledger.json'),
    repoRoot: path.join('C:', 'repo'),
    targetRoot: path.join('C:', 'repo', 'target'),
    roots: [{ name: 'target', path: path.join('C:', 'repo', 'target') }],
    caches: [],
  };
  const baseline = {
    path: path.join('C:', 'baseline.json'),
    hash: 'a'.repeat(64),
    bytes: 50 * GIB,
    approx: true,
    uncertaintyBytes: 10 * MIB,
    effectiveBytes: 50 * GIB - 10 * MIB,
  };
  const measured = new Map([['target', { bytes: 100, path: config.targetRoot }]]);
  const ledger = buildLedger({ config, baseline, measured, policy: SMALL_POLICY });
  for (const bytes of [120, 80, 100]) {
    updateSegments(ledger, new Map([['target', { bytes, path: config.targetRoot }]]), { policy: SMALL_POLICY });
  }
  assert.equal(ledger.used_bytes, SMALL_POLICY.initialChargeBytes + 40);
  assert.equal(ledger.segments[0].charged_bytes, 40);
  assert.equal(ledger.segments[0].high_water_bytes, 120);
});

test('uma raiz que diminui não cancela outra que cresce', () => {
  const config = {
    ledger: path.join('C:', 'fake-ledger.json'),
    repoRoot: path.join('C:', 'repo'),
    targetRoot: path.join('C:', 'repo', 'target'),
    roots: [
      { name: 'target', path: path.join('C:', 'repo', 'target') },
      { name: 'repo', path: path.join('C:', 'repo') },
    ],
    caches: [],
  };
  const baseline = {
    path: path.join('C:', 'baseline.json'),
    hash: 'a'.repeat(64),
    bytes: 50 * GIB,
    approx: true,
    uncertaintyBytes: 10 * MIB,
    effectiveBytes: 50 * GIB - 10 * MIB,
  };
  const measured = new Map([
    ['target', { bytes: 1000, path: config.targetRoot }],
    ['repo', { bytes: 1000, path: config.repoRoot }],
  ]);
  const ledger = buildLedger({ config, baseline, measured, policy: SMALL_POLICY });
  updateSegments(ledger, new Map([
    ['target', { bytes: 600, path: config.targetRoot }],
    ['repo', { bytes: 1200, path: config.repoRoot }],
  ]), { policy: SMALL_POLICY });
  assert.equal(ledger.segments[0].charged_bytes, 0);
  assert.equal(ledger.segments[1].charged_bytes, 200);
  updateSegments(ledger, new Map([
    ['target', { bytes: 1000, path: config.targetRoot }],
    ['repo', { bytes: 1200, path: config.repoRoot }],
  ]), { policy: SMALL_POLICY });
  assert.equal(ledger.used_bytes, SMALL_POLICY.initialChargeBytes + 600);
});

test('medida tolera filho que desaparece no rename, mas bloqueia raiz perdida', () => {
  const root = path.resolve('C:', 'fake');
  const fake = {
    lstatSync: (value) => {
      if (value === root) return { isSymbolicLink: () => false, isDirectory: () => true };
      const error = new Error('ENOENT');
      error.code = 'ENOENT';
      throw error;
    },
    realpathSync: { native: () => root },
    readdirSync: () => [{ name: 'gone' }],
  };
  const result = measureTree(root, { io: fake });
  assert.equal(result.exists, true);
  assert.equal(result.bytes, 0);
  assert.equal(result.transientSkips, 1);

  const rootLost = {
    lstatSync: () => ({ isSymbolicLink: () => false, isDirectory: () => true }),
    realpathSync: { native: () => root },
    readdirSync: () => {
      const error = new Error('ENOENT');
      error.code = 'ENOENT';
      throw error;
    },
  };
  assert.throws(() => measureTree(root, { io: rootLost }), /raiz desapareceu/);
});

test('raiz obrigatória ausente bloqueia antes do filho', async () => {
  const f = fixture();
  const sentinel = sentinelCommand(f);
  const parsed = parseArgs(argvFor(f, sentinel));
  const result = await runWithBudget(parsed, {
    policy: SMALL_POLICY,
    freeBytes: FREE,
    measureTree: (value, opts) => {
      if (value === f.target) return { exists: false, bytes: 0, entries: 0, path: value };
      return measureTree(value, opts);
    },
  });
  assert.equal(result.exitCode, 4);
  assert.equal(fs.existsSync(path.join(f.repo, 'sentinel.txt')), false);
});

test('volume da saga sem piso bloqueia preflight mesmo com target/repo livres', async () => {
  const f = fixture();
  const sentinel = sentinelCommand(f);
  const result = await run(f, {
    ...sentinel,
    freeBytes: (value) => value.includes(f.saga) ? 0 : FREE,
    volumeKey: (value) => value.includes(f.saga) ? 'saga-volume' : 'main-volume',
  });
  assert.equal(result.exitCode, 3);
  assert.equal(result.status, 'blocked');
  assert.equal(fs.existsSync(path.join(f.repo, 'sentinel.txt')), false);
  const ledger = JSON.parse(fs.readFileSync(f.ledger, 'utf8'));
  assert.equal(ledger.last_run.reason, 'floor');
});

test('--init-ledger existente não sobrescreve nem cria novo cap', async () => {
  const f = fixture();
  const first = await run(f);
  assert.equal(first.exitCode, 0);
  const before = fs.readFileSync(f.ledger);
  const sentinel = sentinelCommand(f);
  const result = await run(f, { ...sentinel, init: true });
  assert.equal(result.exitCode, 4);
  assert.equal(fs.existsSync(path.join(f.repo, 'sentinel.txt')), false);
  assert.equal(Buffer.compare(before, fs.readFileSync(f.ledger)), 0);
});

test('migração v1 carrega uso anterior e cobra inicial sem novo cap', async () => {
  const f = fixture();
  const first = await run(f);
  assert.equal(first.exitCode, 0);
  const v2 = JSON.parse(fs.readFileSync(f.ledger, 'utf8'));
  const legacy = {
    ...v2,
    schema: 'previdenciar-run-disk-budget/v1',
    used_bytes: v2.segments.reduce((sum, segment) => sum + segment.charged_bytes, 0),
    remaining_bytes: v2.total_cap_bytes - v2.segments.reduce((sum, segment) => sum + segment.charged_bytes, 0),
    segments: v2.segments.map((segment) => ({
      ...segment,
      charged_bytes: segment.high_water_bytes - segment.start_bytes,
    })),
    status: 'ok',
  };
  delete legacy.initial_charge_bytes;
  delete legacy.initial_charge_at;
  delete legacy.initial_charge_basis;
  delete legacy.previous;
  delete legacy.reconciliation;
  const oldUsed = legacy.used_bytes;
  fs.writeFileSync(f.ledger, JSON.stringify(legacy, null, 2));
  const oldHash = crypto.createHash('sha256').update(fs.readFileSync(f.ledger)).digest('hex');
  const argv = [
    '--ledger', f.ledger,
    '--baseline', f.baseline,
    '--root', `target=${f.target}`,
    '--root', `repo=${f.repo}`,
    '--root', `saga=${f.saga}`,
    '--cwd', f.repo,
    '--reconcile-initial',
  ];
  const result = await runWithBudget(parseArgs(argv), { policy: SMALL_POLICY, freeBytes: FREE });
  assert.equal(result.exitCode, 0);
  const migrated = JSON.parse(fs.readFileSync(f.ledger, 'utf8'));
  assert.equal(migrated.schema, 'previdenciar-run-disk-budget/v2');
  assert.equal(migrated.initial_charge_bytes, SMALL_POLICY.initialChargeBytes);
  assert.equal(migrated.used_bytes, oldUsed + SMALL_POLICY.initialChargeBytes);
  assert.equal(migrated.total_cap_bytes, SMALL_POLICY.totalCapBytes);
  assert.equal(migrated.previous.ledger_sha256, oldHash);
  assert.equal(migrated.previous.used_bytes, oldUsed);
  assert.ok(migrated.reconciliation);
});

test('validateLedger recusa cap, timestamp e segmento forjados', () => {
  const f = fixture();
  const parsed = parseArgs(argvFor(f));
  const config = {
    ledger: parsed.ledger,
    repoRoot: f.repo,
    targetRoot: f.target,
    roots: parsed.roots,
    caches: [],
  };
  const baseline = {
    path: f.baseline,
    hash: 'b'.repeat(64),
    bytes: 50 * GIB,
    approx: true,
    uncertaintyBytes: 10 * MIB,
    effectiveBytes: 50 * GIB - 10 * MIB,
  };
  const measured = new Map([
    ['target', { bytes: 0, path: f.target }],
    ['repo', { bytes: 0, path: f.repo }],
    ['saga', { bytes: 0, path: f.saga }],
  ]);
  const ledger = buildLedger({ config, baseline, measured, policy: SMALL_POLICY });
  assert.throws(() => validateLedger({ ...ledger, total_cap_bytes: 4 * GIB }, {
    config, baseline, policy: SMALL_POLICY,
  }), /cap/);
  assert.throws(() => validateLedger({ ...ledger, updated_at: '2000-01-01T00:00:00.000Z' }, {
    config, baseline, policy: SMALL_POLICY,
  }), /timestamps/);
  assert.throws(() => validateLedger({
    ...ledger,
    segments: [{ ...ledger.segments[0], charged_bytes: 1 }],
  }, { config, baseline, policy: SMALL_POLICY }), /conjunto|segmento|cobrança/);
});

test('measureTree recusa symlink/reparsepoint e makeConfig recusa path duplicado', () => {
  const io = {
    lstatSync: () => ({ isSymbolicLink: () => true, isDirectory: () => false }),
    realpathSync: { native: () => path.resolve('C:', 'fake') },
    readdirSync: () => [],
  };
  assert.throws(() => measureTree(path.resolve('C:', 'fake'), { io }), /symlink/);
  const f = fixture();
  assert.throws(() => parseArgs([
    '--ledger', f.ledger,
    '--baseline', f.baseline,
    '--root', `target=${f.target}`,
    '--root', `repo=${f.repo}`,
    '--root', `saga=${f.saga}`,
    '--cache', `target=${f.target}`,
    '--', process.execPath, '-e', 'process.exit(0)',
  ]), /duplicad/);
});

test('erro de medição e lock existente falham sem sucesso aparente', async () => {
  const f = fixture();
  const parsed = parseArgs(argvFor(f));
  const result = await runWithBudget(parsed, {
    policy: SMALL_POLICY,
    freeBytes: FREE,
    measureTree: () => { throw new RunBudgetError('measurement', 'falha injetada'); },
  });
  assert.equal(result.exitCode, 4);
  assert.equal(result.status, 'error');
  const lock = `${f.ledger}.lock`;
  fs.writeFileSync(lock, 'lock');
  const blocked = await runWithBudget(parseArgs(argvFor(f, { init: false })), {
    policy: SMALL_POLICY,
    freeBytes: FREE,
  });
  assert.equal(blocked.exitCode, 4);
  assert.equal(blocked.status, 'error');
});

test('CLI recusa argumento desconhecido, repetido, timeout zero e flag de afrouxamento', async () => {
  assert.equal((await runCli(['--check'])).code, 4);
  assert.equal((await runCli(['--ledger', 'x'])).code, 4);
  assert.equal((await runCli(['--growth-cap-gb', '4'])).code, 4);
  assert.equal((await runCli(['--ledger', 'a', '--ledger', 'b'])).code, 4);
  assert.equal((await runCli(['--timeout-seconds', '0'])).code, 4);
});
