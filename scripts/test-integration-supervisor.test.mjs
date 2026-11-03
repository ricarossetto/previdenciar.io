import test from 'node:test';
import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {
  oldLedgerHash,
  parseRunnerArgs,
  runRunner,
  supervisorArgs,
} from './test-integration-supervisor.mjs';

const DB_URL = 'postgres://u:senha-sintetica@127.0.0.1:55432/previdenciar_test_1';

function fixture() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'tmp-m1-budget-runner-'));
  const baseline = path.join(root, 'baseline.json');
  const ledger = path.join(root, 'ledger.json');
  const oldLedger = path.join(root, 'old-ledger.json');
  fs.writeFileSync(baseline, '{"authorization_id":"AUTH-TEST"}\n');
  fs.writeFileSync(ledger, '{"schema":"m1-budget-supervisor/v2"}\n');
  fs.writeFileSync(oldLedger, '{"schema":"old","used":1359538780}\n');
  return { root, baseline, ledger, oldLedger };
}

test('runner exige baseline, ledger novo e ledger antigo distintos', () => {
  const f = fixture();
  assert.throws(() => parseRunnerArgs(['--baseline', f.baseline, '--ledger', f.ledger]), /old-ledger/);
  assert.throws(() => parseRunnerArgs(['--baseline', f.baseline, '--ledger', f.ledger, '--old-ledger', f.ledger]), /não pode ser o ledger/);
  const parsed = parseRunnerArgs(['--baseline', f.baseline, '--ledger', f.ledger, '--old-ledger', f.oldLedger]);
  const args = supervisorArgs(parsed);
  assert.deepEqual(args.slice(-8), ['cargo', 'test', '-p', 'previdenciar-web-server', '--locked', '--offline', '-j', '1']);
  assert.ok(args.includes('--baseline'));
  assert.ok(args.includes('--ledger'));
  assert.ok(!args.includes(f.oldLedger), 'ledger antigo não entra no supervisor');
});

test('--check valida DB e não imprime URL/senha', async () => {
  const f = fixture();
  const result = await runRunner([
    '--check',
    '--baseline', f.baseline,
    '--ledger', f.ledger,
    '--old-ledger', f.oldLedger,
  ], { env: { ...process.env, TEST_DATABASE_URL: DB_URL } });
  assert.equal(result.code, 0);
  assert.match(result.stdout, /supervisor-4gib/);
  assert.doesNotMatch(`${result.stdout}${result.stderr}`, /senha-sintetica|postgres:\/\//);
});

test('runner verifica hash do ledger antigo antes/depois', async () => {
  const f = fixture();
  const before = oldLedgerHash(f.oldLedger);
  const calls = [];
  const fake = {
    once(event, callback) {
      if (event === 'exit') queueMicrotask(() => callback(0, null));
      return this;
    },
  };
  const result = await runRunner([
    '--baseline', f.baseline,
    '--ledger', f.ledger,
    '--old-ledger', f.oldLedger,
  ], {
    env: { ...process.env, TEST_DATABASE_URL: DB_URL },
    spawnImpl: (command, args) => {
      calls.push({ command, args });
      return fake;
    },
  });
  assert.equal(result.code, 0);
  assert.equal(calls.length, 1);
  assert.equal(oldLedgerHash(f.oldLedger), before);
});

test('runner recusa execução se o ledger antigo mudar', async () => {
  const f = fixture();
  const fake = {
    once(event, callback) {
      if (event === 'exit') {
        fs.appendFileSync(f.oldLedger, ' ');
        queueMicrotask(() => callback(0, null));
      }
      return this;
    },
  };
  const result = await runRunner([
    '--baseline', f.baseline,
    '--ledger', f.ledger,
    '--old-ledger', f.oldLedger,
  ], {
    env: { ...process.env, TEST_DATABASE_URL: DB_URL },
    spawnImpl: () => fake,
  });
  assert.equal(result.code, 5);
  assert.match(result.stderr, /ledger antigo mudou/);
});
