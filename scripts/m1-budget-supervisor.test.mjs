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
  cancelOwnChild,
  checkLimits,
  loadOrCreateLedger,
  measureRoots,
  persistLedger,
  runSupervisor,
} from './m1-budget-supervisor.mjs';

function fixture() {
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

test('teto durante execução cancela somente o child próprio', async () => {
  const f = fixture();
  let calls = 0;
  const fake = {
    killed: false,
    exit: null,
    once(event, callback) {
      if (event === 'exit') this.exit = callback;
      return this;
    },
    kill() {
      this.killed = true;
      if (this.exit) this.exit(null, null);
    },
  };
  const measureAll = () => {
    calls += 1;
    if (calls === 1) return sample(f.baseline);
    const over = sample(f.baseline);
    over.roots[0].bytes += CAP_BYTES;
    return over;
  };
  const result = await runSupervisor([
    '--baseline', f.baselinePath,
    '--ledger', f.ledger,
    '--interval-seconds', '1',
    '--timeout-seconds', '10',
    '--', 'comando-sintetico',
  ], { measureAll, spawn: () => fake });
  assert.equal(result.code, 3);
  assert.equal(fake.killed, true);
  assert.match(result.stderr, /cap/);
  assert.equal(JSON.parse(fs.readFileSync(f.ledger, 'utf8')).status, 'blocked');
});
