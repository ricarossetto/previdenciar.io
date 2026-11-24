import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import {
  BudgetError,
  evaluateBudget,
  parseBaselineObject,
  runCli,
  measureTargetBytes,
  freeBytesForPath,
  capacityBytesForPath,
} from './disk-budget.mjs';

const GIB = 1024 ** 3;
const BASE_GB = 46.66;
const baseBytes = BASE_GB * GIB;
const targetPath = path.resolve('target');

function near(actual, expected, tolerance = 1e-9) {
  assert.ok(Math.abs(actual - expected) <= tolerance, `${actual} != ${expected}`);
}

test('9.9 GiB livres bloqueia com código 2', () => {
  const result = evaluateBudget({
    freeBytes: 9.9 * GIB,
    targetBytes: baseBytes,
    baselineBytes: baseBytes,
    floorGb: 10,
    growthCapGb: 2,
  });
  assert.equal(result.status, 'blocked');
  assert.equal(result.exitCode, 2);
  assert.match(result.reasons.join(' '), /freeGb/);
});

test('10.1 GiB livres e 1.9 GiB de crescimento libera com código 0', () => {
  const result = evaluateBudget({
    freeBytes: 10.1 * GIB,
    targetBytes: baseBytes + 1.9 * GIB,
    baselineBytes: baseBytes,
    floorGb: 10,
    growthCapGb: 2,
  });
  assert.equal(result.status, 'ok');
  assert.equal(result.exitCode, 0);
  near(result.growthGb, 1.9);
});

test('2.1 GiB de crescimento bloqueia com código 2', () => {
  const result = evaluateBudget({
    freeBytes: 20 * GIB,
    targetBytes: baseBytes + 2.1 * GIB,
    baselineBytes: baseBytes,
    floorGb: 10,
    growthCapGb: 2,
  });
  assert.equal(result.status, 'blocked');
  assert.equal(result.exitCode, 2);
  assert.match(result.reasons.join(' '), /growthGb/);
});

test('fronteiras: exatamente 10 GiB e 2 GiB são permitidos; acima bloqueia', () => {
  const atLimit = evaluateBudget({
    freeBytes: 10 * GIB,
    targetBytes: baseBytes + 2 * GIB,
    baselineBytes: baseBytes,
    floorGb: 10,
    growthCapGb: 2,
  });
  assert.equal(atLimit.status, 'ok');
  assert.equal(atLimit.exitCode, 0);

  const belowFree = evaluateBudget({
    freeBytes: 10 * GIB - 1,
    targetBytes: baseBytes,
    baselineBytes: baseBytes,
    floorGb: 10,
    growthCapGb: 2,
  });
  assert.equal(belowFree.status, 'blocked');

  const aboveGrowth = evaluateBudget({
    freeBytes: 20 * GIB,
    targetBytes: baseBytes + 2 * GIB + 1,
    baselineBytes: baseBytes,
    floorGb: 10,
    growthCapGb: 2,
  });
  assert.equal(aboveGrowth.status, 'blocked');
});

test('baseline aproximado usa incerteza conservadora', () => {
  const ok = evaluateBudget({
    freeBytes: 20 * GIB,
    targetBytes: baseBytes + 1.98 * GIB,
    baselineBytes: baseBytes,
    baselineApprox: true,
    uncertaintyGb: 0.01,
    floorGb: 10,
    growthCapGb: 2,
  });
  assert.equal(ok.status, 'ok');
  near(ok.growthGb, 1.99);

  const blocked = evaluateBudget({
    freeBytes: 20 * GIB,
    targetBytes: baseBytes + 2.0 * GIB,
    baselineBytes: baseBytes,
    baselineApprox: true,
    uncertaintyGb: 0.01,
    floorGb: 10,
    growthCapGb: 2,
  });
  assert.equal(blocked.status, 'blocked');
  near(blocked.growthGb, 2.01);
});

