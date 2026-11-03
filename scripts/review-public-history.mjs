#!/usr/bin/env node
/**
 * Revisão de fonte pública no histórico alcançável.
 *
 * Uso:
 *   node scripts/review-public-history.mjs --history [--rev <range>] [--strict]
 *
 * Varre TODOS os commits alcançáveis pela revisão indicada (padrão: HEAD),
 * incluindo merges e todos os pais, e todas as associações path+blob de cada
 * árvore. Não usa `git rev-list --objects` como fonte de alcance: ele mostra
 * apenas o primeiro path de um blob e não serve para exceções por caminho.
 *
 * O relatório é JSON sanitizado: achados trazem commit, path, linha, regra e
 * excerto redigido; valores brutos não são impressos. `allowed` é um resumo
 * por arquivo+regra, sem valores, para reconciliar EXCECOES.md.
 *
 * Cobertura: blobs acima de --max-blob-bytes (5 MiB), symlinks, gitlinks,
 * LFS, erros de leitura e limites de total tornam `coverage.complete=false` e
 * o status `blocked`. Não há selo de "scan completo" com omissão.
 *
 * Segurança: só leitura; sem rede, submódulo, LFS fetch, shell, escrita de
 * índice/refs/HEAD ou alteração de arquivos. O scan cobre o histórico local
 * alcançável por HEAD; refs remotas podem estar velhas e não são buscadas.
 */
import path from 'node:path';
import process from 'node:process';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { classifyPath, scanText } from './review-public-source.mjs';

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = path.resolve(SCRIPT_DIR, '..');
const DEFAULT_MAX_BLOB_BYTES = 5 * 1024 * 1024;
const DEFAULT_MAX_TOTAL_BLOB_BYTES = 100 * 1024 * 1024;
const DEFAULT_MAX_ASSOCIATIONS = 250_000;
const DEFAULT_MAX_FINDINGS = 5_000;

const USAGE = `Revisão de fonte pública no histórico

Uso:
  node scripts/review-public-history.mjs --history [--rev <range>] [--strict]

  --history       varre o histórico alcançável (obrigatório)
  --rev <range>   revisão/intervalo (padrão: HEAD)
  --strict        ignora a allowlist de exceções
  --help          mostra esta ajuda

Saída: JSON com historyFindings, commitsScanned, allowed, coverage e status.
Código: 0 passed; 1 blocked/erro. Nunca imprime valor bruto de segredo.
Só lê o repositório; não faz fetch, checkout, gc ou escrita.`;

export class HistoryError extends Error {
  constructor(message) {
    super(message);
    this.name = 'HistoryError';
  }
}

function fail(message) {
  throw new HistoryError(message);
}

function git(repo, args, { encoding = 'utf8', maxBuffer = 64 * 1024 * 1024 } = {}) {
  try {
    return execFileSync('git', ['-C', repo, ...args], {
      encoding,
      maxBuffer,
      windowsHide: true,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
  } catch (error) {
    fail(`git ${args[0] ?? ''} falhou; revisão ou repositório não pôde ser lido`);
  }
}

function validateRev(rev) {
  if (typeof rev !== 'string' || !rev.trim()) fail('--rev exige uma revisão');
  const value = rev.trim();
  if (value.length > 200) fail('--rev longo demais');
  if (value.startsWith('-')) fail('--rev não pode começar com hífen');
  if (!/^[A-Za-z0-9._/@{}^~:+-]+$/.test(value)) fail('--rev contém caractere não permitido');
  return value;
}

function parseArgs(argv) {
  const parsed = { history: false, help: false, rev: 'HEAD', strict: false };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === '--history') parsed.history = true;
    else if (arg === '--strict') parsed.strict = true;
    else if (arg === '--help' || arg === '-h') parsed.help = true;
    else if (arg === '--rev') {
      parsed.rev = argv[index + 1];
      index += 1;
      if (parsed.rev === undefined) fail('--rev exige um valor');
    } else {
      fail(`argumento desconhecido: ${arg}`);
    }
  }
  return parsed;
}

function shortHash(hash) {
  return hash.slice(0, 12);
}

function parseTreeEntries(buffer) {
  const entries = [];
  const text = buffer.toString('utf8');
  for (const raw of text.split('\0')) {
    if (!raw) continue;
    const match = /^(\d{6}) (blob|commit) ([0-9a-f]+) +(\d+|-)\t(.*)$/.exec(raw);
    if (!match) fail('entrada de árvore em formato inesperado');
    entries.push({
      mode: match[1],
      type: match[2],
      oid: match[3],
      size: match[4] === '-' ? 0 : Number(match[4]),
      path: match[5],
    });
  }
  return entries;
}

function addFinding(map, finding) {
  const key = [
    finding.commit,
    finding.path,
    finding.rule,
    finding.line,
    finding.excerpt,
  ].join('\u0000');
  const existing = map.get(key);
  if (existing) existing.occurrences += 1;
  else map.set(key, { ...finding, occurrences: 1 });
}

