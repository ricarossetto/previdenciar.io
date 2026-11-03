#!/usr/bin/env node
/**
 * Caminho explícito de execução dos testes com o supervisor M1 de 4 GiB.
 *
 * Reutiliza a validação de TEST_DATABASE_URL e o ambiente sanitizado do
 * test-integration.mjs, mas NÃO usa o wrapper/ledger antigo de 2 GiB.
 * O ledger antigo é somente lido por hash: se mudar, o comando falha.
 */
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { parseDbUrl, sanitizedEnv } from './test-integration.mjs';

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const SUPERVISOR = path.join(SCRIPT_DIR, 'm1-budget-supervisor.mjs');
const USAGE = `Integração do servidor com supervisor M1 (4 GiB)

  node scripts/test-integration-supervisor.mjs --check \\
    --baseline <M1-baseline.json> --ledger <M1-ledger.json> \\
    --old-ledger <ledger-antigo.json> --repo <dir> [--timeout-seconds N]

  node scripts/test-integration-supervisor.mjs \\
    --baseline <M1-baseline.json> --ledger <M1-ledger.json> \\
    --old-ledger <ledger-antigo.json> --repo <dir> [--timeout-seconds N]

O ledger antigo não é usado como orçamento e não é escrito; seu SHA-256 é
conferido antes/depois. TEST_DATABASE_URL é validada e sanitizada. Cargo é
offline, --locked, -j 1 e somente previdenciar-web-server. A URL nunca é impressa.`;

export class SupervisorIntegrationError extends Error {
  constructor(message) {
    super(message);
    this.name = 'SupervisorIntegrationError';
  }
}

const fail = (message) => { throw new SupervisorIntegrationError(message); };

export function parseRunnerArgs(argv) {
  const parsed = { help: false, check: false, baseline: null, ledger: null, oldLedger: null, repo: process.cwd(), timeoutSeconds: 3600 };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === '--help' || arg === '-h') { parsed.help = true; continue; }
    if (arg === '--check') { parsed.check = true; continue; }
    const value = argv[index + 1];
    if (!value) fail(`${arg} exige valor`);
    if (arg === '--baseline') parsed.baseline = path.resolve(value);
    else if (arg === '--ledger') parsed.ledger = path.resolve(value);
    else if (arg === '--old-ledger') parsed.oldLedger = path.resolve(value);
    else if (arg === '--repo') parsed.repo = path.resolve(value);
    else if (arg === '--timeout-seconds') {
      if (!/^\d+$/.test(value) || Number(value) < 1) fail('--timeout-seconds deve ser positivo');
      parsed.timeoutSeconds = Number(value);
    } else fail(`argumento desconhecido: ${arg}`);
    index += 1;
  }
  if (parsed.help) return parsed;
  for (const [key, flag] of [['baseline', 'baseline'], ['ledger', 'ledger'], ['oldLedger', 'old-ledger']]) {
    if (!parsed[key]) fail(`--${flag} é obrigatório`);
  }
  if (path.resolve(parsed.ledger) === path.resolve(parsed.oldLedger)) fail('o ledger antigo não pode ser o ledger do supervisor');
  return parsed;
}

export function supervisorArgs(parsed) {
  return [
    '--baseline', parsed.baseline,
    '--ledger', parsed.ledger,
    '--timeout-seconds', String(parsed.timeoutSeconds),
    '--',
    'cargo', 'test', '-p', 'previdenciar-web-server', '--locked', '--offline', '-j', '1',
  ];
}

export function oldLedgerHash(file) {
  try {
    return crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
  } catch {
    fail('ledger antigo ausente ou ilegível');
  }
}

function assertReadable(file, label) {
  if (!fs.existsSync(file)) fail(`${label} ausente`);
  if (!fs.statSync(file).isFile()) fail(`${label} não é arquivo`);
}

export async function runRunner(argv, { spawnImpl = spawn, env = process.env } = {}) {
  try {
    const parsed = parseRunnerArgs(argv);
    if (parsed.help) return { code: 0, stdout: `${USAGE}\n`, stderr: '' };
    assertReadable(parsed.baseline, 'baseline M1');
    assertReadable(parsed.ledger, 'ledger M1');
    assertReadable(parsed.oldLedger, 'ledger antigo');
    if (!fs.existsSync(parsed.repo)) fail('repo ausente');
    const url = parseDbUrl(env.TEST_DATABASE_URL ?? '');
    const before = oldLedgerHash(parsed.oldLedger);
    if (parsed.check) {
      return {
        code: 0,
        stdout: `${JSON.stringify({
          status: 'ready',
          budget: 'supervisor-4gib',
          oldLedgerSha256: before,
          databaseClass: 'previdenciar_test_*',
          port: url.port,
          cargo: 'not-executed',
        })}\n`,
        stderr: '',
      };
    }
    return await new Promise((resolve) => {
      const child = spawnImpl(process.execPath, [SUPERVISOR, ...supervisorArgs(parsed)], {
        cwd: parsed.repo,
        stdio: 'inherit',
        env: sanitizedEnv(env),
        windowsHide: true,
      });
      child.once('error', () => resolve({ code: 4, stdout: '', stderr: 'falha ao iniciar o supervisor\n' }));
      child.once('exit', (code, signal) => {
        const after = oldLedgerHash(parsed.oldLedger);
        if (after !== before) {
          resolve({ code: 5, stdout: '', stderr: 'o ledger antigo mudou durante a execução; resultado recusado\n' });
          return;
        }
        resolve({ code: signal ? 130 : (Number.isInteger(code) ? code : 4), stdout: '', stderr: '' });
      });
    });
  } catch (error) {
    const message = error instanceof SupervisorIntegrationError ? error.message : 'erro inesperado';
    return { code: 2, stdout: '', stderr: `test-integration-supervisor: ${message}\n` };
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
  const result = await runRunner(process.argv.slice(2));
  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  process.exitCode = result.code;
}
