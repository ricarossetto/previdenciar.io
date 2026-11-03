import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  CAP_BYTES,
  FLOOR_BYTES,
  GIB,
  RESERVE_BYTES,
  accumulate,
  accumulateFast,
  accumulateSlow,
  cancelOwnChild,
  checkLimits,
  evaluateSample,
  loadOrCreateLedger,
  measureRoots,
  measureSlowDefault,
  persistLedger,
  runSupervisor,
  startHeadroomIssue,
} from './m1-budget-supervisor.mjs';

function fixture(capBytes) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'tmp-m1-budget-'));
  const repo = path.join(root, 'repo');
  const target = path.join(repo, 'target');
  const saga = path.join(root, 'saga');
  const temp = path.join(saga, 'tmp-compilar-integracao-20261103');
  const vhd = path.join(root, 'docker_data.vhdx');
  const oldLedger = path.join(root, 'old-ledger.json');
  fs.mkdirSync(target, { recursive: true });
  fs.mkdirSync(temp, { recursive: true });
  fs.writeFileSync(vhd, Buffer.alloc(1024));
  fs.writeFileSync(oldLedger, '{"schema":"old","used":1359538780}\n');
  const baseline = {
    schema: 'm1-budget-baseline/v1',
    authorization_id: 'AUTH-TEST',
    authorized_at: '2026-11-03T00:00:00.000Z',
    old_ledger_path: oldLedger,
    roots: [
      { name: 'repo', path: repo, start_bytes: 1000 },
      { name: 'target', path: target, start_bytes: 2000 },
      { name: 'saga', path: saga, start_bytes: 3000 },
      { name: 'temp', path: temp, start_bytes: 4000 },
    ],
    vhd: [{ name: 'docker_data', path: vhd, start_bytes: 5000 }],
    pagefile: { start_alloc_bytes: 6000 },
  };
  if (capBytes !== undefined) baseline.cap_bytes = capBytes;
  const baselinePath = path.join(root, 'baseline.json');
  fs.writeFileSync(baselinePath, JSON.stringify(baseline, null, 2), 'utf8');
  return { root, baseline, baselinePath, ledger: path.join(root, 'ledger.json'), oldLedger };
}

function sample(start, free = 100 * GIB) {
  return {
    free,
    roots: start.roots.map((root) => ({ ...root, bytes: root.start_bytes })),
    vhd: start.vhd.map((item) => ({ ...item, bytes: item.start_bytes })),
    pagefile_alloc_bytes: start.pagefile.start_alloc_bytes,
  };
}

function fakeChild(pid = 777) {
  const listeners = { exit: [], error: [] };
  const child = {
    pid,
    exitCode: null,
    signalCode: null,
    killed: false,
    once(event, callback) {
      (listeners[event] ??= []).push(callback);
      return child;
    },
    emit(event, ...args) {
      for (const callback of listeners[event] ?? []) callback(...args);
      return child;
    },
    kill() {
      if (child.exitCode !== null) return;
      child.killed = true;
      child.exitCode = 1;
      child.emit('exit', 1, null);
    },
  };
  return child;
}

function fakeClock() {
  const intervals = new Map();
  const timeouts = new Map();
  let nextId = 1;
  return {
    setInterval(callback) { const id = nextId++; intervals.set(id, callback); return id; },
    clearInterval(id) { intervals.delete(id); },
    setTimeout(callback) { const id = nextId++; timeouts.set(id, callback); return id; },
    clearTimeout(id) { timeouts.delete(id); },
    tick() { for (const callback of [...intervals.values()]) callback(); },
    intervalCount() { return intervals.size; },
    timeoutCount() { return timeouts.size; },
  };
}

function ledgerWithRoots(start) {
  return {
    roots: Object.fromEntries(start.roots.map((root) => [root.name, { last_observed_bytes: root.start_bytes, high_water_bytes: root.start_bytes, charged_bytes: 0 }])),
    vhd: Object.fromEntries(start.vhd.map((item) => [item.name, { last_observed_bytes: item.start_bytes, high_water_bytes: item.start_bytes, charged_bytes: 0 }])),
    pagefile: { last_observed_bytes: start.pagefile.start_alloc_bytes, high_water_bytes: start.pagefile.start_alloc_bytes, charged_bytes: 0 },
    charged_bytes: 0,
    updated_at: '',
  };
}

