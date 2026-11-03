import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import {
  HistoryError,
  parseTreeEntries,
  runHistoryScan,
  runCli,
} from './review-public-history.mjs';
import { classifyPath, redactText } from './review-public-source.mjs';

const FIXTURE_ROOT = process.env.HISTORY_FIXTURE_ROOT
  || path.join(os.homedir(), '.sagas', 'previdenciar-fila-integral-20261102', 'tmp-historico');
fs.mkdirSync(FIXTURE_ROOT, { recursive: true });
let fixtureCounter = 0;

function git(repo, args) {
  return execFileSync('git', ['-C', repo, ...args], {
    encoding: 'utf8',
    windowsHide: true,
    stdio: ['ignore', 'pipe', 'pipe'],
  });
}

function newRepo(label) {
  fixtureCounter += 1;
  const repo = path.join(
    FIXTURE_ROOT,
    `tmp-historico-${label}-${Date.now()}-${process.pid}-${fixtureCounter}`,
  );
  fs.mkdirSync(repo, { recursive: true });
  git(repo, ['init', '-q']);
  git(repo, ['config', 'core.autocrlf', 'false']);
  git(repo, ['config', 'user.email', 'fixture@example.invalid']);
  git(repo, ['config', 'user.name', 'Fixture']);
  return repo;
}

function write(repo, relative, content) {
  const target = path.join(repo, relative);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.writeFileSync(target, content);
}

function commit(repo, message) {
  git(repo, ['add', '-A']);
  git(repo, ['commit', '-q', '-m', message]);
}

function snapshot(repo) {
  return {
    head: git(repo, ['rev-parse', 'HEAD']).trim(),
    status: git(repo, ['status', '--porcelain']).trim(),
    refs: git(repo, ['for-each-ref', '--format=%(refname) %(objectname)']).trim(),
    fsck: git(repo, ['fsck', '--no-progress', '--no-dangling']).trim(),
  };
}

function scan(repo, options = {}) {
  return runHistoryScan({ repo, ...options });
}

test('token e caminho pessoal apagados no HEAD são detectados no commit antigo', () => {
  const repo = newRepo('old');
  write(repo, 'clean.txt', 'ok\n');
  commit(repo, 'base');
  const token = 'ghp_' + 'a'.repeat(36);
  const personal = ['C:', 'Users', 'fulano', 'Downloads', 'previdenciar', 'caso.pdf'].join('\\');
  write(repo, 'old.txt', `linha 1\nlinha 2\n${token}\n${personal}\n`);
  commit(repo, 'segredo antigo');
  git(repo, ['rm', '-q', 'old.txt']);
  commit(repo, 'remove segredo');

  const before = snapshot(repo);
  const result = scan(repo);
  const tokenFinding = result.historyFindings.find((item) => item.rule === 'credential-or-private-key');
  const pathFinding = result.historyFindings.find((item) => item.rule === 'personal-local-path');
  assert.equal(result.status, 'blocked');
  assert.equal(result.commitsScanned, 3);
  assert.ok(tokenFinding);
  assert.ok(pathFinding);
  assert.equal(tokenFinding.path, 'old.txt');
  assert.equal(tokenFinding.line, 3);
  assert.match(tokenFinding.commit, /^[0-9a-f]{12}$/);
  assert.ok(!JSON.stringify(result).includes(token));
  assert.ok(!JSON.stringify(result).includes(personal));
  assert.deepEqual(snapshot(repo), before);
});

test('merge com dois pais é varrido e o segredo do ramo é encontrado', () => {
  const repo = newRepo('merge');
  write(repo, 'base.txt', 'base\n');
  commit(repo, 'base');
  const main = git(repo, ['rev-parse', '--abbrev-ref', 'HEAD']).trim();
  git(repo, ['checkout', '-q', '-b', 'side']);
  write(repo, 'side.txt', 'const key = "ghp_' + 'b'.repeat(36) + '";\n');
  commit(repo, 'side');
  git(repo, ['checkout', '-q', main]);
  write(repo, 'main.txt', 'main\n');
  commit(repo, 'main');
  git(repo, ['merge', '-q', '--no-ff', 'side', '-m', 'merge side']);

  const result = scan(repo);
  assert.ok(result.historyFindings.some((item) => item.path === 'side.txt'));
  assert.ok(result.commitsScanned >= 4);
});