test('valores inválidos e incerteza acima de 1 GiB falham fechado', () => {
  assert.throws(() => evaluateBudget({ freeBytes: Number.NaN, targetBytes: 1, baselineBytes: 1 }), BudgetError);
  assert.throws(() => evaluateBudget({ freeBytes: 1, targetBytes: -1, baselineBytes: 1 }), BudgetError);
  assert.throws(() => evaluateBudget({ freeBytes: 1, targetBytes: 1, baselineBytes: -1 }), BudgetError);
  assert.throws(() => evaluateBudget({
    freeBytes: 20 * GIB,
    targetBytes: 1,
    baselineBytes: 1,
    baselineApprox: true,
    uncertaintyGb: 1.1,
  }), /incerteza|uncertaintyGb/);
});

test('baseline exato e aproximado são validados', () => {
  const exact = parseBaselineObject({
    target_path: targetPath,
    created_at: '2026-11-02T00:00:00Z',
    resumption_id: '2026-11-02',
    baseline_source: 'teste exato',
    baseline_bytes: 50 * GIB,
    floor_gb: 10,
    growth_cap_gb: 2,
  }, { targetPath });
  near(exact.baselineGb, 50);
  assert.equal(exact.approx, false);

  const approx = parseBaselineObject({
    target_path: targetPath,
    created_at: '2026-11-02T00:00:00Z',
    resumption_id: '2026-11-02',
    baseline_source: 'teste aproximado',
    baseline_gb: 46.66,
    baseline_approx: true,
    baseline_uncertainty_gb: 0.01,
    floor_gb: 10,
    growth_cap_gb: 2,
  }, { targetPath });
  assert.equal(approx.approx, true);
  near(approx.uncertaintyGb, 0.01);
});

test('baseline impossível, inconsistente ou com política fraca falha fechado', () => {
  const base = {
    target_path: targetPath,
    created_at: '2026-11-02T00:00:00Z',
    resumption_id: '2026-11-02',
    baseline_source: 'teste',
    baseline_gb: 46.66,
    floor_gb: 10,
    growth_cap_gb: 2,
  };
  assert.throws(() => parseBaselineObject(base, { targetPath }), /baseline_approx/);
  assert.throws(() => parseBaselineObject({ ...base, baseline_approx: true, floor_gb: 9 }, { targetPath }), /piso/);
  assert.throws(() => parseBaselineObject({ ...base, baseline_approx: true, growth_cap_gb: 2.5 }, { targetPath }), /teto/);
  assert.throws(() => parseBaselineObject({ ...base, baseline_approx: true, target_path: path.resolve('outro') }, { targetPath }), /target_path/);
  assert.throws(() => parseBaselineObject({
    ...base,
    baseline_approx: true,
    baseline_bytes: 1 * GIB,
    baseline_gb: 2,
  }, { targetPath }), /divergem/);
});

test('CLI: erros de configuração retornam 1 e não aprovam', () => {
  assert.equal(runCli([]).code, 1);
  assert.equal(runCli(['--check']).code, 1);
  assert.equal(runCli(['--check', '--baseline', 'caminho-inexistente.json']).code, 1);
  assert.equal(runCli(['--check', '--baseline', '46.66', '--floor-gb', '9'], {
    freeBytes: 20 * GIB,
    targetBytes: 1,
  }).code, 1);
  assert.equal(runCli(['--check', '--baseline', '46.66', '--growth-cap-gb', '2.5'], {
    freeBytes: 20 * GIB,
    targetBytes: 1,
  }).code, 1);
});

test('CLI: saída JSON e códigos 0/2 com medições injetadas', () => {
  const ok = runCli(['--check', '--baseline', '46.66'], {
    freeBytes: 20 * GIB,
    capacityBytes: 100 * GIB,
    targetBytes: baseBytes,
  });
  assert.equal(ok.code, 0);
  const okReport = JSON.parse(ok.stdout);
  assert.equal(okReport.status, 'ok');
  assert.equal(okReport.exitCode, 0);
  assert.ok(Object.hasOwn(okReport, 'freeGb'));
  assert.ok(Object.hasOwn(okReport, 'targetGb'));
  assert.ok(Object.hasOwn(okReport, 'growthGb'));
  assert.equal(okReport.baselineCreatedAt, 'não informado (CLI)');
  assert.equal(okReport.baselineResumptionId, 'não informado (CLI)');
  assert.ok(!Number.isNaN(Date.parse(okReport.measuredAt)));

  const low = runCli(['--check', '--baseline', '46.66'], {
    freeBytes: 9.9 * GIB,
    capacityBytes: 100 * GIB,
    targetBytes: baseBytes,
  });
  assert.equal(low.code, 2);
  assert.equal(JSON.parse(low.stdout).status, 'blocked');

  const growth = runCli(['--check', '--baseline', '46.66'], {
    freeBytes: 20 * GIB,
    capacityBytes: 100 * GIB,
    targetBytes: baseBytes + 2.1 * GIB,
  });
  assert.equal(growth.code, 2);
  assert.equal(JSON.parse(growth.stdout).status, 'blocked');
});

