#!/usr/bin/env node
/**
 * Contagem real de testes, por comando.
 *
 * O script pergunta às próprias ferramentas quantos testes existem, em vez de
 * manter números copiados em documentos. Ele pode ser executado da raiz do
 * repositório ou de `apps/desktop`.
 *
 * Uso:
 *   node scripts/count-tests.mjs            # tabela legível + resumo em texto
 *   node scripts/count-tests.mjs --json     # só o JSON ({ generatedAt, suites, totals })
 *   node scripts/count-tests.mjs --markdown # bloco pronto para colar no README
 *   node scripts/count-tests.mjs --only=rust,vitest
 *   node scripts/count-tests.mjs --run      # executa a suíte Rust para contar aprovados/ignorados
 *   node scripts/count-tests.mjs --skip-build  # não tenta compilar o frontend antes do cargo --list
 *
 * Suítes:
 *   rust       cargo test --workspace --locked -j 2 -- --list
 *   vitest     node apps/desktop/node_modules/vitest/vitest.mjs list --json
 *   playwright node apps/desktop/node_modules/@playwright/test/cli.js test --list
 *              (reserva com `npx --no-install` quando node_modules não existe)
 *
 * Particularidades do ambiente (documentadas em vez de escondidas):
 *   - `cargo ... --list` compila o workspace antes de listar. O build script do
 *     Tauri exige `apps/desktop/dist`; se a pasta não existir, o script compila o
 *     frontend uma vez (a não ser com --skip-build) e registra isso em `notes`
 *     da suíte rust.
 *   - Os CLIs do vitest e do Playwright são chamados direto pelo node a partir de
 *     `apps/desktop/node_modules`. Neste ambiente, `npx` e `npm.cmd` via execFile
 *     falharam com `spawn EINVAL`; a chamada direta evita shell, não baixa pacote
 *     da rede e não depende do PATH.
 *   - `--list` conta os testes, mas não diz quantos são ignorados; a coluna
 *     "execução real" só aparece com --run, porque ela roda a suíte Rust inteira.
 *   - `playwright test --list` não sobe o `webServer` da configuração nem abre
 *     navegador: só enumera. A execução real continua exigindo
 *     `npm run build` e `npx playwright install chromium`.
 *   - Quando uma listagem falha, a suíte entra em `suites` com `status: "failed"`,
 *     o motivo em `error` e a contagem fica `null`. O script nunca inventa número.
 *
 * Código de saída: 0 quando todas as suítes foram contadas; 1 se alguma falhou.
 */
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { execFile } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(SCRIPT_DIR, '..');
const DESKTOP_DIR = path.join(REPO_ROOT, 'apps', 'desktop');

const argv = process.argv.slice(2);
const asJson = argv.includes('--json');
const asMarkdown = argv.includes('--markdown');
const quiet = argv.includes('--quiet');
const skipBuild = argv.includes('--skip-build');
const rustRun = argv.includes('--run');
const onlyInline = argv.find((item) => item.startsWith('--only='));
const onlyIndex = argv.indexOf('--only');
const onlyMissingValue = onlyIndex >= 0 && (argv[onlyIndex + 1] === undefined || argv[onlyIndex + 1].startsWith('--'));
const onlyRaw =
  onlyInline !== undefined
    ? onlyInline.slice('--only='.length)
    : onlyIndex >= 0 && !onlyMissingValue
      ? argv[onlyIndex + 1]
      : null;
const only = onlyRaw
  ? onlyRaw
      .split(',')
      .map((item) => item.trim())
      .filter(Boolean)
  : null;

function log(message) {
  // O progresso vai para stderr para não contaminar --json/--markdown.
  // Fica em silêncio quando a saída é redirecionada, para não poluir pipes.
  if (quiet || asJson || asMarkdown) return;
  process.stderr.write(`${message}\n`);
}

function exists(target) {
  try {
    fs.accessSync(target);
    return true;
  } catch {
    return false;
  }
}

/** No Windows, npm e npx são .cmd; chamar o .cmd direto evita shell e o aviso DEP0190. */
function resolveCommand(command) {
  if (process.platform !== 'win32') return command;
  if (command === 'npm' || command === 'npx') return `${command}.cmd`;
  return command;
}

/**
 * Prefere o binário local instalado em apps/desktop/node_modules, executado
 * pelo próprio node. Evita depender do npx/.cmd do Windows (que falha com
 * spawn EINVAL em alguns ambientes) e não baixa pacote nenhum da rede.
 */
