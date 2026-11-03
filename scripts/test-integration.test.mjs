import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import process from 'node:process';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import {
  TestIntegrationError,
  fixtureCases,
  parseDbUrl,
  runCli,
} from './test-integration.mjs';

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const SCRIPT = path.join(SCRIPT_DIR, 'test-integration.mjs');

function childCheck(env) {
  return spawnSync(process.execPath, [SCRIPT, '--check'], {
    encoding: 'utf8',
    env: { ...process.env, ...env },
    timeout: 30_000,
  });
}

test('validador usa a fixture compartilhada com o Rust', () => {
  const cases = fixtureCases();
  for (const url of cases.valid) {
    assert.doesNotThrow(() => parseDbUrl(url), `deveria aceitar ${url}`);
  }
  for (const url of cases.invalid) {
    assert.throws(() => parseDbUrl(url), TestIntegrationError, `deveria recusar ${url}`);
  }
});

test('host literal de loopback e banco dedicado são obrigatórios', () => {
  assert.deepEqual(parseDbUrl('postgres://u:p@127.0.0.1:55432/previdenciar_test_1'), {
    host: '127.0.0.1',
    port: 55432,
    database: 'previdenciar_test_1',
    username: 'u',
  });
  assert.throws(() => parseDbUrl('postgres://u:p@localhost:55432/previdenciar_test_1'), /host não local/);
  assert.throws(() => parseDbUrl('postgres://u:p@192.0.2.1:55432/previdenciar_test_1'), /host não local/);
  assert.throws(() => parseDbUrl('postgres://u:p@127.0.0.1:55432/postgres'), /previdenciar_test/);
  assert.throws(() => parseDbUrl('postgres://u:p@127.0.0.1:55432/previdenciar_test_1?host=x'), /query/);
});

test('sem TEST_DATABASE_URL o comando de check falha antes de Cargo', () => {
  const result = childCheck({ TEST_DATABASE_URL: '' });
  assert.equal(result.status, 2);
  assert.match(result.stderr, /TEST_DATABASE_URL/);
  assert.doesNotMatch(result.stderr, /cargo/i);
});

test('host remoto no comando de check falha antes de conexão/Cargo', () => {
  const result = childCheck({
    TEST_DATABASE_URL: 'postgres://u:p@192.0.2.1:55432/previdenciar_test_1',
  });
  assert.equal(result.status, 2);
  assert.match(result.stderr, /host não local/);
  assert.doesNotMatch(result.stderr, /cargo/i);
});

test('URL local válida no check não imprime senha nem executa Cargo', () => {
  const result = childCheck({
    TEST_DATABASE_URL: 'postgres://u:segredo-sintetico@127.0.0.1:55432/previdenciar_test_1',
  });
  assert.equal(result.status, 0);
  assert.match(result.stdout, /not-executed/);
  assert.doesNotMatch(result.stdout, /segredo-sintetico/);
  assert.doesNotMatch(result.stderr, /segredo-sintetico/);
});

test('run sem roots falha antes de wrapper e Cargo', async () => {
  const result = await runCli([
    '--run',
  ]);
  assert.equal(result.code, 2);
  assert.match(result.stderr, /--ledger/);
  assert.doesNotMatch(result.stderr, /cargo/i);
});

test('run com roots e sem banco falha antes de wrapper e Cargo', () => {
  const result = spawnSync(process.execPath, [
    SCRIPT, '--run',
    '--ledger', 'ledger.json',
    '--baseline', 'baseline.json',
    '--saga', '.',
    '--target', 'target',
    '--repo', '.',
  ], {
    encoding: 'utf8',
    env: { ...process.env, TEST_DATABASE_URL: '' },
    timeout: 30_000,
  });
  assert.equal(result.status, 2);
  assert.match(result.stderr, /TEST_DATABASE_URL/);
  assert.doesNotMatch(result.stderr, /cargo/i);
});
