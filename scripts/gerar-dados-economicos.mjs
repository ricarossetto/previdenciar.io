// Gera as séries oficiais embutidas do motor a partir das séries públicas do
// Banco Central (SGS 188 = INPC/IBGE e SGS 190 = IGP-DI/FGV) e atualiza o
// JSON público com a mesma origem declarada.
//
// Uso: node scripts/gerar-dados-economicos.mjs
// A rede pública oficial é a única fonte; nenhum dado de caso entra aqui.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import https from 'node:https';

const repo = path.resolve(import.meta.dirname, '..');
const first = '1994-07';
const source = {
  inpc: { id: 188, label: 'INPC/IBGE', file: 'inpc.rs', var: 'INPC_MENSAL', fonte: 'INPC_FONTE' },
  igpdi: { id: 190, label: 'IGP-DI/FGV', file: 'igpdi.rs', var: 'IGPDI_MENSAL', fonte: 'IGPDI_FONTE' },
};

function fetchJson(url) {
  return new Promise((resolve, reject) => {
    const attempt = (tries) => {
      const request = https.get(url, { headers: { 'User-Agent': 'previdenciar.io/gerar-dados' } }, (response) => {
        let body = '';
        response.on('data', (chunk) => { body += chunk; });
        response.on('end', () => {
          if (body.trimStart().startsWith('[')) { resolve(body); return; }
          if (tries > 4) { reject(new Error(`resposta inesperada do provedor: ${body.slice(0, 80)}`)); return; }
          setTimeout(() => attempt(tries + 1), 3000);
        });
      });
      request.setTimeout(45000, () => request.destroy(new Error('tempo limite da consulta pública')));
      request.on('error', (error) => {
        if (tries > 4) { reject(error); return; }
        setTimeout(() => attempt(tries + 1), 3000);
      });
    };
    attempt(1);
  });
}

function readSeries(body) {
  const rows = JSON.parse(body);
  return rows
    .map((row) => {
      const [day, month, year] = row.data.split('/');
      return { period: `${year}-${month}`, value: String(row.valor).replace(',', '.') };
    })
    .filter((row) => row.period >= first)
    .sort((a, b) => (a.period < b.period ? -1 : 1));
}

function rustFile(config, rows, extractedAt) {
  const values = rows.map((row) => `    ("${row.period}", "${row.value}"),`).join('\n');
  return `// GERADO AUTOMATICAMENTE pelo script scripts/gerar-dados-economicos.mjs
// a partir da série pública SGS/${config.id} do Banco Central (${config.label}).
// Não editar à mão: a fonte oficial e a data de extração ficam declaradas aqui.
//
// (competência, valor). Ordenado por competência.
pub static ${config.var}: [(&str, &str); ${rows.length}] = [
${values}
];

pub const ${config.fonte}: &str = "${config.label} — variação mensal (SGS/BCB, série ${config.id}), extraída em ${extractedAt}.";
`;
}

const extractedAt = new Date().toISOString().slice(0, 10);
const publicJsonPath = path.join(repo, 'crates/previus-rgps/src/public-economic.json');
const publicJson = JSON.parse(fs.readFileSync(publicJsonPath, 'utf8'));

for (const [key, config] of Object.entries(source)) {
  const body = await fetchJson(`https://api.bcb.gov.br/dados/serie/bcdata.sgs.${config.id}/dados?formato=json`);
  const rows = readSeries(body);
  if (rows.length < 100) { throw new Error(`série ${config.id} com cobertura insuficiente: ${rows.length}`); }
  const filePath = path.join(repo, 'crates/rgps-core/src/dados', config.file);
  const content = rustFile(config, rows, extractedAt);
  fs.writeFileSync(filePath, content, 'utf8');
  const sha256 = crypto.createHash('sha256').update(content).digest('hex');
  publicJson[key] = {
    source_path: `src/dados/${config.file}`,
    source_sha256: sha256,
    source_declaration: content.match(/pub const \w+: &str = "([^"]+)";/)[1],
    first: rows[0].period,
    last: rows[rows.length - 1].period,
    count: rows.length,
    values: rows.map((row) => ({ period: row.period, value: row.value })),
  };
  console.log(`${config.label}: ${rows.length} meses (${rows[0].period}..${rows[rows.length - 1].period}) sha256=${sha256.slice(0, 16)}…`);
}

fs.writeFileSync(publicJsonPath, `${JSON.stringify(publicJson, null, 2)}\n`, 'utf8');
console.log(`public-economic.json atualizado (schema ${publicJson.schema_version}).`);