function addLimitation(coverage, category, count = 1) {
  coverage.limitations.push({ category, count });
}

/**
 * Executa a varredura. Exportado para testes; `repo` é usado apenas pelo teste
 * sintético e nunca exposto na CLI pública.
 */
export function runHistoryScan({
  repo = REPO_ROOT,
  rev = 'HEAD',
  strict = false,
  maxBlobBytes = DEFAULT_MAX_BLOB_BYTES,
  maxTotalBlobBytes = DEFAULT_MAX_TOTAL_BLOB_BYTES,
  maxAssociations = DEFAULT_MAX_ASSOCIATIONS,
  maxFindings = DEFAULT_MAX_FINDINGS,
} = {}) {
  const repository = path.resolve(repo);
  const revision = validateRev(rev);
  if (!Number.isSafeInteger(maxBlobBytes) || maxBlobBytes <= 0) fail('maxBlobBytes inválido');
  if (!Number.isSafeInteger(maxTotalBlobBytes) || maxTotalBlobBytes <= 0) fail('maxTotalBlobBytes inválido');
  if (!Number.isSafeInteger(maxAssociations) || maxAssociations <= 0) fail('maxAssociations inválido');
  if (!Number.isSafeInteger(maxFindings) || maxFindings <= 0) fail('maxFindings inválido');

  const startedAt = new Date().toISOString();
  const head = String(git(repository, ['rev-parse', '--verify', 'HEAD'])).trim();
  const commitsText = String(
    git(repository, ['rev-list', '--reverse', '--end-of-options', revision]),
  ).trim();
  const commits = commitsText ? commitsText.split(/\r?\n/).filter(Boolean) : [];
  if (!commits.length) fail('revisão sem commits');

  const coverage = {
    complete: true,
    commitsExpected: commits.length,
    commitsScanned: 0,
    paths: 0,
    associations: 0,
    uniqueBlobs: 0,
    uniqueBlobBytes: 0,
    scannedBlobs: 0,
    scannedBytes: 0,
    binaryBlobsScanned: 0,
    contentPolicyScanned: 0,
    lfsPointers: 0,
    skippedOversize: 0,
    skippedSymlink: 0,
    skippedGitlink: 0,
    skippedType: 0,
    readErrors: 0,
    limitations: [],
  };

  const associations = new Map();
  const pathFindings = new Map();
  const pathSet = new Set();
  const blobSizes = new Map();
  let truncated = false;

  for (let index = 0; index < commits.length; index += 1) {
    const commit = commits[index];
    coverage.commitsScanned += 1;
    const tree = git(repository, ['ls-tree', '-r', '-l', '-z', commit], {
      encoding: 'buffer',
    });
    const entries = parseTreeEntries(tree);
    for (const entry of entries) {
      pathSet.add(entry.path);
      if (entry.mode === '160000' || entry.type === 'commit') {
        coverage.skippedGitlink += 1;
        coverage.complete = false;
        addLimitation(coverage, 'gitlink/submodule', 1);
        continue;
      }
      if (entry.mode === '120000') {
        coverage.skippedSymlink += 1;
        coverage.complete = false;
        addLimitation(coverage, 'symlink', 1);
        continue;
      }
      if (entry.type !== 'blob') {
        coverage.skippedType += 1;
        coverage.complete = false;
        addLimitation(coverage, 'tipo de entrada não suportado', 1);
        continue;
      }
      if (entry.size > maxBlobBytes) {
        coverage.skippedOversize += 1;
        coverage.complete = false;
        addLimitation(coverage, 'blob acima de maxBlobBytes', 1);
        continue;
      }
      blobSizes.set(entry.oid, entry.size);

      const key = `${entry.path}\u0000${entry.oid}`;
      const existing = associations.get(key);
      if (existing) {
        existing.commits.add(index);
      } else {
        if (associations.size >= maxAssociations) fail('limite de associações path+blob excedido');
        associations.set(key, {
          path: entry.path,
          oid: entry.oid,
          size: entry.size,
          firstIndex: index,
          commits: new Set([index]),
        });
      }

      const pathRule = classifyPath(entry.path);
      if (pathRule?.rule) {
        const pathKey = `${entry.path}\u0000${pathRule.rule}`;
        const pathFinding = pathFindings.get(pathKey);
        if (pathFinding) {
          pathFinding.commits.add(index);
          pathFinding.blobs.add(entry.oid);
        } else {
          pathFindings.set(pathKey, {
            commit: shortHash(commit),
            commitIndex: index,
            commitsFromHead: commits.length - 1 - index,
            path: entry.path,
            rule: pathRule.rule,
            reason: pathRule.reason,
            hint: pathRule.hint,
            excerpt: '[caminho privado/gerado; conteúdo não exibido]',
            commits: new Set([index]),
            blobs: new Set([entry.oid]),
          });
        }
      }
    }
  }

  const uniqueBlobBytes = [...blobSizes.values()].reduce((sum, size) => sum + size, 0);
  coverage.paths = pathSet.size;
  coverage.associations = associations.size;
  coverage.uniqueBlobs = blobSizes.size;
  coverage.uniqueBlobBytes = uniqueBlobBytes;
  if (uniqueBlobBytes > maxTotalBlobBytes) {
    coverage.complete = false;
    addLimitation(coverage, 'total de blobs acima de maxTotalBlobBytes', 1);
    truncated = true;
  }

  const findings = new Map();
  const allowedCounts = new Map();
  for (const association of pathFindings.values()) {
    addFinding(findings, {
      commit: association.commit,
      commitIndex: association.commitIndex,
      commitsFromHead: association.commitsFromHead,
      path: association.path,
      line: 0,
      rule: association.rule,
      reason: association.reason,
      hint: association.hint,
      excerpt: association.excerpt,
      commits: association.commits.size,
      blobs: association.blobs.size,
    });
  }

  if (!truncated) {
    const blobs = new Map();
    for (const association of associations.values()) {
      const existing = blobs.get(association.oid);
      if (existing) existing.paths.add(association.path);
      else blobs.set(association.oid, { size: association.size, paths: new Set([association.path]) });
    }

    for (const [oid, blob] of blobs) {
      let buffer;
      try {
        buffer = git(repository, ['cat-file', 'blob', oid], {
          encoding: 'buffer',
          maxBuffer: Math.max(maxBlobBytes + 1, 8 * 1024 * 1024),
        });
      } catch {
        coverage.readErrors += 1;
        coverage.complete = false;
        addLimitation(coverage, 'erro ao ler blob', 1);
        continue;
      }
      coverage.scannedBlobs += 1;
      coverage.scannedBytes += buffer.length;
      if (buffer.includes(0)) coverage.binaryBlobsScanned += 1;
      const text = buffer.toString('utf8');
      if (text.startsWith('version https://git-lfs.github.com/spec/v1')) {
        coverage.lfsPointers += 1;
        coverage.complete = false;
        addLimitation(coverage, 'ponteiro LFS', 1);
      }
      for (const filePath of blob.paths) {
        const pathRule = classifyPath(filePath);
        if (pathRule?.skipContent) coverage.contentPolicyScanned += 1;
        const result = scanText(text, strict ? { file: filePath, allowlist: [] } : { file: filePath });
        for (const finding of result.findings) {
          const association = associations.get(`${filePath}\u0000${oid}`);
          addFinding(findings, {
            commit: shortHash(commits[association.firstIndex]),
            commitIndex: association.firstIndex,
            commitsFromHead: commits.length - 1 - association.firstIndex,
            path: filePath,
            line: finding.line,
            rule: finding.rule,
            reason: finding.reason,
            hint: finding.hint,
            excerpt: finding.excerpt,
            commits: association.commits.size,
            blobs: 1,
          });
        }
        for (const allowed of result.allowed) {
          const key = `${allowed.file}\u0000${allowed.rule}`;
          const existing = allowedCounts.get(key);
          if (existing) existing.count += 1;
          else allowedCounts.set(key, { file: allowed.file, rule: allowed.rule, count: 1 });
        }
        if (findings.size > maxFindings) {
          coverage.complete = false;
          addLimitation(coverage, 'limite de achados excedido', 1);
          break;
        }
      }
      if (findings.size > maxFindings) break;
    }
  }

  const historyFindings = [...findings.values()].sort((a, b) =>
    a.commitIndex - b.commitIndex || a.path.localeCompare(b.path) || a.line - b.line || a.rule.localeCompare(b.rule),
  );
  const allowed = [...allowedCounts.values()].sort((a, b) =>
    a.file.localeCompare(b.file) || a.rule.localeCompare(b.rule),
  );
  const status = historyFindings.length || !coverage.complete ? 'blocked' : 'passed';
  return {
    status,
    exitCode: status === 'passed' ? 0 : 1,
    rev: revision,
    head,
    strict,
    commitsScanned: coverage.commitsScanned,
    historyFindings,
    allowed,
    coverage,
    startedAt,
    finishedAt: new Date().toISOString(),
  };
}

export function runCli(argv) {
  try {
    const parsed = parseArgs(argv);
    if (parsed.help) return { code: 0, stdout: `${USAGE}\n`, stderr: '' };
    if (!parsed.history) fail('use --history');
    const result = runHistoryScan({ rev: parsed.rev, strict: parsed.strict });
    return { code: result.exitCode, stdout: `${JSON.stringify(result, null, 2)}\n`, stderr: '' };
  } catch (error) {
    const message = error instanceof HistoryError ? error.message : 'erro inesperado ao varrer o histórico';
    const result = {
      status: 'error',
      exitCode: 1,
      historyFindings: [],
      allowed: [],
      commitsScanned: 0,
      coverage: { complete: false, limitations: [{ category: message, count: 1 }] },
      error: message,
    };
    return { code: 1, stdout: `${JSON.stringify(result, null, 2)}\n`, stderr: `history-scan: ${message}\n` };
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
  const result = runCli(process.argv.slice(2));
  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  process.exitCode = result.code;
}