test('aumento queda aumento cobra pelo último observado, sem crédito', () => {
  const ledger = {
    roots: { repo: { start_bytes: 100, last_observed_bytes: 100, high_water_bytes: 100, charged_bytes: 0 } },
    vhd: { docker_data: { start_bytes: 1000, last_observed_bytes: 1000, high_water_bytes: 1000, charged_bytes: 0 } },
    pagefile: { start_bytes: 1000, last_observed_bytes: 1000, high_water_bytes: 1000, charged_bytes: 0 },
    charged_bytes: 0,
    updated_at: '',
  };
  const at = (value) => ({
    roots: [{ name: 'repo', bytes: value }],
    vhd: [{ name: 'docker_data', bytes: 1000 }],
    pagefile_alloc_bytes: 1000,
  });
  assert.equal(accumulate(ledger, at(100)), 0);
  assert.equal(accumulate(ledger, at(120)), 20);
  assert.equal(accumulate(ledger, at(80)), 0);
  assert.equal(accumulate(ledger, at(100)), 20);
  assert.equal(ledger.charged_bytes, 40);
  assert.equal(ledger.roots.repo.last_observed_bytes, 100);
  assert.equal(ledger.roots.repo.high_water_bytes, 120);
});

test('teto e piso param antes de exceder reserva', () => {
  const ledger = { charged_bytes: CAP_BYTES - RESERVE_BYTES };
  assert.equal(checkLimits(ledger, FLOOR_BYTES + RESERVE_BYTES), null);
  assert.equal(checkLimits({ charged_bytes: CAP_BYTES - RESERVE_BYTES + 1 }, FLOOR_BYTES + RESERVE_BYTES), 'cap');
  assert.equal(checkLimits({ charged_bytes: 0 }, FLOOR_BYTES + RESERVE_BYTES - 1), 'floor');
});

test('cap explícito de 2 GiB é vinculado à autorização e não vira 4 GiB', () => {
  const f = fixture(2 * GIB);
  const loaded = loadOrCreateLedger(f.baselinePath, f.ledger);
  assert.equal(loaded.capBytes, 2 * GIB);
  assert.equal(loaded.ledger.cap_bytes, 2 * GIB);
  assert.equal(checkLimits({ ...loaded.ledger, charged_bytes: 2 * GIB - RESERVE_BYTES }, 100 * GIB), null);
  assert.equal(checkLimits({ ...loaded.ledger, charged_bytes: 2 * GIB - RESERVE_BYTES + 1 }, 100 * GIB), 'cap');
  assert.equal(checkLimits({ ...loaded.ledger, charged_bytes: 0 }, FLOOR_BYTES + RESERVE_BYTES - 1), 'floor');
});

test('CLI cap explícito exige baseline e ledger com o mesmo cap', async () => {
  const f = fixture(2 * GIB);
  loadOrCreateLedger(f.baselinePath, f.ledger, 2 * GIB);
  await assert.rejects(
    runSupervisor(['--baseline', f.baselinePath, '--ledger', f.ledger, '--cap-bytes', String(4 * GIB), '--check'], { measureAll: () => sample(f.baseline) }),
    (error) => error.code === 'cap',
  );

  const legacy = fixture();
  await assert.rejects(
    runSupervisor(['--baseline', legacy.baselinePath, '--ledger', legacy.ledger, '--cap-bytes', String(2 * GIB), '--check'], { measureAll: () => sample(legacy.baseline) }),
    (error) => error.code === 'cap',
  );
});

