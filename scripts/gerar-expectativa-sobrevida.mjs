// Gera crates/rgps-core/src/dados/expectativa_sobrevida.rs a partir da tabela
// padrão do produto de referência (export REA). O CSV não entra no repositório:
// passe o caminho como argumento. Nenhum dado de cliente é lido.
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';

const source = process.argv[2];
if (!source) {
  console.error('Uso: node scripts/gerar-expectativa-sobrevida.mjs <tabelaexpectativasobrevidapadrao.csv>');
  process.exit(2);
}
const lines = readFileSync(source, 'latin1').split(/\r?\n/).filter((line) => line.trim());
const headerDelimiter = (lines[0].split(';').length > lines[0].split(',').length) ? ';' : ',';
const header = lines[0].split(headerDelimiter).map((name) => name.trim());
const indexOf = (name) => header.indexOf(name);
const firstH = indexOf('Idade0H');
const firstM = indexOf('Idade0M');
const lastUUID = indexOf('TabelaExpectativaUUID');
if (firstH < 0 || firstM < 0 || lastUUID < 0) throw new Error('Cabeçalho sem as colunas Idade0H/Idade0M/UUID.');
const slots = 91; // Idade0..Idade90
const rows = [];
for (const line of lines.slice(1)) {
  const fields = line.split(headerDelimiter);
  if (fields.length < lastUUID + 1) throw new Error(`Linha com ${fields.length} campos; esperado ${lastUUID + 1}.`);
  const vigencia = fields[0];
  if (!/^\d{4}-\d{2}-\d{2}$/.test(vigencia)) throw new Error(`Vigência inválida: ${vigencia}`);
  const slice = (start) => Array.from({ length: slots }, (_, age) => {
    const value = (fields[start + age] ?? '').trim();
    return value && Number(value) > 0 ? value : '';
  });
  rows.push([vigencia, slice(firstH), slice(firstM)]);
}
for (let i = 1; i < rows.length; i += 1) {
  if (rows[i - 1][0] >= rows[i][0]) throw new Error('Vigências fora de ordem.');
}
const digest = createHash('sha256').update(JSON.stringify(rows)).digest('hex');
const table = (sexIndex) => rows.map(([vigencia, h, m]) => {
  const values = (sexIndex === 0 ? h : m).map((value) => `"${value}"`).join(',');
  return `    ("${vigencia}", [${values}]),`;
}).join('\n');
const output = `// GERADO AUTOMATICAMENTE por scripts/gerar-expectativa-sobrevida.mjs a partir da
// tabela padrão do produto de referência (export REA). Não editar à mão.
// Série: SHA-256 ${digest}
// Cobertura: ${rows[0][0]} a ${rows[rows.length - 1][0]} (${rows.length} vigências), Idade0..Idade90.
// Vazio significa idade sem valor publicado na vigência.

/// (vigência, [expectativa por idade 0..90], homens).
pub static EXPECTATIVA_MASCULINA: [(&str, [&str; ${slots}]); ${rows.length}] = [
${table(0)}
];

/// (vigência, [expectativa por idade 0..90], mulheres).
pub static EXPECTATIVA_FEMININA: [(&str, [&str; ${slots}]); ${rows.length}] = [
${table(1)}
];

pub const EXPECTATIVA_FONTE: &str = "Tábua de expectativa de sobrevida por idade e sexo, vigências anuais de dezembro (tabela padrão do produto de referência, export REA); conferir com a publicação IBGE vigente antes de uso judicial.";
`;
writeFileSync('crates/rgps-core/src/dados/expectativa_sobrevida.rs', output, 'utf8');
console.log(JSON.stringify({ rows: rows.length, first: rows[0][0], last: rows[rows.length - 1][0], digest }));
