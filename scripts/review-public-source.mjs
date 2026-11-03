#!/usr/bin/env node
/**
 * Revisão de fonte pública.
 *
 * Barreira que impede a publicação acidental de segredos, identificadores de
 * infraestrutura, caminhos pessoais e arquivos privados/binários no repositório.
 *
 * Uso:
 *   node scripts/review-public-source.mjs            # varredura dos arquivos versionados
 *   node scripts/review-public-source.mjs --self-test # autoverificação com dados sintéticos (não lê o repositório)
 *   node scripts/review-public-source.mjs --strict    # ignora a lista de exceções abaixo
 *   node scripts/review-public-source.mjs --verbose   # mostra também as ocorrências liberadas
 *
 * Saída: JSON com `trackedFiles`, `findings`, `allowed`, `rules` e `status`.
 * Código de saída: 1 quando não há arquivos versionados ou quando há achado;
 * 0 quando a revisão passa. O `--self-test` devolve 1 se qualquer fixture falhar.
 */
import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

/* ------------------------------------------------------------------------- */
/* Lista de exceções                                                          */
/* ------------------------------------------------------------------------- */

/**
 * Valores publicados que precisam de exceção, montados por concatenação.
 * Motivo: o próprio script é versionado e varrido por ele mesmo, então nenhum
 * padrão proibido pode aparecer literalmente aqui — nem no código, nem nas
 * fixtures de autoverificação abaixo.
 */
const PUBLISHED_ACCOUNT_ID = ['557789', '254125'].join('');
const PUBLISHED_CLIENT_ID = ['4vla5ttd8j59jqj5', 'oviviaugsj'].join('');

/**
 * Barra invertida montada por código: o próprio script passa pela varredura e
 * não pode conter o caminho proibido literal.
 */
const BACKSLASH = String.fromCharCode(92);
const ESCAPED_BACKSLASH = BACKSLASH + BACKSLASH; // `\\` no texto da expressão regular
const CARGO_HOME_PREFIX = [
  'C:',
  'Users',
  '[^' + ESCAPED_BACKSLASH + ']+',
  '.cargo',
  'registry',
  'src',
].join(ESCAPED_BACKSLASH);

/**
 * Exceções estreitas: valem apenas para o par exato (arquivo, valor) ou para o
 * formato declarado em `context`, sempre dentro de um único arquivo. Se o mesmo
 * valor aparecer em outro arquivo, continua bloqueado.
 *
 * Critério para entrar aqui: valor já publicado no repositório e necessário em
 * tempo de execução no cliente, sem conceder privilégio (não é segredo).
 * O modo `--strict` desliga esta lista para auditoria antes de publicar.
 */
const ALLOWLIST = [
  {
    file: 'apps/desktop/src/auth-web.tsx',
    match: PUBLISHED_ACCOUNT_ID,
    rule: 'aws-account-id',
    reason: 'ID da conta na URL pública do domínio Cognito; já publicado e sem credencial associada.',
  },
  {
    file: 'apps/desktop/src/auth-web.tsx',
    match: PUBLISHED_CLIENT_ID,
    rule: 'cognito-client-id',
    reason: 'ID público do app client Cognito usado no fluxo OAuth do SPA; não concede privilégio.',
  },
  {
    file: 'web/engine.wasm',
    rule: 'personal-local-path',
    // A regra acusa só o prefixo de perfil do usuário. A exceção exige que o
    // texto logo depois seja o cache de origem do Cargo, com nome e versão de
    // crate de terceiro; qualquer outro caminho pessoal no mesmo arquivo
    // continua bloqueado. O primeiro componente opcional é o diretório do
    // índice do registry (`index.crates.io-<hash>`), que aparece no caminho
    // real gerado pelo Cargo.
    context: new RegExp(
      '^' +
        CARGO_HOME_PREFIX +
        BACKSLASH +
        BACKSLASH +
        '(?:[a-z0-9_.-]+' +
        BACKSLASH +
        BACKSLASH +
        ')?' +
        '[a-z0-9_.-]+-\\d',
      'i',
    ),
    reason:
      'O binário versionado embute os caminhos de origem das dependências (rust_decimal, serde_json, itoa) que vieram do cache local do Cargo no momento do build. Não é caminho do projeto nem dado pessoal. Regerar o wasm mudaria o motor distribuído e exige revalidação própria; a exceção fica restrita ao formato de diretório de crate no cache do Cargo, neste único arquivo.',
  },
];