test('baseline com cap inválido ou ausente falha fechado', () => {
  for (const value of [1.5, -1, 0, CAP_BYTES + 1, '2147483648', null, 9007199254740992]) {
    const f = fixture();
    f.baseline.cap_bytes = value;
    fs.writeFileSync(f.baselinePath, JSON.stringify(f.baseline), 'utf8');
    assert.throws(() => loadOrCreateLedger(f.baselinePath, f.ledger), (error) => error.code === 'baseline', String(value));
  }

  const missing = fixture();
  missing.baseline.task_id = 'TASK-NOVA-SEM-CAP';
  fs.writeFileSync(missing.baselinePath, JSON.stringify(missing.baseline), 'utf8');
  assert.throws(() => loadOrCreateLedger(missing.baselinePath, missing.ledger), (error) => error.code === 'baseline');
});

test('ledger com cap inválido ou ausente sob baseline com cap falha fechado', () => {
  const f = fixture();
  const { ledger } = loadOrCreateLedger(f.baselinePath, f.ledger);
  ledger.cap_bytes = 0;
  fs.writeFileSync(f.ledger, JSON.stringify(ledger), 'utf8');
  assert.throws(() => loadOrCreateLedger(f.baselinePath, f.ledger), (error) => error.code === 'ledger');

  const f2 = fixture(2 * GIB);
  const legacy = loadOrCreateLedger(fixture().baselinePath, path.join(f2.root, 'legacy-ledger.json')).ledger;
  delete legacy.cap_bytes;
  fs.writeFileSync(f2.ledger, JSON.stringify(legacy), 'utf8');
  assert.throws(() => loadOrCreateLedger(f2.baselinePath, f2.ledger), (error) => error.code === 'cap');
});

test('cap inválido falha fechado antes de criar ledger', async () => {
  for (const value of ['1.5', '-1', 'NaN', '0', String(CAP_BYTES + 1), '9007199254740992', 'abc', '']) {
    const f = fixture();
    await assert.rejects(
      runSupervisor(['--baseline', f.baselinePath, '--ledger', f.ledger, '--cap-bytes', value, '--check'], { measureAll: () => sample(f.baseline) }),
      (error) => error.code === 'config',
      value,
    );
    assert.equal(fs.existsSync(f.ledger), false, `ledger não deve existir para ${value}`);
  }
});

test('paths do baseline, ledger e ledger antigo não podem colidir', () => {
  const f = fixture();
  assert.throws(() => loadOrCreateLedger(f.baselinePath, f.baselinePath), (error) => error.code === 'identity');

  const oldCollision = { ...f.baseline, old_ledger_path: f.ledger };
  const oldCollisionPath = path.join(f.root, 'baseline-old-collision.json');
  fs.writeFileSync(oldCollisionPath, JSON.stringify(oldCollision), 'utf8');
  assert.throws(() => loadOrCreateLedger(oldCollisionPath, f.ledger), (error) => error.code === 'identity');
});

test('avaliação acumula a amostra atual antes de checar o cap', () => {
  const ledger = {
    roots: { repo: { start_bytes: 100, last_observed_bytes: 100, high_water_bytes: 100, charged_bytes: 0 } },
    vhd: { docker_data: { start_bytes: 1000, last_observed_bytes: 1000, high_water_bytes: 1000, charged_bytes: 0 } },
    pagefile: { start_bytes: 1000, last_observed_bytes: 1000, high_water_bytes: 1000, charged_bytes: 0 },
    charged_bytes: CAP_BYTES - RESERVE_BYTES - 10,
    updated_at: '',
  };
  const limit = evaluateSample(ledger, {
    free: 100 * GIB,
    roots: [{ name: 'repo', bytes: 120 }],
    vhd: [{ name: 'docker_data', bytes: 1000 }],
    pagefile_alloc_bytes: 1000,
  });
  assert.equal(limit, 'cap');
  assert.equal(ledger.charged_bytes, CAP_BYTES - RESERVE_BYTES + 10);
});