function localCli(relative) {
  const target = path.join(DESKTOP_DIR, 'node_modules', relative);
  return exists(target) ? target : null;
}

function runNodeScript(script, args, cwd = DESKTOP_DIR) {
  return run(process.execPath, [script, ...args], { cwd });
}

function run(command, args, options = {}) {
  return new Promise((resolve) => {
    const started = Date.now();
    execFile(
      resolveCommand(command),
      args,
      {
        cwd: options.cwd ?? REPO_ROOT,
        maxBuffer: 256 * 1024 * 1024,
        // Só o npm de reserva precisa de shell; o caminho normal não usa.
        shell: options.shell === true,
        timeout: options.timeoutMs ?? 0,
      },
      (error, stdout, stderr) => {
        resolve({
          ok: !error,
          code: error?.code ?? 0,
          stdout: String(stdout ?? ''),
          stderr: String(stderr ?? ''),
          elapsedMs: Date.now() - started,
        });
      },
    );
  });
}

/* ------------------------------------------------------------------------- */
/* Rust                                                                      */
/* ------------------------------------------------------------------------- */

function parseCargoList(stdout) {
  // Formato: "caminho::modulo::teste: test" e, para benchmarks, ": benchmark".
  const tests = [];
  const benchmarks = [];
  for (const rawLine of stdout.split(/\r?\n/)) {
    const line = rawLine.trimEnd();
    if (!/: (test|benchmark)$/.test(line)) continue;
    if (/: benchmark$/.test(line)) benchmarks.push(line.replace(/: benchmark$/, ''));
    else tests.push(line);
  }
  return { tests, benchmarks };
}

/**
 * Soma os blocos "test result:" de uma execução real do cargo test.
 * Usado com --run, porque `--list` não informa quantos testes são ignorados.
 */
function parseCargoTotals(stdout) {
  // "test result: ok. 643 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 1.2s"
  const totals = { passed: 0, failed: 0, ignored: 0, filteredOut: 0, binaries: 0 };
  const regex = /test result: [^\n]*?(\d+) passed; (\d+) failed; (\d+) ignored; \d+ measured; (\d+) filtered out/g;
  let found;
  while ((found = regex.exec(stdout)) !== null) {
    totals.binaries += 1;
    totals.passed += Number(found[1]);
    totals.failed += Number(found[2]);
    totals.ignored += Number(found[3]);
    totals.filteredOut += Number(found[4]);
  }
  return totals;
}

/**
 * Build do frontend sem passar pelo npm: executa o tsc e o vite locais com o
 * próprio node. Motivo: neste ambiente, `npm`/`npm.cmd` via execFile falha com
 * `spawn EINVAL`; chamar os CLIs por node evita shell e funciona igual no CI.
 * Em último caso, tenta o npm com shell (comportamento antigo).
 */
async function buildFrontend() {
  const tsc = localCli(path.join('typescript', 'bin', 'tsc'));
  const vite = localCli(path.join('vite', 'bin', 'vite.js'));
  if (!tsc || !vite) {
    return run('npm', ['run', 'build'], { cwd: DESKTOP_DIR, shell: true });
  }
  const typecheck = await runNodeScript(tsc, ['-b']);
  if (!typecheck.ok) return { ...typecheck, ok: false };
  return runNodeScript(vite, ['build']);
}

