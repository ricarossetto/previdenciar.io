import test from 'node:test';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
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
    roots: Object.fromEntries(start.roots.map((root) => [root.name, { high_water_bytes: root.start_bytes, charged_bytes: 0 }])),
    vhd: Object.fromEntries(start.vhd.map((item) => [item.name, { high_water_bytes: item.start_bytes, charged_bytes: 0 }])),
    pagefile: { high_water_bytes: start.pagefile.start_alloc_bytes, charged_bytes: 0 },
    charged_bytes: 0,
    updated_at: '',
  };
}

test('soma positiva é cumulativa e redução não gera crédito', () => {
  const f = fixture();
  const ledger = ledgerWithRoots(f.baseline);
  const first = sample(f.baseline);
  first.roots[0].bytes += 10;
  first.roots[1].bytes += 20;
  first.vhd[0].bytes += 30;
  first.pagefile_alloc_bytes += 40;
  assert.equal(accumulate(ledger, first), 100);
  assert.equal(ledger.charged_bytes, 100);

  const lower = sample(f.baseline);
  lower.roots[0].bytes += 1;
  lower.roots[1].bytes += 2;
  lower.vhd[0].bytes += 3;
  lower.pagefile_alloc_bytes += 4;
  assert.equal(accumulate(ledger, lower), 0);
  assert.equal(ledger.charged_bytes, 100);

  const higher = sample(f.baseline);
  higher.roots[0].bytes += 15;
  higher.roots[1].bytes += 25;
  higher.vhd[0].bytes += 35;
  higher.pagefile_alloc_bytes += 45;
  assert.equal(accumulate(ledger, higher), 20);
  assert.equal(ledger.charged_bytes, 120);
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

test('fonte de raiz ausente falha fechado', () => {
  const missing = path.join(os.tmpdir(), 'tmp-m1-budget-missing-source-nao-existe');
  assert.throws(() => measureRoots([{ name: 'sumiu', path: missing }]), (error) => error.code === 'source');
});

test('cancelamento só chama kill do filho próprio', () => {
  const child = { killed: false, kill() { this.killed = true; } };
  assert.equal(cancelOwnChild(child), true);
  assert.equal(child.killed, true);
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