test('ledger novo retoma high-water e rejeita identidade diferente', () => {
  const f = fixture();
  const { ledger } = loadOrCreateLedger(f.baselinePath, f.ledger);
  const first = sample(f.baseline);
  first.roots[0].bytes += 10;
  accumulate(ledger, first);
  persistLedger(f.ledger, ledger);

  const resumed = loadOrCreateLedger(f.baselinePath, f.ledger).ledger;
  assert.equal(resumed.charged_bytes, 10);
  const lower = sample(f.baseline);
  lower.roots[0].bytes += 5;
  assert.equal(accumulate(resumed, lower), 0);
  assert.equal(resumed.charged_bytes, 10);

  const bad = { ...f.baseline, authorization_id: 'AUTH-OUTRA' };
  const badPath = path.join(f.root, 'bad-baseline.json');
  fs.writeFileSync(badPath, JSON.stringify(bad), 'utf8');
  assert.throws(() => loadOrCreateLedger(badPath, f.ledger), /autorização difere/);
});

test('ledger legado sem cap permanece bloqueado com charged, histórico e hash intactos', () => {
  const f = fixture();
  const { ledger } = loadOrCreateLedger(f.baselinePath, f.ledger);
  delete ledger.cap_bytes;
  ledger.status = 'blocked';
  ledger.charged_bytes = 6744564688;
  ledger.history = [{ at: '2026-11-03T00:00:00.000Z', status: 'blocked', charged_bytes: 6744564688, error: 'cap' }];
  fs.writeFileSync(f.ledger, `${JSON.stringify(ledger, null, 2)}\n`, 'utf8');
  const before = crypto.createHash('sha256').update(fs.readFileSync(f.ledger)).digest('hex');

  const loaded = loadOrCreateLedger(f.baselinePath, f.ledger);
  const after = crypto.createHash('sha256').update(fs.readFileSync(f.ledger)).digest('hex');
  assert.equal(loaded.capBytes, CAP_BYTES);
  assert.equal(loaded.ledger.status, 'blocked');
  assert.equal(loaded.ledger.charged_bytes, 6744564688);
  assert.equal(loaded.ledger.history.length, 1);
  assert.equal(before, after);
});

test('cap do ledger divergente da baseline falha fechado', () => {
  const f = fixture();
  loadOrCreateLedger(f.baselinePath, f.ledger);
  const baseline2 = { ...f.baseline, cap_bytes: 2 * GIB };
  const baseline2Path = path.join(f.root, 'baseline-cap-2.json');
  fs.writeFileSync(baseline2Path, JSON.stringify(baseline2), 'utf8');
  assert.throws(() => loadOrCreateLedger(baseline2Path, f.ledger), (error) => error.code === 'cap');
});

test('ledger v1 migra preservando charged e inicializa o último', () => {
  const f = fixture();
  const { ledger } = loadOrCreateLedger(f.baselinePath, f.ledger);
  const v1 = JSON.parse(JSON.stringify(ledger));
  v1.schema = 'm1-budget-supervisor/v1';
  delete v1.migration;
  v1.charged_bytes = 123;
  v1.roots.repo.charged_bytes = 123;
  for (const group of [v1.roots, v1.vhd]) {
    for (const state of Object.values(group)) delete state.last_observed_bytes;
  }
  delete v1.pagefile.last_observed_bytes;
  const v1Path = path.join(f.root, 'ledger-v1.json');
  fs.writeFileSync(v1Path, JSON.stringify(v1), 'utf8');
  const migrated = loadOrCreateLedger(f.baselinePath, v1Path).ledger;
  assert.equal(migrated.schema, 'm1-budget-supervisor/v2');
  assert.equal(migrated.charged_bytes, 123);
  assert.equal(migrated.roots.repo.last_observed_bytes, migrated.roots.repo.high_water_bytes);
});

test('fonte de raiz ausente falha fechado', () => {
  const missing = path.join(os.tmpdir(), 'tmp-m1-budget-missing-source-nao-existe');
  assert.throws(() => measureRoots([{ name: 'sumiu', path: missing }]), (error) => error.code === 'source');
});