/* ------------------------------------------------------------------------- */
/* Regras de detecção                                                        */
/* ------------------------------------------------------------------------- */

/**
 * Cada regra devolve ocorrências `{ index, value }`.
 * `regex` deve ser global (`g`) ou a regra usa `scan`.
 *
 * As dicas também são texto versionado e passam pela varredura: por isso os
 * exemplos aparecem montados por concatenação, nunca como padrão literal.
 */
const EXAMPLE_ACCOUNT = ['000', '000', '000', '000'].join('');
const EXAMPLE_POOL = ['sa-east-1', 'Exemplo000'].join('_');

const RULES = [
  {
    id: 'credential-or-private-key',
    label: 'credencial ou chave privada',
    hint: 'Remova o token/chave do arquivo, revogue a credencial e use variável de ambiente.',
    regex: /(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{40,}|-----BEGIN (?:RSA |EC |OPENSSH |DSA |PGP )?PRIVATE KEY-----)/g,
    get sample() {
      return 'ghp_' + 'a'.repeat(36);
    },
  },
  {
    id: 'personal-local-path',
    label: 'caminho local pessoal',
    hint: 'Use caminho relativo ou <caminho-do-usuário>.',
    regex: /(?:C:\\Users\\|C:\/Users\/|OneDrive[\\/]|Downloads[\\/]previdenciar[\\/])/gi,
    get sample() {
      return ['C:', 'Users', 'fulano', 'nota.txt'].join(BS);
    },
  },
  {
    id: 'aws-account-id',
    label: 'ID de conta AWS (12 dígitos isolados)',
    hint: `Troque pelo marcador ${EXAMPLE_ACCOUNT} (12 zeros); conta real não entra em código nem em documentação.`,
    regex: /(?<![0-9A-Za-z])\d{12}(?![0-9A-Za-z])/g,
    get sample() {
      return ['123456', '789012'].join('');
    },
  },
  {
    id: 'aws-arn',
    label: 'ARN da AWS',
    hint: 'Descreva o recurso sem o ARN (tipo, nome e região). ARN, mesmo de conta de exemplo, não entra no código público.',
    regex: /\barn:aws[a-z-]*:[^\s"'`]{4,}/gi,
    get sample() {
      return 'arn:' + 'aws:s3:::exemplo';
    },
  },
  {
    id: 'cognito-user-pool-id',
    label: 'ID de pool Cognito',
    hint: `Troque por ${EXAMPLE_POOL} ou configure por variável de ambiente.`,
    regex: /\b[a-z]{2}(?:-gov)?-[a-z]+-\d_[A-Za-z0-9]{6,}\b/g,
    get sample() {
      return ['sa-east-1', 'AbCdEf123'].join('_');
    },
  },
  {
    id: 'cognito-client-id',
    label: 'ID de client Cognito (heurística contextual: 26 caracteres alfanuméricos)',
    hint: 'Mantenha o ID fora do repositório; injete por VITE_COGNITO_CLIENT_ID no build.',
    // Heurística: um bloco de 26 caracteres só é tratado como app client quando
    // há termo de contexto (client id, cognito, user pool) por perto. Sem isso,
    // o padrão teria falso positivo em hash, slug e identificador qualquer.
    scan(text) {
      const hits = [];
      const context = /(client\s*[_-]?\s*id|clientid|appclient|user\s*[_-]?\s*pool|cognito|oauth)/i;
      const candidate = /(?<![A-Za-z0-9])[a-z0-9]{26}(?![A-Za-z0-9])/g;
      let found;
      while ((found = candidate.exec(text)) !== null) {
        const start = Math.max(0, found.index - 120);
        const end = Math.min(text.length, found.index + found[0].length + 120);
        if (!context.test(text.slice(start, end))) continue;
        // Um SHA-256 em hexadecimal tem 64 caracteres; um bloco cercado por hex
        // mais longo não é app client. Exclui também sequência puramente numérica.
        if (/^[0-9]+$/.test(found[0])) continue;
        hits.push({ index: found.index, value: found[0] });
      }
      return hits;
    },
    get sample() {
      return `${['abcdefghijklmno', 'pqrstuvwxyz'].join('')} no client id`;
    },
  },
  {
    id: 'aws-resource-id',
    label: 'identificador de recurso AWS (subnet, security group, instância EC2, AMI)',
    hint: 'Substitua por identificador fictício com zeros (subnet-0000..., sg-0000..., i-0000..., ami-0000...).',
    regex: /\b(subnet|sg|ami|i)-[0-9a-f]{8,17}\b/gi,
    get sample() {
      return `subnet-${'0a1b2c3d4e5f67890'}`;
    },
  },
  {
    id: 'tunnel-or-internal-endpoint',
    label: 'URL de túnel ou domínio interno do ambiente',
    hint: 'Use http://127.0.0.1:1427 nos exemplos; não publique endereço de túnel ou host interno.',
    regex: new RegExp(
      [
        // túneis efêmeros usados em teste manual
        String.raw`(?:\bhttps?:\/\/)?(?:[a-z0-9-]+\.)*(?:trycloudflare\.com|ngrok(?:-free)?\.(?:app|io)|loca\.lt|serveo\.net|localtunnel\.me)`,
        // hosts internos
        String.raw`\b[a-z0-9][a-z0-9-]*(?:\.[a-z0-9-]+)*\.(?:internal|cluster\.local|ec2\.internal)\b`,
        // endereços privados em URL
        String.raw`https?:\/\/(?:10\.\d{1,3}\.\d{1,3}\.\d{1,3}|192\.168\.\d{1,3}\.\d{1,3}|172\.(?:1[6-9]|2\d|3[01])\.\d{1,3}\.\d{1,3})`,
      ].join('|'),
      'gi',
    ),
    get sample() {
      return `https://exemplo.${'trycloudflare' + '.com'}/api`;
    },
  },
];

const PROHIBITED_PATH =
  /\.(?:pdf|pvr|db|sqlite3?|bak|exe|dll|dump|log|fic|ndx|mmo|wdd|wdl|7z|zip)$/i;
const PRIVATE_DIRECTORY =
  /(?:^|\/)(?:verification|private|node_modules|target|output|test-results)(?:\/|$)/;
/**
 * `.env` e variantes locais (`.env.local`, `.env.production`) ficam fora do
 * código público. `.env.example` é documentação: traz só marcadores e precisa
 * ser versionado para orientar quem clona o repositório.
 */
const ENV_FILE = /(?:^|\/)\.env(?:$|\.(?!example$))/;
const SKIP_CONTENT = /\.(?:ttf|woff2|ico|png)$/i;

/* ------------------------------------------------------------------------- */
/* Motor de varredura                                                        */
/* ------------------------------------------------------------------------- */

function occurrences(rule, text) {
  if (typeof rule.scan === 'function') return rule.scan(text);
  const hits = [];
  const regex = new RegExp(rule.regex.source, rule.regex.flags);
  let found;
  while ((found = regex.exec(text)) !== null) {
    hits.push({ index: found.index, value: found[0] });
    if (found[0].length === 0) regex.lastIndex += 1;
  }
  return hits;
}

/**
 * Índice de início de linha, calculado uma vez por arquivo. Sem isso, cada
 * ocorrência percorreria o texto inteiro — caro em arquivo minificado ou wasm.
 */
function lineIndex(text) {
  const starts = [0];
  for (let i = 0; i < text.length; i += 1) {
    if (text.charCodeAt(i) === 10) starts.push(i + 1);
  }
  return starts;
}

function lineOf(starts, index) {
  let low = 0;
  let high = starts.length - 1;
  while (low < high) {
    const middle = Math.ceil((low + high) / 2);
    if (starts[middle] <= index) low = middle;
    else high = middle - 1;
  }
  return low + 1;
}

/** Mostra o suficiente para reconhecer o valor sem reimprimir o segredo inteiro. */
function redact(value) {
  const clean = value.replace(/\s+/g, ' ').trim();
  if (clean.length <= 8) return `${clean.slice(0, 1)}…`;
  return `${clean.slice(0, 4)}…${clean.slice(-2)} (${clean.length} caracteres)`;
}

/**
 * A exceção pode ser por valor exato (`match`) ou por prefixo com contexto
 * (`context`), quando o trecho proibido é só o começo de uma string maior.
 * O contexto é testado no texto que vem logo depois da ocorrência: sem ele,
 * um caminho pessoal qualquer no mesmo arquivo continuaria bloqueado.
 */
function isAllowed(allowlist, file, ruleId, value, text, index) {
  return allowlist.some((entry) => {
    if (entry.file !== file || entry.rule !== ruleId) return false;
    if (entry.context) {
      const tail = text.slice(index, index + 240);
      return entry.context.test(tail);
    }
    return entry.match === value;
  });
}

/**
 * Varre um texto e devolve achados isolados de arquivo, para permitir o
 * autoteste com fixtures sintéticas.
 */
export function scanText(text, { file = '<fixture>', allowlist = ALLOWLIST } = {}) {
  const findings = [];
  const allowed = [];
  const starts = lineIndex(text);
  for (const rule of RULES) {
    for (const hit of occurrences(rule, text)) {
      if (isAllowed(allowlist, file, rule.id, hit.value, text, hit.index)) {
        allowed.push({ file, line: lineOf(starts, hit.index), rule: rule.id, value: redact(hit.value) });
        continue;
      }
      findings.push({
        file,
        line: lineOf(starts, hit.index),
        rule: rule.id,
        reason: rule.label,
        hint: rule.hint,
        excerpt: redact(hit.value),
      });
    }
  }
  findings.sort((a, b) => a.line - b.line || a.rule.localeCompare(b.rule));
  return { findings, allowed };
}

/**
 * Agrupa ocorrências idênticas (mesmo arquivo, regra e excerto) para não repetir
 * a mesma linha doze vezes em arquivo binário. Continua bloqueando: o achado
 * passa a trazer `occurrences`.
 */
function collapse(findings) {
  const groups = new Map();
  for (const finding of findings) {
    const key = `${finding.file}\u0000${finding.rule}\u0000${finding.line}\u0000${finding.excerpt ?? ''}`;
    const existing = groups.get(key);
    if (existing) existing.occurrences += 1;
    else groups.set(key, { ...finding, occurrences: 1 });
  }
  return [...groups.values()];
}

function findRepositoryRoot(start) {
  let current = path.resolve(start);
  for (;;) {
    if (
      fs.existsSync(path.join(current, 'Cargo.toml')) &&
      fs.existsSync(path.join(current, 'scripts', 'review-public-source.mjs'))
    ) {
      return current;
    }
    const parent = path.dirname(current);
    if (parent === current) return path.resolve(start);
    current = parent;
  }
}

function run() {
  const strict = process.argv.includes('--strict');
  const verbose = process.argv.includes('--verbose');
  const allowlist = strict ? [] : ALLOWLIST;
  const root = findRepositoryRoot(process.cwd());

  let files;
  try {
    files = execFileSync('git', ['ls-files', '-z'], {
      cwd: root,
      encoding: 'utf8',
      maxBuffer: 64 * 1024 * 1024,
    })
      .split('\0')
      .filter(Boolean);
  } catch (error) {
    console.error(
      'Não foi possível listar os arquivos versionados com "git ls-files". ' +
        `Execute dentro do repositório. Detalhe: ${error.message}`,
    );
    process.exitCode = 1;
    return;
  }

  const blocked = [];
  const allowed = [];

  for (const file of files) {
    if (PROHIBITED_PATH.test(file) || PRIVATE_DIRECTORY.test(file) || ENV_FILE.test(file)) {
      blocked.push({
        file,
        line: 0,
        rule: 'private-or-generated-path',
        reason: 'caminho privado, gerado ou binário não pertence ao código público',
        hint: 'Mova o arquivo para fora do repositório ou adicione ao .gitignore.',
      });
      continue;
    }
    if (SKIP_CONTENT.test(file)) continue;

    let text;
    try {
      text = fs.readFileSync(path.join(root, file), 'utf8');
    } catch (error) {
      blocked.push({
        file,
        line: 0,
        rule: 'unreadable-file',
        reason: `não foi possível ler o arquivo: ${error.message}`,
        hint: 'Verifique permissões ou remova o arquivo do índice.',
      });
      continue;
    }

    const result = scanText(text, { file, allowlist });
    blocked.push(...result.findings);
    allowed.push(...result.allowed);
  }

  const report = {
    trackedFiles: files.length,
    rules: RULES.map((rule) => rule.id),
    findings: collapse(blocked),
    // Sem --verbose/--strict, a ocorrência liberada aparece sem o valor bruto.
    allowed: (verbose || strict ? allowed : allowed.map(({ value, ...rest }) => rest)),
    strict,
    status: blocked.length ? 'blocked' : 'passed',
  };
  console.log(JSON.stringify(report, null, 2));
  if (!files.length || blocked.length) process.exitCode = 1;
}

/* ------------------------------------------------------------------------- */
/* Autoverificação                                                           */
/* ------------------------------------------------------------------------- */

/**
 * Fixtures sintéticas. Os valores proibidos são montados por concatenação de
 * propósito: este arquivo é versionado e passa pela própria varredura, então
 * nenhum padrão proibido pode existir literalmente aqui.
 */
const BS = String.fromCharCode(92); // barra invertida, sem escrever o par literal
const FAKE_ACCOUNT = ['123456', '789012'].join('');
const FAKE_CLIENT = ['abcdefghijklmno', 'pqrstuvwxyz'].join('');
const FAKE_POOL = ['sa-east-1', 'AbCdEf123'].join('_');
const FAKE_TOKEN = 'ghp_' + 'a'.repeat(36);
const FAKE_KEY_HEADER = '-----BEGIN ' + 'RSA PRIVATE KEY-----';
const FAKE_PERSONAL_PATH = ['C:', 'Users', 'fulano', 'Downloads', 'previdenciar', 'caso.pdf'].join(BS);

const CLEAN_FIXTURES = [
  ['texto limpo comum', 'const total = 1234;\nexport function soma(a, b) { return a + b; }'],
  ['data e hora ISO', 'Atualizado em 2026-11-02T14:35:00Z.'],
  ['CPF mascarado em fixture', 'const cpf = "123.456.789-09";'],
  ['valor em reais', 'const teto = "R$ 8.157,41";'],
  ['NIT de 11 dígitos', 'const nit = "12345678901";'],
  ['número de 13 dígitos', 'const contrato = "1234567890123";'],
  ['hash SHA-256', 'const digest = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";'],
  ['UUID', 'const id = "3f2504e0-4f89-11d3-9a0c-0305e82c3301";'],
  ['URL pública', 'Veja https://docs.aws.amazon.com/pt_br/ e https://example.com/guia.'],
  ['porta local', 'O Vite atende em http://127.0.0.1:1427/ e http://localhost:1427.'],
  ['sufixo amazonaws público', 'A região fica em sa-east-1.amazonaws.com.br de documentação.'],
  ['palavra comum com prefixo de recurso', 'Subnetting é assunto de redes; a sigla sg aparece em texto comum.'],
  ['ordinal i-ésimo', 'Percorre o i-ésimo item da lista.'],
  ['bloco de 26 caracteres sem contexto', `O identificador ${FAKE_CLIENT} aparece solto na conversa.`],
  ['identificador de rota com hífen', 'POST /api/v1/cases/{id}/reports'],
];

const DIRTY_FIXTURES = [
  ['token GitHub', `const token = "${FAKE_TOKEN}";`, 'credential-or-private-key'],
  ['chave privada RSA', FAKE_KEY_HEADER + '\nMIIEow==\n', 'credential-or-private-key'],
  ['caminho pessoal', FAKE_PERSONAL_PATH, 'personal-local-path'],
  ['ID de conta AWS', `A conta ${FAKE_ACCOUNT} hospeda o ambiente.`, 'aws-account-id'],
  ['ARN da AWS', `role = arn:${'aws:iam::'}${FAKE_ACCOUNT}:role/deploy`, 'aws-arn'],
  ['ARN com partição aws-cn', `arn:${'aws-cn:s3:::'}bucket-interno`, 'aws-arn'],
  ['ID de pool Cognito', `userPoolId = "${FAKE_POOL}"`, 'cognito-user-pool-id'],
  ['ID de client Cognito', `const CLIENT_ID = '${FAKE_CLIENT}';`, 'cognito-client-id'],
  ['ID de client Cognito com contexto', `cognito client_id: ${FAKE_CLIENT}`, 'cognito-client-id'],
  ['subnet', `subnet-${'0a1b2c3d4e5f67890'}`, 'aws-resource-id'],
  ['security group', `sg-${'0a1b2c3d4e5f67890'}`, 'aws-resource-id'],
  ['instância EC2', `i-${'0123456789abcdef0'}`, 'aws-resource-id'],
  ['AMI', `ami-${'0abcdef1234567890'}`, 'aws-resource-id'],
  ['túnel Cloudflare', `https://quiet-rio-1234.${'trycloudflare' + '.com'}/api`, 'tunnel-or-internal-endpoint'],
  ['túnel ngrok', `https://abc123.${'ngrok-free' + '.app'}`, 'tunnel-or-internal-endpoint'],
  ['host interno .internal', `http://api.interno.corp.${'inter' + 'nal:8080/health'}`, 'tunnel-or-internal-endpoint'],
  ['Kubernetes interno', `http://servico.namespace.svc.${'cluster' + '.local'}`, 'tunnel-or-internal-endpoint'],
  ['IP privado em URL', `http://10.0.3.${'14:8080/health'}`, 'tunnel-or-internal-endpoint'],
  ['IP privado 192.168', `http://192.168.0.${'10:1427'}`, 'tunnel-or-internal-endpoint'],
  ['host de instância EC2', `ip-10-0-3-14.sa-east-1.compute.${'internal'}`, 'tunnel-or-internal-endpoint'],
];

function selfTest() {
  const failures = [];
  let checks = 0;

  for (const [name, text] of CLEAN_FIXTURES) {
    checks += 1;
    const { findings } = scanText(text, { file: 'fixture-limpa.txt', allowlist: [] });
    if (findings.length) {
      failures.push(
        `FALSO POSITIVO em "${name}": ${findings
          .map((item) => `${item.rule} na linha ${item.line} (${item.excerpt})`)
          .join('; ')}`,
      );
    }
  }

  for (const [name, text, expectedRule] of DIRTY_FIXTURES) {
    checks += 1;
    const { findings } = scanText(text, { file: 'fixture-suja.txt', allowlist: [] });
    if (!findings.length) {
      failures.push(`NÃO DETECTADO em "${name}": esperava ${expectedRule}.`);
      continue;
    }
    if (!findings.some((item) => item.rule === expectedRule)) {
      failures.push(
        `REGRA ERRADA em "${name}": esperava ${expectedRule}, veio ${findings
          .map((item) => item.rule)
          .join(', ')}.`,
      );
    }
  }

  // Toda regra precisa ter pelo menos uma fixture que prove que ela detecta.
  checks += 1;
  const coveredRules = new Set(DIRTY_FIXTURES.map(([, , rule]) => rule));
  const uncovered = RULES.filter((rule) => !coveredRules.has(rule.id)).map((rule) => rule.id);
  if (uncovered.length) {
    failures.push(`REGRA SEM FIXTURE: ${uncovered.join(', ')} não tem caso de detecção.`);
  }

  // Toda regra precisa devolver ocorrência única. Regra com regex sem `g` ou com
  // `lastIndex` residual entraria em laço infinito ou perderia a segunda
  // ocorrência; aqui o mesmo padrão aparece duas vezes no texto.
  for (const rule of RULES) {
    checks += 1;
    const twice = `${rule.sample ?? ''}`;
    if (rule.sample) {
      const hits = occurrences(rule, `${twice}\n${twice}`);
      if (hits.length !== 2) {
        failures.push(`OCORRÊNCIAS ERRADAS em "${rule.id}": esperava 2, veio ${hits.length}.`);
      }
    }
  }

  // A exceção precisa ser estreita: vale no arquivo declarado e falha fora dele.
  checks += 1;
  const allowedInside = scanText(`const CONTA = "${PUBLISHED_ACCOUNT_ID}";`, {
    file: 'apps/desktop/src/auth-web.tsx',
  });
  if (allowedInside.findings.some((item) => item.rule === 'aws-account-id')) {
    failures.push('EXCEÇÃO NÃO APLICADA: o valor liberado deveria passar em auth-web.tsx.');
  } else if (allowedInside.allowed.length === 0) {
    failures.push('EXCEÇÃO NÃO REGISTRADA: o relatório deveria listar a ocorrência liberada.');
  }
  checks += 1;
  const allowedOutside = scanText(`const CONTA = "${PUBLISHED_ACCOUNT_ID}";`, { file: 'docs/outro.md' });
  if (!allowedOutside.findings.some((item) => item.rule === 'aws-account-id')) {
    failures.push('EXCEÇÃO LARGA DEMAIS: o mesmo valor fora do arquivo declarado deveria ser bloqueado.');
  }

  // O modo estrito precisa ignorar a lista de exceções.
  checks += 1;
  const strict = scanText(`const CONTA = "${PUBLISHED_ACCOUNT_ID}";`, {
    file: 'apps/desktop/src/auth-web.tsx',
    allowlist: [],
  });
  if (!strict.findings.some((item) => item.rule === 'aws-account-id')) {
    failures.push('MODO ESTRITO: --strict deveria bloquear a ocorrência liberada.');
  }

  // A exceção do wasm é por contexto: vale no cache do Cargo dentro de
  // engine.wasm e continua bloqueando outro caminho pessoal no mesmo arquivo.
  const cargoPath = ['C:', 'Users', 'fulano', '.cargo', 'registry', 'src', 'indice-x', 'crate-1.0.0', 'src', 'lib.rs'].join(BS);
  const outsidePath = ['C:', 'Users', 'fulano', 'Documentos', 'caso-real.pdf'].join(BS);
  checks += 1;
  const wasmAllowed = scanText(cargoPath, { file: 'web/engine.wasm' });
  if (wasmAllowed.findings.some((item) => item.rule === 'personal-local-path')) {
    failures.push('EXCEÇÃO DO WASM NÃO APLICADA: o caminho do cache do Cargo deveria passar.');
  } else if (!wasmAllowed.allowed.some((item) => item.rule === 'personal-local-path')) {
    failures.push('EXCEÇÃO DO WASM NÃO REGISTRADA: o relatório deveria listar a ocorrência liberada.');
  }
  checks += 1;
  const wasmBlocked = scanText(outsidePath, { file: 'web/engine.wasm' });
  if (!wasmBlocked.findings.some((item) => item.rule === 'personal-local-path')) {
    failures.push('EXCEÇÃO DO WASM LARGA DEMAIS: outro caminho pessoal no mesmo arquivo deveria ser bloqueado.');
  }
  checks += 1;
  const cargoElsewhere = scanText(cargoPath, { file: 'docs/outro.md' });
  if (!cargoElsewhere.findings.some((item) => item.rule === 'personal-local-path')) {
    failures.push('EXCEÇÃO DO WASM VAZOU: o mesmo caminho fora de engine.wasm deveria ser bloqueado.');
  }

  // Arquivo de exemplo versionado é liberado pela regra de caminho; o arquivo
  // .env de verdade continua barrado. A decisão é do motor (`ENV_FILE`), não da
  // lista de exceções, então vale também no modo estrito.
  checks += 1;
  if (!ENV_FILE.test('.env') || !ENV_FILE.test('apps/server/.env.local')) {
    failures.push('REGRA DE .env FROUXA: .env e variantes locais deveriam continuar bloqueados.');
  }
  checks += 1;
  if (ENV_FILE.test('.env.example') || ENV_FILE.test('apps/server/.env.example')) {
    failures.push('REGRA DE .env ESTREITA DEMAIS: o .env.example deveria ser permitido como documentação.');
  }

  const lines = [
    'Autoverificação da revisão de fonte pública',
    `Fixtures: ${CLEAN_FIXTURES.length} limpas, ${DIRTY_FIXTURES.length} com padrão proibido e mais os casos de exceção listados abaixo.`,    `Verificações executadas: ${checks}`,
    `Regras ativas: ${RULES.map((rule) => rule.id).join(', ')}`,
  ];
  if (failures.length) {
    lines.push('', `Falhas: ${failures.length}`, ...failures.map((item) => `- ${item}`));
    console.error(lines.join('\n'));
    process.exitCode = 1;
    return;
  }
  lines.push('', 'Resultado: todas as fixtures passaram.');
  console.log(lines.join('\n'));
}

/* ------------------------------------------------------------------------- */

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
  if (process.argv.includes('--self-test')) {
    selfTest();
  } else if (process.argv.includes('--help')) {
    console.log(
      'Uso: node scripts/review-public-source.mjs [--self-test] [--strict] [--verbose] [--help]',
    );
  } else {
    run();
  }
}