test('CLI: baseline numérico absurdo e acima do target falham com código 1', () => {
  const absurd = runCli(['--check', '--baseline', '999'], {
    freeBytes: 20 * GIB,
    capacityBytes: 100 * GIB,
    targetBytes: 1,
  });
  assert.equal(absurd.code, 1);
  assert.match(absurd.stderr, /capacidade/);

  const aboveTarget = runCli(['--check', '--baseline', '47'], {
    freeBytes: 20 * GIB,
    capacityBytes: 100 * GIB,
    targetBytes: baseBytes,
  });
  assert.equal(aboveTarget.code, 1);
  assert.match(aboveTarget.stderr, /acima do target/);

  const withinUncertainty = runCli(['--check', '--baseline', '46.66'], {
    freeBytes: 20 * GIB,
    capacityBytes: 100 * GIB,
    targetBytes: 46.655 * GIB,
  });
  assert.equal(withinUncertainty.code, 0);

  const beyondUncertainty = runCli(['--check', '--baseline', '46.66'], {
    freeBytes: 20 * GIB,
    capacityBytes: 100 * GIB,
    targetBytes: 46.64 * GIB,
  });
  assert.equal(beyondUncertainty.code, 1);
});

test('CLI: argumento desconhecido e --target explícito', () => {
  assert.equal(runCli(['--check', '--baseline', '46.66', '--bogus'], {
    freeBytes: 20 * GIB,
    capacityBytes: 100 * GIB,
    targetBytes: baseBytes,
  }).code, 1);

  const explicit = runCli([
    '--check',
    '--baseline',
    '46.66',
    '--target',
    path.join(process.cwd(), 'scripts'),
  ], {
    freeBytes: 20 * GIB,
    capacityBytes: 100 * GIB,
    targetBytes: baseBytes,
  });
  assert.equal(explicit.code, 0);
  assert.equal(
    JSON.parse(explicit.stdout).targetPath,
    path.resolve('scripts'),
  );
});

test('medição real do diretório scripts é positiva e não dispara build', () => {
  const measured = measureTargetBytes(path.join(process.cwd(), 'scripts'));
  assert.ok(measured.bytes > 0);
  assert.ok(measured.entries > 0);
  assert.equal(measured.root, path.resolve('scripts'));
  assert.ok(freeBytesForPath(process.cwd()) > 0);
  assert.ok(capacityBytesForPath(process.cwd()) >= freeBytesForPath(process.cwd()));
});

test('target ausente ou arquivo comum falha fechado', () => {
  assert.throws(
    () => measureTargetBytes(path.join(process.cwd(), 'target-inexistente-disk-budget')),
    /não pôde ser medido/,
  );
  assert.throws(
    () => measureTargetBytes(path.join(process.cwd(), 'scripts', 'disk-budget.mjs')),
    /não é diretório/,
  );
});

test('link/reparsepoint e permissão são falhas fechadas (injeção, não teste real)', () => {
  const linkIo = {
    lstatSync: () => ({ isSymbolicLink: () => true, isDirectory: () => false }),
    realpathSync: { native: () => 'C:\\fake' },
    readdirSync: () => [],
  };
  assert.throws(() => measureTargetBytes('C:\\fake', linkIo), /symlink\/reparsepoint/);

  const permissionIo = {
    lstatSync: () => ({ isSymbolicLink: () => false, isDirectory: () => true }),
    realpathSync: { native: () => 'C:\\fake' },
    readdirSync: () => {
      const error = new Error('EACCES: permission denied');
      error.code = 'EACCES';
      throw error;
    },
  };
  assert.throws(() => measureTargetBytes('C:\\fake', permissionIo), /falha ao ler/);
});
