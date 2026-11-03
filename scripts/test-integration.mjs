#!/usr/bin/env node
/**
 * Comando único de integração do servidor.
 *
 * --check: valida TEST_DATABASE_URL e o plano, sem Cargo, sem conexão e sem
 *          imprimir a URL. Sem banco válido retorna não-zero antes de SQL.
 * --run:   valida e delega ao wrapper de orçamento aprovado; o Cargo é serial,
 *          offline, -j 1 e somente previdenciar-web-server. Não use antes da
 *          revisão estática. O ledger é persistente; não há reset nem init.
 */
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const WRAPPER = path.join(SCRIPT_DIR, 'run-with-disk-budget.mjs');
const FIXTURE = path.join(SCRIPT_DIR, 'test-db-url-cases.json');
const DB_PREFIX = 'previdenciar_test_';
const USAGE = `Integração segura do servidor

  node scripts/test-integration.mjs --check
  node scripts/test-integration.mjs --run \\
    --ledger <json> --baseline <json> \\
    --saga <dir> --target <dir> --repo <dir> \\
    [--temp <dir>] [--pg <dir>] [--cache <nome>=<dir>] \\
    [--timeout-seconds N]

--check valida TEST_DATABASE_URL e não executa Cargo, banco ou rede.
--run usa o wrapper aprovado, Cargo offline -j 1 e ledger persistente.
Sem TEST_DATABASE_URL, host não literal de loopback ou banco fora de
previdenciar_test_<id>, o comando falha antes de SQL. A URL nunca é impressa.`;

export class TestIntegrationError extends Error {
  constructor(message) {
    super(message);
    this.name = 'TestIntegrationError';
  }
}

function fail(message) {
  throw new TestIntegrationError(message);
}

/**
 * Mesmas regras do validador Rust:
 * host literal 127.0.0.1/::1, porta e banco explícitos, sem query/host
 * override, multi-host, localhost/DNS, fragmento ou banco padrão.
 */