async function countRust() {
  const notes = [];
  if (!exists(path.join(DESKTOP_DIR, 'dist')) && !skipBuild) {
    notes.push('apps/desktop/dist ausente: compilado o frontend antes de listar, porque o build script do Tauri exige a pasta.');
    log('… dist ausente, compilando o frontend para permitir o cargo --list');
    const build = await buildFrontend();
    if (!build.ok) {
      notes.push(`o build do frontend falhou (${firstLine(build.stderr) || firstLine(build.stdout)}); a listagem pode falhar.`);
    }
  } else if (!exists(path.join(DESKTOP_DIR, 'dist')) && skipBuild) {
    notes.push('apps/desktop/dist ausente e --skip-build informado: o build script do Tauri provavelmente fará a listagem falhar.');
  }

  const result = await run('cargo', [
    'test',
    '--workspace',
    '--locked',
    '-j',
    '2',
    '--',
    '--list',
  ]);
  if (!result.ok) {
    return {
      id: 'rust',
      command: 'cargo test --workspace --locked -j 2 -- --list',
      status: 'failed',
      count: null,
      error: firstLine(result.stderr) || firstLine(result.stdout) || `cargo saiu com código ${result.code}`,
      hint: 'Confirme que apps/desktop/dist existe (npm run build) e que o workspace compila.',
      notes,
    };
  }
  const { tests, benchmarks } = parseCargoList(result.stdout);
  const entry = {
    id: 'rust',
    command: 'cargo test --workspace --locked -j 2 -- --list',
    status: 'ok',
    count: tests.length,
    benchmarks: benchmarks.length,
    elapsedMs: result.elapsedMs,
    notes,
  };

  // `--list` não diz quantos testes são ignorados. Só uma execução real responde,
  // e ela é opcional porque roda a suíte inteira.
  if (rustRun) {
    log('… executando a suíte Rust para contar aprovados, falhas e ignorados');
    const runResult = await run('cargo', ['test', '--workspace', '--locked', '-j', '2'], {
      timeoutMs: 0,
    });
    const totals = parseCargoTotals(runResult.stdout + runResult.stderr);
    entry.execution = {
      command: 'cargo test --workspace --locked -j 2',
      status: runResult.ok ? 'ok' : 'failed',
      passed: totals.passed,
      failed: totals.failed,
      ignored: totals.ignored,
      binaries: totals.binaries,
      elapsedMs: runResult.elapsedMs,
    };
    if (!runResult.ok && !totals.binaries) {
      entry.execution.error =
        firstLine(runResult.stderr) || firstLine(runResult.stdout) || `cargo saiu com código ${runResult.code}`;
    }
  }

  return entry;
}

/* ------------------------------------------------------------------------- */
/* Vitest                                                                    */
/* ------------------------------------------------------------------------- */

const VITEST_CLI = 'apps/desktop/node_modules/vitest/vitest.mjs';
const PLAYWRIGHT_CLI = 'apps/desktop/node_modules/@playwright/test/cli.js';

async function countVitest() {
  const cli = localCli(path.join('vitest', 'vitest.mjs'));
  const command = cli ? `node ${VITEST_CLI} list --json` : 'npx --no-install vitest list --json';
  const result = cli
    ? await runNodeScript(cli, ['list', '--json'])
    : await run('npx', ['--no-install', 'vitest', 'list', '--json'], { cwd: DESKTOP_DIR });
  if (!result.ok) {
    return {
      id: 'vitest',
      command,
      status: 'failed',
      count: null,
      error: firstLine(result.stderr) || firstLine(result.stdout) || `vitest saiu com código ${result.code}`,
      hint: 'Execute "npm ci" em apps/desktop antes de contar.',
      notes: [],
    };
  }
  // `vitest list --json` imprime um array; algumas versões prefixam avisos.
  const start = result.stdout.indexOf('[');
  const end = result.stdout.lastIndexOf(']');
  if (start === -1 || end === -1) {
    return {
      id: 'vitest',
      command,
      status: 'failed',
      count: null,
      error: 'a saída de "vitest list --json" não contém um array JSON',
      hint: 'Rode "npx vitest list" sem --json para ver o formato atual.',
      notes: [],
    };
  }
  let entries;
  try {
    entries = JSON.parse(result.stdout.slice(start, end + 1));
  } catch (error) {
    return {
      id: 'vitest',
      command,
      status: 'failed',
      count: null,
      error: `JSON inválido: ${error.message}`,
      hint: 'Rode "npx vitest list" sem --json para ver o formato atual.',
      notes: [],
    };
  }
  const files = new Set(entries.map((entry) => entry.file).filter(Boolean));
  return {
    id: 'vitest',
    command,
    status: 'ok',
    count: entries.length,
    files: files.size,
    elapsedMs: result.elapsedMs,
    notes: [],
  };
}

/* ------------------------------------------------------------------------- */
/* Playwright                                                                */
/* ------------------------------------------------------------------------- */

function parsePlaywrightList(stdout) {
  // Última linha: "Total: 38 tests in 11 files"
  const totals = [...stdout.matchAll(/Total:\s+(\d+)\s+tests?\s+in\s+(\d+)\s+files?/g)].pop();
  if (totals) return { count: Number(totals[1]), files: Number(totals[2]) };
  // Fallback: conta linhas "  arquivo.spec.ts:12:1 › título"
  const lines = stdout.split(/\r?\n/).filter((line) => /^\s{2}\S+\.(?:spec|test)\.[cm]?[jt]sx?:\d+:\d+\s+›/.test(line));
  const files = new Set(lines.map((line) => line.trim().split(':')[0]));
  if (!lines.length) return null;
  return { count: lines.length, files: files.size };
}

