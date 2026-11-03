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
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import crypto from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { ALLOWLIST, classifyPath, redactText, scanText } from './review-public-source.mjs';

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
    return execFileSync('git', ['--no-optional-locks', '-C', repo, ...args], {
      encoding,
      maxBuffer,
      windowsHide: true,
      stdio: ['ignore', 'pipe', 'pipe'],
      env: {
        ...process.env,
        GIT_NO_LAZY_FETCH: '1',
        GIT_NO_REPLACE_OBJECTS: '1',
      },
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

function sha256Buffer(buffer) {
  return crypto.createHash('sha256').update(buffer).digest('hex');
}

export function parseTreeEntries(buffer) {
  const entries = [];
  let start = 0;
  while (start < buffer.length) {
    const end = buffer.indexOf(0, start);
    if (end === -1) fail('entrada de árvore sem terminador NUL');
    if (end === start) {
      start = end + 1;
      continue;
    }
    const raw = buffer.subarray(start, end);
    const tab = raw.indexOf(9);
    if (tab < 0) fail('entrada de árvore sem separador de path');
    const header = raw.subarray(0, tab).toString('ascii');
    const match = /^(\d{6}) (blob|commit) ([0-9a-f]+) +(\d+|-)$/.exec(header);
    if (!match) fail('entrada de árvore em formato inesperado');
    const pathBytes = Buffer.from(raw.subarray(tab + 1));
    let pathText = null;
    let pathValid = true;
    try {
      pathText = new TextDecoder('utf-8', { fatal: true }).decode(pathBytes);
      if (!Buffer.from(pathText, 'utf8').equals(pathBytes)) pathValid = false;
    } catch {
      pathValid = false;
    }
    entries.push({
      mode: match[1],
      type: match[2],
      oid: match[3],
      size: match[4] === '-' ? 0 : Number(match[4]),
      pathBytes,
      pathText,
      pathValid,
      pathHash: sha256Buffer(pathBytes),
    });
    start = end + 1;
  }
  return entries;
}

function displayPath(pathText) {
  const pathScan = scanText(pathText, { file: '<history-path>', allowlist: [] });
  if (!pathScan.findings.length) {
    return {
      text: pathText,
      redacted: false,
      rules: [],
      complete: true,
      findings: [],
    };
  }
  const redacted = redactText(pathText);
  return {
    text: redacted.complete ? redacted.text : '[REDACTADO:path]',
    redacted: true,
    rules: redacted.rules,
    complete: redacted.complete,
    findings: pathScan.findings,
  };
}

function addFinding(map, finding) {
  const key = [
    finding.commit,
    finding.pathHash ?? '',
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

function gitMaybe(repo, args) {
  try {
    return String(git(repo, args)).trim();
  } catch {
    return null;
  }
}

function detectHistoryControls(repository) {
  const controls = {
    shallow: false,
    replaceRefs: 0,
    grafts: false,
    partialClone: false,
    promisor: false,
  };
  controls.shallow = gitMaybe(repository, ['rev-parse', '--is-shallow-repository']) === 'true';
  const refs = gitMaybe(repository, ['for-each-ref', 'refs/replace', '--format=%(refname)']);
  controls.replaceRefs = refs ? refs.split(/\r?\n/).filter(Boolean).length : 0;

  const graftPath = gitMaybe(repository, ['rev-parse', '--git-path', 'info/grafts']);
  if (graftPath) {
    const resolved = path.isAbsolute(graftPath) ? graftPath : path.resolve(repository, graftPath);
    controls.grafts = fs.existsSync(resolved);
  }
  const partial = gitMaybe(repository, ['config', '--get', 'extensions.partialClone']);
  const filter = gitMaybe(repository, ['config', '--get', 'remote.origin.partialclonefilter']);
  controls.partialClone = Boolean((partial && partial.length) || (filter && filter.length));
  controls.promisor = gitMaybe(repository, ['config', '--get', 'remote.origin.promisor']) === 'true';
  return controls;
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
  maxAllowedEntries = 2_000,
} = {}) {
  const repository = path.resolve(repo);
  const revision = validateRev(rev);
  if (!Number.isSafeInteger(maxBlobBytes) || maxBlobBytes <= 0) fail('maxBlobBytes inválido');
  if (!Number.isSafeInteger(maxTotalBlobBytes) || maxTotalBlobBytes <= 0) fail('maxTotalBlobBytes inválido');
  if (!Number.isSafeInteger(maxAssociations) || maxAssociations <= 0) fail('maxAssociations inválido');
  if (!Number.isSafeInteger(maxFindings) || maxFindings <= 0) fail('maxFindings inválido');
  if (!Number.isSafeInteger(maxAllowedEntries) || maxAllowedEntries <= 0) fail('maxAllowedEntries inválido');

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
    invalidPaths: 0,
    historyControls: detectHistoryControls(repository),
    limitations: [],
  };
  if (coverage.historyControls.shallow) {
    coverage.complete = false;
    addLimitation(coverage, 'repositório shallow: histórico limitado', 1);
  }
  if (coverage.historyControls.replaceRefs > 0) {
    coverage.complete = false;
    addLimitation(coverage, 'refs/replace presentes: histórico substituído', coverage.historyControls.replaceRefs);
  }
  if (coverage.historyControls.grafts) {
    coverage.complete = false;
    addLimitation(coverage, 'grafts presentes: histórico alterado', 1);
  }
  if (coverage.historyControls.partialClone || coverage.historyControls.promisor) {
    coverage.complete = false;
    addLimitation(coverage, 'clone parcial/promisor: objetos podem faltar', 1);
  }

  const associations = new Map();
  const pathFindings = new Map();
  const pathDisplays = new Map();
  const pathSet = new Set();
  const blobSizes = new Map();
  let truncated = false;

  function displayFor(entry) {
    const cached = pathDisplays.get(entry.pathHash);
    if (cached) return cached;
    const display = displayPath(entry.pathText);
    if (!display.complete) {
      coverage.complete = false;
      addLimitation(coverage, 'redação de path incompleta', 1);
    }
    const result = {
      ...display,
      pathHash: entry.pathHash,
      pathBytes: entry.pathBytes.length,
    };
    pathDisplays.set(entry.pathHash, result);
    return result;
  }

  for (let index = 0; index < commits.length; index += 1) {
    const commit = commits[index];
    coverage.commitsScanned += 1;
    const tree = git(repository, ['ls-tree', '-r', '-l', '-z', commit], {
      encoding: 'buffer',
    });
    const entries = parseTreeEntries(tree);
    for (const entry of entries) {
      if (!entry.pathValid) {
        coverage.invalidPaths += 1;
        coverage.complete = false;
        addLimitation(coverage, 'path não decodifica em UTF-8 estrito', 1);
        continue;
      }
      pathSet.add(entry.pathHash);
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
      const display = displayFor(entry);

      const key = `${entry.pathHash}\u0000${entry.oid}`;
      const existing = associations.get(key);
      if (existing) {
        existing.commits.add(index);
      } else {
        if (associations.size >= maxAssociations) fail('limite de associações path+blob excedido');
        associations.set(key, {
          pathText: entry.pathText,
          pathHash: entry.pathHash,
          pathBytes: entry.pathBytes.length,
          oid: entry.oid,
          size: entry.size,
          firstIndex: index,
          commits: new Set([index]),
        });
      }

      for (const finding of display.findings) {
        const pathContentKey = `${entry.pathHash}\u0000${finding.rule}\u0000${finding.line}\u0000${finding.excerpt}`;
        const existing = pathFindings.get(pathContentKey);
        if (existing) {
          existing.commits.add(index);
          existing.blobs.add(entry.oid);
        } else {
          pathFindings.set(pathContentKey, {
            commit: shortHash(commit),
            commitIndex: index,
            commitsFromHead: commits.length - 1 - index,
            path: display.text,
            pathHash: entry.pathHash,
            pathBytes: entry.pathBytes.length,
            pathRedacted: display.redacted,
            line: finding.line,
            rule: finding.rule,
            reason: finding.reason,
            hint: finding.hint,
            excerpt: finding.excerpt,
            commits: new Set([index]),
            blobs: new Set([entry.oid]),
          });
        }
      }

      const pathRule = classifyPath(entry.pathText);
      if (pathRule?.rule) {
        const pathKey = `${entry.pathHash}\u0000${pathRule.rule}`;
        const pathFinding = pathFindings.get(pathKey);
        if (pathFinding) {
          pathFinding.commits.add(index);
          pathFinding.blobs.add(entry.oid);
        } else {
          pathFindings.set(pathKey, {
            commit: shortHash(commit),
            commitIndex: index,
            commitsFromHead: commits.length - 1 - index,
            path: display.text,
            pathHash: entry.pathHash,
            pathBytes: entry.pathBytes.length,
            pathRedacted: display.redacted,
            line: 0,
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

  function addFindingWithLimit(finding) {
    if (findings.size >= maxFindings) {
      coverage.complete = false;
      addLimitation(coverage, 'limite de achados excedido', 1);
      truncated = true;
      return false;
    }
    addFinding(findings, finding);
    return true;
  }

  for (const association of pathFindings.values()) {
    if (!addFindingWithLimit({
      commit: association.commit,
      commitIndex: association.commitIndex,
      commitsFromHead: association.commitsFromHead,
      path: association.path,
      pathHash: association.pathHash,
      pathBytes: association.pathBytes,
      pathRedacted: association.pathRedacted,
      line: association.line,
      rule: association.rule,
      reason: association.reason,
      hint: association.hint,
      excerpt: association.excerpt,
      commits: association.commits.size,
      blobs: association.blobs.size,
    })) break;
  }

  if (!truncated) {
    const blobs = new Map();
    for (const [associationKey, association] of associations) {
      const existing = blobs.get(association.oid);
      if (existing) existing.associations.add(associationKey);
      else blobs.set(association.oid, { size: association.size, associations: new Set([associationKey]) });
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
      for (const associationKey of blob.associations) {
        const association = associations.get(associationKey);
        const display = pathDisplays.get(association.pathHash);
        const pathRule = classifyPath(association.pathText);
        if (pathRule?.skipContent) coverage.contentPolicyScanned += 1;
        const result = scanText(
          text,
          strict ? { file: association.pathText, allowlist: [] } : { file: association.pathText },
        );
        for (const finding of result.findings) {
          if (!addFindingWithLimit({
            commit: shortHash(commits[association.firstIndex]),
            commitIndex: association.firstIndex,
            commitsFromHead: commits.length - 1 - association.firstIndex,
            path: display.text,
            pathHash: association.pathHash,
            pathBytes: association.pathBytes,
            pathRedacted: display.redacted,
            line: finding.line,
            rule: finding.rule,
            reason: finding.reason,
            hint: finding.hint,
            excerpt: finding.excerpt,
            commits: association.commits.size,
            blobs: 1,
          })) break;
        }
        for (const allowed of result.allowed) {
          const policy = ALLOWLIST.find((entry) => entry.file === association.pathText
            && entry.rule === allowed.rule);
          const key = `${association.pathHash}\u0000${allowed.rule}`;
          const existing = allowedCounts.get(key);
          if (!existing && allowedCounts.size >= maxAllowedEntries) {
            coverage.complete = false;
            addLimitation(coverage, 'limite de allowed excedido', 1);
            truncated = true;
            break;
          }
          if (existing) {
            existing.occurrences += 1;
            existing.pathBlobs.add(`${association.pathHash}\u0000${oid}`);
          } else {
            allowedCounts.set(key, {
              id: policy?.id ?? null,
              file: display.text,
              fileHash: association.pathHash,
              rule: allowed.rule,
              reason: policy?.reason ?? null,
              scope: policy?.scope ?? null,
              source: 'scripts/review-public-source.mjs',
              occurrences: 1,
              pathBlobs: new Set([`${association.pathHash}\u0000${oid}`]),
            });
          }
          if (!policy) {
            coverage.complete = false;
            addLimitation(coverage, 'ocorrência liberada sem metadado de política', 1);
          }
        }
        if (truncated) break;
      }
      if (truncated) break;
    }
  }

  const historyFindings = [...findings.values()].sort((a, b) =>
    a.commitIndex - b.commitIndex
    || a.path.localeCompare(b.path)
    || a.line - b.line
    || a.rule.localeCompare(b.rule),
  );
  const allowed = [...allowedCounts.values()]
    .map(({ pathBlobs, ...entry }) => ({ ...entry, pathBlobs: pathBlobs.size }))
    .sort((a, b) => a.file.localeCompare(b.file) || a.rule.localeCompare(b.rule));
  const status = historyFindings.length || !coverage.complete ? 'blocked' : 'passed';
  const checkedAt = new Date().toISOString();
  return {
    status,
    exitCode: status === 'passed' ? 0 : 1,
    rev: revision,
    head,
    checkedAt,
    strict,
    commitsScanned: coverage.commitsScanned,
    historyFindings,
    allowed,
    coverage,
    startedAt,
    finishedAt: checkedAt,
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