test('cancelamento usa taskkill /PID do filho próprio', () => {
  const calls = [];
  const child = {
    pid: 4321,
    exitCode: null,
    killed: false,
    kill() { this.killed = true; },
  };
  const result = cancelOwnChild(child, {
    platform: 'win32',
    taskkill: (pid) => { calls.push(pid); return { status: 0 }; },
  });
  assert.equal(result, true);
  assert.deepEqual(calls, [4321]);
  assert.equal(child.killed, false, 'taskkill vencedor não deve chamar kill');
});

test('cancelamento não toca filho já terminado e faz fallback finito', () => {
  let calls = 0;
  const exited = { pid: 4321, exitCode: 0, kill() { calls += 1; } };
  assert.equal(cancelOwnChild(exited, { platform: 'win32', taskkill: () => { calls += 1; return { status: 0 }; } }), false);
  assert.equal(calls, 0);

  const fallback = { pid: 4322, exitCode: null, killed: false, kill() { this.killed = true; } };
  assert.equal(cancelOwnChild(fallback, {
    platform: 'win32',
    taskkill: () => ({ status: 1 }),
  }), true);
  assert.equal(fallback.killed, true);
  assert.equal(cancelOwnChild(null), false);
});

test('--check preserva o ledger antigo e não iniciar child', async () => {
  const f = fixture();
  const before = crypto.createHash('sha256').update(fs.readFileSync(f.oldLedger)).digest('hex');
  const result = await runSupervisor([
    '--baseline', f.baselinePath,
    '--ledger', f.ledger,
    '--check',
  ], { measureAll: () => sample(f.baseline) });
  const after = crypto.createHash('sha256').update(fs.readFileSync(f.oldLedger)).digest('hex');
  assert.equal(result.code, 0);
  assert.equal(before, after);
  assert.equal(JSON.parse(fs.readFileSync(f.ledger, 'utf8')).charged_bytes, 0);
});

test('CLI direto funciona em caminho com espaço', () => {
  const script = path.join(path.dirname(fileURLToPath(import.meta.url)), 'm1-budget-supervisor.mjs');
  const result = spawnSync(process.execPath, [script, '--help'], { encoding: 'utf8' });
  assert.equal(result.status, 0);
  assert.match(result.stdout, /Uso: node scripts\/m1-budget-supervisor\.mjs/);
});

function loopArgs(f) {
  return [
    '--baseline', f.baselinePath,
    '--ledger', f.ledger,
    '--interval-seconds', '1',
    '--timeout-seconds', '10',
    '--', 'comando-sintetico',
  ];
}

function loopDeps(clock, child, extra = {}) {
  return {
    spawn: () => child,
    setInterval: clock.setInterval,
    clearInterval: clock.clearInterval,
    setTimeout: clock.setTimeout,
    clearTimeout: clock.clearTimeout,
    platform: 'win32',
    ...extra,
  };
}

test('loop real: justo abaixo do cap segue, amostra seguinte bloqueia e cancela antes da próxima', async () => {
  const f = fixture();
  const clock = fakeClock();
  const child = fakeChild();
  const base = sample(f.baseline);
  const limitCharge = CAP_BYTES - RESERVE_BYTES;
  let measures = 0;
  let measuresAtCancel = null;
  const taskkillPids = [];
  const measureAll = () => {
    measures += 1;
    if (measures === 1) return base;
    const current = sample(f.baseline);
    current.roots[0].bytes += measures === 2 ? limitCharge : limitCharge + 1;
    return current;
  };
  const pending = runSupervisor(loopArgs(f), loopDeps(clock, child, {
    measureAll,
    taskkill: (pid) => { taskkillPids.push(pid); return { status: 1 }; },
    cancelOwnChild: (ownChild, deps) => {
      measuresAtCancel = measures;
      return cancelOwnChild(ownChild, deps);
    },
  }));
  clock.tick();
  clock.tick();
  const result = await pending;

  assert.equal(result.code, 3);
  assert.match(result.stderr, /cap/);
  assert.equal(measuresAtCancel, 3, 'cancelamento ocorre na amostra que cruzou, antes da final');
  assert.equal(measures, 4, 'nenhuma amostra de intervalo após o bloqueio; só a final');
  assert.deepEqual(taskkillPids, [777]);
  assert.equal(child.killed, true);
  assert.equal(clock.intervalCount(), 0);
  assert.equal(clock.timeoutCount(), 0);

  const ledger = JSON.parse(fs.readFileSync(f.ledger, 'utf8'));
  assert.equal(ledger.status, 'blocked');
  assert.equal(ledger.charged_bytes, limitCharge + 1, 'cobrança da amostra que cruzou é preservada');
  assert.deepEqual(ledger.history.map((entry) => entry.status), ['running', 'blocked', 'blocked']);
  assert.equal(ledger.history[1].charged_bytes, limitCharge + 1);
  assert.equal(ledger.history[1].error, 'cap');
});