async function countPlaywright() {
  const cli = localCli(path.join('@playwright', 'test', 'cli.js'));
  const command = cli ? `node ${PLAYWRIGHT_CLI} test --list` : 'npx --no-install playwright test --list';
  const result = cli
    ? await runNodeScript(cli, ['test', '--list'])
    : await run('npx', ['--no-install', 'playwright', 'test', '--list'], { cwd: DESKTOP_DIR });
  if (!result.ok) {
    const missingBrowser = /Executable doesn't exist|playwright install/i.test(result.stderr + result.stdout);
    return {
      id: 'playwright',
      command,
      status: 'failed',
      count: null,
      error: firstLine(result.stderr) || firstLine(result.stdout) || `playwright saiu com código ${result.code}`,
      hint: missingBrowser
        ? 'Instale o navegador com "npx playwright install chromium"; a listagem não abre navegador, mas a configuração pode exigir.'
        : 'Confirme "npm ci" e "npx playwright install chromium" em apps/desktop.',
      notes: [],
    };
  }
  const parsed = parsePlaywrightList(result.stdout);
  if (!parsed) {
    return {
      id: 'playwright',
      command,
      status: 'failed',
      count: null,
      error: 'a saída de "playwright test --list" não trouxe a linha "Total: N tests in M files"',
      hint: 'Rode "npx playwright test --list" e confira o formato do relator.',
      notes: [],
    };
  }
  return {
    id: 'playwright',
    command,
    status: 'ok',
    count: parsed.count,
    files: parsed.files,
    elapsedMs: result.elapsedMs,
    notes: [],
  };
}

/* ------------------------------------------------------------------------- */

function firstLine(text) {
  const line = String(text ?? '')
    .split(/\r?\n/)
    .map((item) => item.trim())
    .find(Boolean);
  return line ?? '';
}

const SUITES = [
  { id: 'rust', title: 'Rust', run: countRust },
  { id: 'vitest', title: 'Interface (vitest)', run: countVitest },
  { id: 'playwright', title: 'Navegador (Playwright)', run: countPlaywright },
];

const HELP_TEXT = `Contagem real de testes, por comando.

Uso (da raiz do repositório ou de apps/desktop):
  node scripts/count-tests.mjs                tabela legível
  node scripts/count-tests.mjs --markdown     bloco pronto para o README
  node scripts/count-tests.mjs --json         JSON com suites e totais
  node scripts/count-tests.mjs --only=vitest  só uma suíte (rust, vitest, playwright)
  node scripts/count-tests.mjs --run          também executa a suíte Rust para contar
                                              aprovados, falhas e ignorados
  node scripts/count-tests.mjs --skip-build   não compila o frontend antes do cargo --list
  node scripts/count-tests.mjs --quiet        sem mensagens de progresso

Comandos executados:
  cargo test --workspace --locked -j 2 -- --list     (contagem de testes Rust)
  cargo test --workspace --locked -j 2               (só com --run: aprovados/ignorados)
  node apps/desktop/node_modules/vitest/vitest.mjs list --json
  node apps/desktop/node_modules/@playwright/test/cli.js test --list

Código de saída: 0 quando tudo foi contado; 1 quando alguma listagem falhou.`;

async function main() {
  if (argv.includes('--help') || argv.includes('-h')) {
    console.log(HELP_TEXT);
    return;
  }
  if (onlyMissingValue) {
    console.error('Falta o valor de --only. Use --only=rust, --only=vitest, --only=playwright ou combine com vírgula.');
    process.exitCode = 1;
    return;
  }
  const selected = only ? SUITES.filter((suite) => only.includes(suite.id)) : SUITES;
  if (only && !selected.length) {
    console.error(`Nenhuma suíte reconhecida em --only=${only.join(',')}. Use: rust, vitest, playwright.`);
    process.exitCode = 1;
    return;
  }

  log('Contando testes com as próprias ferramentas. O cargo --list compila o workspace e pode levar alguns minutos.');
  const suites = [];
  for (const suite of selected) {
    log(`… ${suite.title}`);
    const result = await suite.run();
    suites.push({ title: suite.title, ...result });
  }

  const okSuites = suites.filter((suite) => suite.status === 'ok');
  const payload = {
    generatedAt: new Date().toISOString(),
    repository: REPO_ROOT,
    suites,
    totals: {
      suitesCounted: okSuites.length,
      suitesFailed: suites.length - okSuites.length,
      tests: okSuites.reduce((sum, suite) => sum + suite.count, 0),
    },
  };

  if (asJson) {
    console.log(JSON.stringify(payload, null, 2));
  } else if (asMarkdown) {
    console.log(toMarkdown(payload));
  } else {
    console.log(toTable(payload));
  }
  if (payload.totals.suitesFailed) process.exitCode = 1;
}

