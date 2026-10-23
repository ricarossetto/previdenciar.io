// Gera crates/rgps-core/src/dados/salario_minimo.rs a partir do snapshot
// público da Fábrica de Cálculos (TRF3). O snapshot não entra no repositório:
// passe o caminho como argumento. Nenhum dado de cliente é lido.
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';

const source = process.argv[2];
if (!source) {
  console.error('Uso: node scripts/gerar-salario-minimo.mjs <snapshot.json>');
  process.exit(2);
}
const snapshot = JSON.parse(readFileSync(source, 'utf8'));
if (!Array.isArray(snapshot.limits) || snapshot.limits.length === 0) {
  throw new Error('Snapshot sem limites.');
}
const months = new Set();
const rows = snapshot.limits.map((limit) => {
  const month = String(limit.month ?? '');
  if (!/^\d{4}-\d{2}$/.test(month)) throw new Error(`Competência inválida: ${month}`);
  if (months.has(month)) throw new Error(`Competência duplicada: ${month}`);
  months.add(month);
  const value = Number(limit.minimum);
  if (!Number.isFinite(value) || value <= 0) throw new Error(`Mínimo inválido em ${month}`);
  return [month, value.toFixed(2)];
});
for (let i = 1; i < rows.length; i += 1) {
  if (rows[i - 1][0] >= rows[i][0]) throw new Error('Série fora de ordem.');
}
const digest = createHash('sha256')
  .update(JSON.stringify(rows))
  .digest('hex');
const body = rows.map(([month, value]) => `    ("${month}", "${value}"),`).join('\n');
const output = `// GERADO AUTOMATICAMENTE por scripts/gerar-salario-minimo.mjs a partir do
// snapshot público da Fábrica de Cálculos (TRF3). Não editar à mão.
// Série (competência, valor): SHA-256 ${digest}
// Cobertura: ${rows[0][0]} a ${rows[rows.length - 1][0]} (${rows.length} competências).

/// (competência, salário mínimo). Ordenado por competência.
pub static SALARIO_MINIMO: [(&str, &str); ${rows.length}] = [
${body}
];

pub const SALARIO_MINIMO_FONTE: &str = "Salário mínimo por competência — série oficial do snapshot público da Fábrica de Cálculos (TRF3), conferida com as leis e decretos de cada ano.";
`;
writeFileSync('crates/rgps-core/src/dados/salario_minimo.rs', output, 'utf8');
console.log(JSON.stringify({ months: rows.length, first: rows[0], last: rows[rows.length - 1], digest }));