test('loop real: amostra que cruza o piso bloqueia e cancela na mesma passagem', async () => {
  const f = fixture();
  const clock = fakeClock();
  const child = fakeChild();
  const base = sample(f.baseline);
  let measures = 0;
  const measureAll = () => {
    measures += 1;
    if (measures === 1) return base;
    const current = sample(f.baseline, FLOOR_BYTES + RESERVE_BYTES - 1);
    current.roots[0].bytes += 10;
    return current;
  };
  const pending = runSupervisor(loopArgs(f), loopDeps(clock, child, {
    measureAll,
    taskkill: () => ({ status: 1 }),
  }));
  clock.tick();
  const result = await pending;

  assert.equal(result.code, 3);
  assert.match(result.stderr, /floor/);
  assert.equal(child.killed, true);
  const ledger = JSON.parse(fs.readFileSync(f.ledger, 'utf8'));
  assert.equal(ledger.status, 'blocked');
  assert.equal(ledger.charged_bytes, 10);
  assert.deepEqual(ledger.history.map((entry) => entry.status), ['blocked', 'blocked']);
  assert.equal(ledger.history[0].error, 'floor');
});

test('loop real: falha de fonte cancela e termina em erro sem seguir amostrando', async () => {
  const f = fixture();
  const clock = fakeClock();
  const child = fakeChild();
  const base = sample(f.baseline);
  let measures = 0;
  let measuresAtCancel = null;
  const measureAll = () => {
    measures += 1;
    if (measures === 1) return base;
    const error = new Error('fonte sumiu');
    error.code = 'source';
    throw error;
  };
  const pending = runSupervisor(loopArgs(f), loopDeps(clock, child, {
    measureAll,
    taskkill: () => ({ status: 1 }),
    cancelOwnChild: (ownChild, deps) => {
      measuresAtCancel = measures;
      return cancelOwnChild(ownChild, deps);
    },
  }));
  clock.tick();
  const result = await pending;

  assert.equal(result.code, 4);
  assert.match(result.stderr, /fonte sumiu/);
  assert.equal(measuresAtCancel, 3);
  assert.equal(measures, 4, 'sem ticks de intervalo após a falha; só a medição final');
  assert.equal(child.killed, true);
  assert.equal(JSON.parse(fs.readFileSync(f.ledger, 'utf8')).status, 'error');
});

test('loop real: child já terminado não recebe cancelamento', async () => {
  const f = fixture();
  const clock = fakeClock();
  const child = fakeChild();
  let taskkillCalls = 0;
  const result = await runSupervisor(loopArgs(f), loopDeps(clock, child, {
    measureAll: () => sample(f.baseline),
    spawn: () => {
      queueMicrotask(() => { child.exitCode = 0; child.emit('exit', 0, null); });
      return child;
    },
    taskkill: () => { taskkillCalls += 1; return { status: 0 }; },
  }));

  assert.equal(result.code, 0);
  assert.equal(taskkillCalls, 0);
  assert.equal(child.killed, false);
  assert.equal(JSON.parse(fs.readFileSync(f.ledger, 'utf8')).status, 'ok');
});