test('mesmo blob em path permitido e proibido: allowlist é path-exact', () => {
  const repo = newRepo('allowlist');
  const published = ['557789', '254125'].join('');
  const content = `const ACCOUNT = "${published}";\n`;
  write(repo, 'apps/desktop/src/auth-web.tsx', content);
  write(repo, 'docs/copia.tsx', content);
  commit(repo, 'blob duplicado');

  const result = scan(repo);
  const forbidden = result.historyFindings.filter((item) => item.rule === 'aws-account-id');
  assert.ok(forbidden.some((item) => item.path === 'docs/copia.tsx'));
  assert.ok(!forbidden.some((item) => item.path === 'apps/desktop/src/auth-web.tsx'));
  assert.ok(result.allowed.some((item) => item.file === 'apps/desktop/src/auth-web.tsx'
    && item.rule === 'aws-account-id' && item.occurrences >= 1
    && item.id === 'EX-01' && item.reason && item.scope));

  const strict = scan(repo, { strict: true });
  assert.ok(strict.historyFindings.some((item) => item.path === 'apps/desktop/src/auth-web.tsx'));
  assert.ok(!strict.allowed.length);
});

test('range relativo funciona e range/ref inválido falha sem vazar conteúdo', () => {
  const repo = newRepo('range');
  write(repo, 'a.txt', 'a\n');
  commit(repo, 'a');
  write(repo, 'b.txt', 'ghp_' + 'c'.repeat(36) + '\n');
  commit(repo, 'b');

  const range = scan(repo, { rev: 'HEAD~1..HEAD' });
  assert.equal(range.commitsScanned, 1);
  assert.ok(range.historyFindings.some((item) => item.path === 'b.txt'));
  assert.throws(() => scan(repo, { rev: '--all' }), HistoryError);
  const cli = runCli(['--history', '--rev', '--all']);
  assert.equal(cli.code, 1);
  assert.ok(!cli.stdout.includes('ghp_'));
});

test('oversize e binário são cobertos/limitação explicitada; binário é varrido', () => {
  const repo = newRepo('coverage');
  write(repo, 'small.txt', 'ok\n');
  write(repo, 'big.txt', 'x'.repeat(64));
  commit(repo, 'tamanhos');
  const oversize = scan(repo, { maxBlobBytes: 16 });
  assert.equal(oversize.status, 'blocked');
  assert.equal(oversize.coverage.complete, false);
  assert.ok(oversize.coverage.skippedOversize >= 1);

  const binaryRepo = newRepo('binary');
  write(binaryRepo, 'binary.dat', Buffer.from([0, 1, 2, 103, 104, 112, 95, ...Buffer.from('d'.repeat(36))]));
  commit(binaryRepo, 'binario');
  const binary = scan(binaryRepo);
  assert.ok(binary.coverage.binaryBlobsScanned >= 1);
  assert.ok(binary.historyFindings.some((item) => item.rule === 'credential-or-private-key'));
});

test('repositório inválido falha fechado; CLI desconhecida retorna 1', () => {
  assert.throws(() => scan(path.join(FIXTURE_ROOT, 'nao-existe')), HistoryError);
  assert.equal(runCli(['--history', '--bogus']).code, 1);
  assert.equal(runCli([]).code, 1);
});

test('path com token é redigido, com hash, sem vazar no JSON', () => {
  const repo = newRepo('path-sanitize');
  const token = 'ghp_' + 'e'.repeat(36);
  write(repo, `${token}.txt`, 'conteudo limpo\n');
  commit(repo, 'arquivo com token no nome');
  const result = scan(repo);
  const finding = result.historyFindings.find((item) => item.rule === 'credential-or-private-key');
  assert.ok(finding);
  assert.ok(finding.pathRedacted);
  assert.ok(!finding.path.includes(token));
  assert.match(finding.pathHash, /^[0-9a-f]{64}$/);
  assert.ok(finding.pathBytes > 0);
  assert.ok(!JSON.stringify(result).includes(token));
  assert.match(result.head, /^[0-9a-f]{40,64}$/);
  assert.ok(!Number.isNaN(Date.parse(result.checkedAt)));
});