export function parseDbUrl(raw) {
  const value = String(raw ?? '').trim();
  if (!value) fail('TEST_DATABASE_URL ausente; banco isolado explícito é obrigatório');
  if (/[\s\u0000-\u001f]/.test(value)) fail('TEST_DATABASE_URL com espaço ou controle');
  const match = /^(postgres|postgresql):\/\/(.*)$/.exec(value);
  if (!match) fail('TEST_DATABASE_URL deve usar postgres:// ou postgresql://');
  const rest = match[2];
  if (/[?#\\,]/.test(rest)) fail('TEST_DATABASE_URL não aceita query, fragmento, multi-host ou escape');
  const slash = rest.indexOf('/');
  if (slash <= 0) fail('TEST_DATABASE_URL sem banco explícito');
  const authority = rest.slice(0, slash);
  const database = rest.slice(slash + 1);
  if (!database || database.includes('/') || database.includes('%')) {
    fail('TEST_DATABASE_URL precisa de um único banco explícito');
  }
  if (!database.startsWith(DB_PREFIX)
    || database.length <= DB_PREFIX.length
    || !/^[A-Za-z0-9_-]+$/.test(database.slice(DB_PREFIX.length))) {
    fail('TEST_DATABASE_URL deve usar banco previdenciar_test_<id>');
  }
  const at = authority.lastIndexOf('@');
  if (at <= 0) fail('TEST_DATABASE_URL exige usuário explícito');
  const userinfo = authority.slice(0, at);
  const hostport = authority.slice(at + 1);
  const username = userinfo.split(':')[0];
  if (!username || username.includes('%')) fail('TEST_DATABASE_URL exige usuário explícito');
  let host;
  let port;
  if (hostport.startsWith('[')) {
    const end = hostport.indexOf(']');
    if (end < 0) fail('TEST_DATABASE_URL com IPv6 inválido');
    host = hostport.slice(1, end);
    const after = hostport.slice(end + 1);
    if (!after.startsWith(':')) fail('TEST_DATABASE_URL exige porta explícita');
    port = after.slice(1);
  } else {
    const colon = hostport.lastIndexOf(':');
    if (colon <= 0) fail('TEST_DATABASE_URL exige porta explícita');
    host = hostport.slice(0, colon);
    port = hostport.slice(colon + 1);
  }
  if (host !== '127.0.0.1' && host !== '::1') {
    fail('TEST_DATABASE_URL recusa host não local (use 127.0.0.1 ou ::1)');
  }
  if (!/^\d+$/.test(port) || Number(port) < 1 || Number(port) > 65535) {
    fail('TEST_DATABASE_URL com porta inválida');
  }
  return { host, port: Number(port), database, username };
}

function readEnvUrl() {
  return process.env.TEST_DATABASE_URL ?? '';
}

function parseArgs(argv) {
  const parsed = {
    help: false,
    check: false,
    run: false,
    ledger: null,
    baseline: null,
    saga: null,
    target: null,
    repo: null,
    temp: null,
    pg: null,
    caches: [],
    timeoutSeconds: 3600,
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === '--help' || arg === '-h') {
      parsed.help = true;
      continue;
    }
    if (arg === '--check') {
      parsed.check = true;
      continue;
    }
    if (arg === '--run') {
      parsed.run = true;
      continue;
    }
    if (arg === '--cache') {
      const value = argv[index + 1];
      if (!value) fail('--cache exige nome=path');
      parsed.caches.push(value);
      index += 1;
      continue;
    }
    const value = argv[index + 1];
    if (!value) fail(`${arg} exige um valor`);
    if (arg === '--ledger') parsed.ledger = value;
    else if (arg === '--baseline') parsed.baseline = value;
    else if (arg === '--saga') parsed.saga = value;
    else if (arg === '--target') parsed.target = value;
    else if (arg === '--repo') parsed.repo = value;
    else if (arg === '--temp') parsed.temp = value;
    else if (arg === '--pg') parsed.pg = value;
    else if (arg === '--timeout-seconds') {
      if (!/^\d+$/.test(value) || Number(value) < 1) fail('--timeout-seconds deve ser positivo');
      parsed.timeoutSeconds = Number(value);
    } else fail('argumento desconhecido');
    index += 1;
  }
  if (parsed.help) return parsed;
  if (parsed.check === parsed.run) fail('use --check ou --run');
  return parsed;
}

function validateForRun(parsed) {
  for (const key of ['ledger', 'baseline', 'saga', 'target', 'repo']) {
    if (!parsed[key]) fail(`--${key} é obrigatório em --run`);
  }
}

export function checkEnvironment() {
  const parsed = parseDbUrl(readEnvUrl());
  return {
    status: 'ready',
    hostClass: 'loopback',
    databaseClass: `${DB_PREFIX}*`,
    port: parsed.port,
    cargo: 'not-executed',
  };
}

function wrapperArgs(parsed) {
  const args = [
    '--ledger', parsed.ledger,
    '--baseline', parsed.baseline,
    '--root', `target=${parsed.target}`,
    '--root', `repo=${parsed.repo}`,
    '--root', `saga=${parsed.saga}`,
    '--cwd', parsed.repo,
    '--timeout-seconds', String(parsed.timeoutSeconds),
  ];
  if (parsed.temp) args.push('--root', `temp=${parsed.temp}`);
  if (parsed.pg) args.push('--root', `pg=${parsed.pg}`);
  for (const cache of parsed.caches) args.push('--cache', cache);
  args.push('--', 'cargo', 'test', '-p', 'previdenciar-web-server', '--locked', '--offline', '-j', '1');
  return args;
}

export async function runCli(argv, { spawnImpl = spawn } = {}) {
  try {
    const parsed = parseArgs(argv);
    if (parsed.help) return { code: 0, stdout: `${USAGE}\n`, stderr: '' };
    if (parsed.check) {
      const result = checkEnvironment();
      return { code: 0, stdout: `${JSON.stringify(result)}\n`, stderr: '' };
    }
    validateForRun(parsed);
    parseDbUrl(readEnvUrl());
    return await new Promise((resolve) => {
      const child = spawnImpl(process.execPath, [WRAPPER, ...wrapperArgs(parsed)], {
        stdio: 'inherit',
        env: process.env,
        windowsHide: true,
      });
      child.once('error', () => resolve({ code: 4, stdout: '', stderr: 'falha ao iniciar o wrapper\n' }));
      child.once('exit', (code, signal) => {
        resolve({
          code: signal ? 130 : (Number.isInteger(code) ? code : 4),
          stdout: '',
          stderr: '',
        });
      });
    });
  } catch (error) {
    const message = error instanceof TestIntegrationError ? error.message : 'erro inesperado';
    return { code: 2, stdout: '', stderr: `test-integration: ${message}\n` };
  }
}

export function fixtureCases() {
  return JSON.parse(fs.readFileSync(FIXTURE, 'utf8'));
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
