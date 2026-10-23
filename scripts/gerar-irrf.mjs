// Gera crates/rgps-core/src/dados/irrf.rs a partir da tabela padrão do produto
// de referência (export REA). O CSV não entra no repositório: passe o caminho
// como argumento. Valida a tabela vigente da Lei 14.848/2024 antes de emitir.
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';

const source = process.argv[2];
if (!source) {
  console.error('Uso: node scripts/gerar-irrf.mjs <tabelairrfpadrao.csv>');
  process.exit(2);
}
const text = readFileSync(source, 'latin1').split(/\r?\n/).filter((line) => line.trim());
const header = text[0].split(/[;,]/).map((name) => name.trim());
const column = (name) => {
  const index = header.indexOf(name);
  if (index < 0) throw new Error(`Coluna ausente: ${name}`);
  return index;
};
const delimiter = text[0].includes(';') ? ';' : ',';
const cData = column('Data');
const cIsencao = column('FimFaixa1');
const cFim2 = column('FimFaixa2');
const cFim3 = column('FimFaixa3');
const cFim4 = column('FimFaixa4');
const cPerc = [2, 3, 4, 5].map((n) => column(`PercFaixa${n}`));
const cDed = [2, 3, 4, 5].map((n) => column(`DeduFaixa${n}`));
const cDep = column('Dependente');

const rows = [];
for (const line of text.slice(1)) {
  const fields = line.split(delimiter);
  const data = fields[cData];
  if (!/^\d{4}-\d{2}-\d{2}$/.test(data)) throw new Error(`Data inválida: ${data}`);
  const num = (index) => {
    const value = Number(fields[index]);
    if (!Number.isFinite(value) || value < 0) throw new Error(`Valor inválido em ${data}: ${fields[index]}`);
    return value;
  };
  rows.push({
    data,
    isencao: num(cIsencao),
    limites: [num(cFim2), num(cFim3), num(cFim4)],
    aliquotas: cPerc.map((index) => num(index)),
    deducoes: cDed.map((index) => num(index)),
    dependente: num(cDep),
  });
}
for (let i = 1; i < rows.length; i += 1) {
  if (rows[i - 1].data >= rows[i].data) throw new Error('Datas fora de ordem.');
}
// Sanidade da tabela vigente (Lei 14.848/2024): isenção 2.428,80; faixas
// 2.826,65 / 3.751,05 / 4.664,68; deduções 182,16 / 394,16 / 675,49 / 908,73.
const vigente = rows.filter((row) => row.data >= '2026-01-01').pop();
if (!vigente) throw new Error('Sem linha vigente.');
const fix = (value) => value.toFixed(2);
if (fix(vigente.isencao) !== '2428.80') throw new Error(`Isenção vigente inesperada: ${fix(vigente.isencao)}`);
if (fix(vigente.dependente) !== '189.59') throw new Error(`Dependente vigente inesperado: ${fix(vigente.dependente)}`);
const esperado = { limites: ['2826.65', '3751.05', '4664.68'], aliquotas: ['7.50', '15.00', '22.50', '27.50'], deducoes: ['182.16', '394.16', '675.49', '908.73'] };
if (vigente.limites.map(fix).join(',') !== esperado.limites.join(',')) throw new Error('Faixas vigentes divergem da Lei 14.848/2024.');
if (vigente.aliquotas.map(fix).join(',') !== esperado.aliquotas.join(',')) throw new Error('Alíquotas vigentes divergem da Lei 14.848/2024.');
if (vigente.deducoes.map(fix).join(',') !== esperado.deducoes.join(',')) throw new Error('Deduções vigentes divergem da Lei 14.848/2024.');

const digest = createHash('sha256').update(JSON.stringify(rows)).digest('hex');
const faixas = (row) => {
  const faixa = (limite, aliquota, deducao) => `FaixaIrrf { limite: "${limite}", aliquota: "${aliquota}", deducao: "${deducao}" }`;
  return [
    faixa(fix(row.isencao), '0.00', '0.00'),
    faixa(fix(row.limites[0]), fix(row.aliquotas[0]), fix(row.deducoes[0])),
    faixa(fix(row.limites[1]), fix(row.aliquotas[1]), fix(row.deducoes[1])),
    faixa(fix(row.limites[2]), fix(row.aliquotas[2]), fix(row.deducoes[2])),
    faixa('0', fix(row.aliquotas[3]), fix(row.deducoes[3])),
  ].join(', ');
};
const body = rows
  .map((row) => `    TabelaIrrf { competencia: "${row.data.slice(0, 7)}", faixas: [${faixas(row)}], dependente: "${fix(row.dependente)}" },`)
  .join('\n');
const output = `// GERADO AUTOMATICAMENTE por scripts/gerar-irrf.mjs a partir da tabela padrão
// do produto de referência (export REA). Não editar à mão.
// Série: SHA-256 ${digest}
// Cobertura: ${rows[0].data.slice(0, 7)} a ${rows[rows.length - 1].data.slice(0, 7)} (${rows.length} competências).
// A última faixa tem limite "0" e representa "acima da anterior".

pub struct FaixaIrrf {
    pub limite: &'static str,
    pub aliquota: &'static str,
    pub deducao: &'static str,
}

pub struct TabelaIrrf {
    pub competencia: &'static str,
    pub faixas: [FaixaIrrf; 5],
    pub dependente: &'static str,
}

pub static IRRF: [TabelaIrrf; ${rows.length}] = [
${body}
];

pub const IRRF_FONTE: &str = "Tabela de IRRF mensal por competência (tabela padrão do produto de referência, export REA; faixas e deduções conferidas com a Lei 14.848/2024 na vigência atual).";
`;
writeFileSync('crates/rgps-core/src/dados/irrf.rs', output, 'utf8');
console.log(JSON.stringify({ rows: rows.length, first: rows[0].data.slice(0, 7), last: rows[rows.length - 1].data.slice(0, 7), digest }));