test('caminho pessoal simulado é redigido sem allowlist', () => {
  const personal = ['C:', 'Users', 'fulano', 'Downloads', 'previdenciar', 'caso.pdf'].join('\\');
  const redacted = redactText(personal);
  assert.ok(!redacted.text.includes('C:\\Users'));
  assert.ok(redacted.rules.includes('personal-local-path'));
});

test('duas ocorrências liberadas no mesmo path+blob contam 2, não 1', () => {
  const repo = newRepo('occurrences');
  const cargo = (name) => ['C:', 'Users', 'fulano', '.cargo', 'registry', 'src',
    'index.crates.io-123', 'crate-1.0.0', 'src', name].join('\\');
  write(repo, 'web/engine.wasm', `${cargo('a.rs')}\n${cargo('b.rs')}\n`);
  commit(repo, 'duas ocorrencias');
  const result = scan(repo);
  const allowed = result.allowed.find((item) => item.file === 'web/engine.wasm');
  assert.ok(allowed);
  assert.equal(allowed.occurrences, 2);
  assert.equal(allowed.pathBlobs, 1);
  assert.equal(allowed.id, 'EX-03');
  assert.ok(allowed.reason && allowed.scope);
});

test('limite de achados combinado é fail-closed e não ultrapassa o teto', () => {
  const repo = newRepo('limit');
  for (let index = 0; index < 6; index += 1) {
    write(repo, `f${index}.txt`, `ghp_${String(index).repeat(36)}\n`);
  }
  commit(repo, 'muitos achados');
  const result = scan(repo, { maxFindings: 2 });
  assert.equal(result.status, 'blocked');
  assert.equal(result.coverage.complete, false);
  assert.ok(result.historyFindings.length <= 2);
  assert.ok(result.coverage.limitations.some((item) => item.category.includes('limite de achados')));
});

test('refs/replace é detectado e bloqueia cobertura', () => {
  const repo = newRepo('replace');
  write(repo, 'base.txt', 'base\n');
  commit(repo, 'base');
  const first = git(repo, ['rev-parse', 'HEAD']).trim();
  write(repo, 'changed.txt', 'changed\n');
  commit(repo, 'changed');
  const second = git(repo, ['rev-parse', 'HEAD']).trim();
  git(repo, ['replace', first, second]);
  const result = scan(repo);
  assert.equal(result.status, 'blocked');
  assert.equal(result.coverage.complete, false);
  assert.ok(result.coverage.historyControls.replaceRefs >= 1);
  assert.ok(result.coverage.limitations.some((item) => item.category.includes('refs/replace')));
});

test('parser de árvore: path UTF-8 inválido é incompleto; U+FFFD legítimo é válido', () => {
  const oid = 'a'.repeat(40);
  const entry = (pathBytes) => Buffer.concat([
    Buffer.from(`100644 blob ${oid} 1\t`, 'ascii'),
    pathBytes,
    Buffer.from([0]),
  ]);
  const invalid = parseTreeEntries(entry(Buffer.from([0xc3, 0x28])));
  assert.equal(invalid.length, 1);
  assert.equal(invalid[0].pathValid, false);
  assert.equal(invalid[0].pathText, null);
  assert.match(invalid[0].pathHash, /^[0-9a-f]{64}$/);

  const replacementCharBytes = Buffer.from('\uFFFD', 'utf8');
  const valid = parseTreeEntries(entry(replacementCharBytes));
  assert.equal(valid[0].pathValid, true);
  assert.equal(valid[0].pathText, '\uFFFD');
  assert.match(valid[0].pathHash, /^[0-9a-f]{64}$/);
});

test('classificação de caminho compartilhada não libera .env nem amplia allowlist', () => {
  assert.equal(classifyPath('.env').rule, 'private-or-generated-path');
  assert.equal(classifyPath('apps/server/.env.local').rule, 'private-or-generated-path');
  assert.equal(classifyPath('.env.example'), null);
  assert.equal(classifyPath('web/engine.wasm'), null);
});