test('loop real: ledger já bloqueado não inicia child', async () => {
  const f = fixture();
  const over = sample(f.baseline);
  over.roots[0].bytes += CAP_BYTES;
  const first = await runSupervisor(loopArgs(f), { measureAll: () => over });
  assert.equal(first.code, 3);
  let spawned = 0;
  const second = await runSupervisor(loopArgs(f), {
    measureAll: () => over,
    spawn: () => { spawned += 1; return fakeChild(); },
  });
  assert.equal(second.code, 3);
  assert.equal(spawned, 0);
});

test('loop real: cap 2 GiB bloqueado não inicia child', async () => {
  const f = fixture(2 * GIB);
  const over = sample(f.baseline);
  over.roots[0].bytes += 2 * GIB;
  let spawned = 0;
  const result = await runSupervisor(loopArgs(f), {
    measureAll: () => over,
    spawn: () => { spawned += 1; return fakeChild(); },
  });
  assert.equal(result.code, 3);
  assert.match(result.stderr, /cap/);
  assert.equal(spawned, 0);
  assert.equal(JSON.parse(fs.readFileSync(f.ledger, 'utf8')).cap_bytes, 2 * GIB);
});

test('loop real: cap 2 GiB bloqueia e cancela na mesma amostra', async () => {
  const f = fixture(2 * GIB);
  const clock = fakeClock();
  const child = fakeChild();
  const base = sample(f.baseline);
  const limitCharge = 2 * GIB - RESERVE_BYTES;
  let measures = 0;
  let measuresAtCancel = null;
  const taskkillPids = [];
  const measureAll = () => {
    measures += 1;
    if (measures === 1) return base;
    const current = sample(f.baseline);
    current.roots[0].bytes += measures === 2 ? limitCharge : limitCharge + 1;
    return current;
  };
  const pending = runSupervisor(loopArgs(f), loopDeps(clock, child, {
    measureAll,
    taskkill: (pid) => { taskkillPids.push(pid); return { status: 1 }; },
    cancelOwnChild: (ownChild, deps) => {
      measuresAtCancel = measures;
      return cancelOwnChild(ownChild, deps);
    },
  }));
  clock.tick();
  clock.tick();
  const result = await pending;

  assert.equal(result.code, 3);
  assert.match(result.stderr, /cap/);
  assert.equal(measuresAtCancel, 3, 'cancelamento ocorre na amostra que cruzou');
  assert.deepEqual(taskkillPids, [777]);
  assert.equal(child.killed, true);
  const ledger = JSON.parse(fs.readFileSync(f.ledger, 'utf8'));
  assert.equal(ledger.cap_bytes, 2 * GIB);
  assert.equal(ledger.status, 'blocked');
  assert.equal(ledger.charged_bytes, limitCharge + 1);
  assert.deepEqual(ledger.history.map((entry) => entry.status), ['running', 'blocked', 'blocked']);
  assert.equal(ledger.history[1].error, 'cap');
});

test('fast cancela antes da varredura lenta resolver', async () => {
  const f = fixture(2 * GIB);
  const child = fakeChild();
  const baseSlow = { roots: sample(f.baseline).roots, vhd: sample(f.baseline).vhd };
  const baseFast = { free: 100 * GIB, pagefile_alloc_bytes: 6000 };
  let fastCalls = 0;
  let slowCalls = 0;
  let slowCanceled = false;
  const pendingSlow = new Promise(() => {});
  pendingSlow.cancel = () => { slowCanceled = true; };
  const measureFast = () => {
    fastCalls += 1;
    return Promise.resolve(fastCalls === 1 ? baseFast : { ...baseFast, pagefile_alloc_bytes: 6000 + 2 * GIB });
  };
  const measureSlow = () => {
    slowCalls += 1;
    return slowCalls === 1 ? Promise.resolve(baseSlow) : pendingSlow;
  };
  const result = await runSupervisor(loopArgs(f), {
    measureFast,
    measureSlow,
    spawn: () => child,
    taskkill: () => ({ status: 1 }),
  });
  assert.equal(result.code, 3);
  assert.match(result.stderr, /cap/);
  assert.equal(slowCalls, 2);
  assert.equal(slowCanceled, true);
  assert.equal(child.killed, true);
  const ledger = JSON.parse(fs.readFileSync(f.ledger, 'utf8'));
  assert.equal(ledger.status, 'blocked');
  assert.equal(ledger.history.at(-1).status, 'blocked');
  assert.equal(ledger.history.at(-1).error, 'cap');
});