function formatDate(iso) {
  const date = new Date(iso);
  return date.toLocaleString('pt-BR', {
    dateStyle: 'short',
    timeStyle: 'short',
    timeZone: 'America/Sao_Paulo',
  });
}

function suiteLabel(suite) {
  const extra = [];
  if (typeof suite.files === 'number') extra.push(`${suite.files} arquivos`);
  if (typeof suite.benchmarks === 'number' && suite.benchmarks) extra.push(`${suite.benchmarks} benchmarks`);
  if (typeof suite.elapsedMs === 'number' && suite.elapsedMs > 1000) extra.push(`${Math.round(suite.elapsedMs / 1000)} s`);
  if (suite.status === 'failed') extra.push('falhou');
  return extra.join(' · ');
}

/** "643 passaram · 2 ignorados · 0 falharam", quando a execução real rodou. */
function executionLabel(suite) {
  if (!suite.execution) return null;
  const result = suite.execution;
  const parts = [
    `${result.passed} passaram`,
    `${result.ignored} ignorados`,
    `${result.failed} falharam`,
  ];
  return `execução real: ${parts.join(' · ')}${result.status === 'failed' ? ' (a suíte falhou)' : ''}`;
}

function toTable(payload) {
  const lines = [];
  lines.push('Contagem real de testes');
  lines.push(`Gerada em ${formatDate(payload.generatedAt)} por scripts/count-tests.mjs`);
  lines.push('');
  const width = Math.max(...payload.suites.map((suite) => suite.title.length), 10);
  for (const suite of payload.suites) {
    const count = suite.status === 'ok' ? String(suite.count).padStart(6) : '   n/d';
    const label = suiteLabel(suite);
    lines.push(`${suite.title.padEnd(width)}  ${count}${label ? `  ${label}` : ''}`);
    if (suite.status === 'failed') {
      lines.push(`${' '.repeat(width)}  motivo: ${suite.error}`);
      if (suite.hint) lines.push(`${' '.repeat(width)}  alternativa: ${suite.hint}`);
    }
    for (const note of suite.notes ?? []) lines.push(`${' '.repeat(width)}  nota: ${note}`);
    const execution = executionLabel(suite);
    if (execution) lines.push(`${' '.repeat(width)}  ${execution}`);
  }
  lines.push('');
  lines.push(`Total contado: ${payload.totals.tests} testes em ${payload.totals.suitesCounted} suíte(s).`);
  if (payload.totals.suitesFailed) {
    lines.push(`Suítes com falha na listagem: ${payload.totals.suitesFailed}. Nenhum número foi inventado para elas.`);
  }
  return lines.join('\n');
}

function toMarkdown(payload) {
  const byId = new Map(payload.suites.map((suite) => [suite.id, suite]));
  const cell = (id) => {
    const suite = byId.get(id);
    if (!suite) return null;
    return suite.status === 'ok' ? String(suite.count) : 'não contado';
  };
  // Quando houve execução real, o README pode citar aprovados e ignorados.
  const rustSuite = byId.get('rust');
  const rustDetail =
    rustSuite?.execution && rustSuite.execution.status === 'ok'
      ? ` (${rustSuite.execution.passed} passaram, ${rustSuite.execution.ignored} ignorados)`
      : '';
  const parts = [
    ['rust', `testes Rust (workspace completo)${rustDetail}`],
    ['vitest', 'testes de interface'],
    ['playwright', 'testes de navegador'],
  ]
    .map(([id, label]) => {
      const value = cell(id);
      return value === null ? null : `${value} ${label}`;
    })
    .filter(Boolean);

  const lines = [];
  lines.push(`- ${parts.join('; ')}.`);
  lines.push('');
  lines.push(
    `Contagem gerada por \`node scripts/count-tests.mjs\` em ${formatDate(payload.generatedAt)}. ` +
      'A fonte oficial do número é o script, não este texto: rode-o de novo depois de adicionar testes.',
  );
  if (payload.totals.suitesFailed) {
    lines.push('');
    lines.push(
      `Atenção: ${payload.totals.suitesFailed} suíte(s) não puderam ser contadas nesta execução. ` +
        'O número correspondente ficou como "não contado".',
    );
  }
  return lines.join('\n');
}

main().catch((error) => {
  console.error(`Falha inesperada ao contar testes: ${error?.message ?? error}`);
  process.exitCode = 1;
});