test('measureSlowDefault worker mede caminhos locais conhecidos sem recursão', async () => {
  const baseline = {
    roots: [{ name: 'scripts', path: path.resolve('scripts'), start_bytes: 0 }],
    vhd: [{ name: 'cargo', path: path.resolve('Cargo.toml'), start_bytes: 0 }],
  };
  const slow = await measureSlowDefault(baseline, { measureTimeoutMs: 5000 });
  assert.equal(slow.roots.length, 1);
  assert.equal(slow.vhd.length, 1);
  assert.ok(slow.roots[0].bytes > 0);
});

test('measureSlowDefault timeout falha fechado e encerra worker', async () => {
  let terminated = false;
  const worker = { once() { return worker; }, terminate() { terminated = true; } };
  const f = fixture();
  await assert.rejects(
    measureSlowDefault(f.baseline, { createWorker: () => worker, measureTimeoutMs: 10 }),
    (error) => error.code === 'measure-timeout',
  );
  assert.equal(terminated, true);
});

test('timeout de medição antes do spawn não inicia child', async () => {
  const f = fixture();
  let spawned = 0;
  const result = await runSupervisor(loopArgs(f), {
    measureFast: () => Promise.resolve({ free: 100 * GIB, pagefile_alloc_bytes: 6000 }),
    measureSlow: () => Promise.reject(Object.assign(new Error('slow'), { code: 'measure-timeout' })),
    spawn: () => { spawned += 1; return fakeChild(); },
  });
  assert.equal(result.code, 4);
  assert.match(result.stderr, /measure-timeout/);
  assert.equal(spawned, 0);
  assert.equal(JSON.parse(fs.readFileSync(f.ledger, 'utf8')).status, 'error');
});

test('headroom observado insuficiente bloqueia antes do spawn', async () => {
  const f = fixture(2 * GIB);
  const { ledger } = loadOrCreateLedger(f.baselinePath, f.ledger);
  ledger.charged_bytes = 2 * GIB - RESERVE_BYTES - 1;
  ledger.history = [
    { pagefile_bytes: 100, roots_bytes: 100, charged_bytes: 0 },
    { pagefile_bytes: 600, roots_bytes: 700, charged_bytes: 0 },
  ];
  persistLedger(f.ledger, ledger);
  let spawned = 0;
  const result = await runSupervisor(loopArgs(f), {
    measureFast: () => Promise.resolve({ free: 100 * GIB, pagefile_alloc_bytes: 6000 }),
    measureSlow: () => Promise.resolve({ roots: sample(f.baseline).roots, vhd: sample(f.baseline).vhd }),
    spawn: () => { spawned += 1; return fakeChild(); },
  });
  assert.equal(result.code, 3);
  assert.match(result.stderr, /headroom/);
  assert.equal(spawned, 0);
  assert.equal(startHeadroomIssue(ledger), 'headroom');
});

test('fast e slow cobram segmentos distintos sem dupla contagem', () => {
  const f = fixture();
  const { ledger } = loadOrCreateLedger(f.baselinePath, f.ledger);
  const base = sample(f.baseline);
  const fast = { free: base.free, pagefile_alloc_bytes: base.pagefile_alloc_bytes + 100 };
  const slow = {
    roots: base.roots.map((root, index) => index === 0 ? { ...root, bytes: root.bytes + 10 } : root),
    vhd: base.vhd.map((item, index) => index === 0 ? { ...item, bytes: item.bytes + 20 } : item),
  };
  assert.equal(accumulateFast(ledger, fast), 100);
  assert.equal(accumulateSlow(ledger, slow), 30);
  assert.equal(ledger.charged_bytes, 130);
  assert.equal(accumulateFast(ledger, fast), 0);
  assert.equal(accumulateSlow(ledger, slow), 0);
  assert.equal(ledger.charged_bytes, 130);
});
