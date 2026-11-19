import type { Calculation } from './model';
import { blockers } from './model';

const INJECTION_PATTERNS = [
  /(?:ignore|desconsidere|esqueça|despreze)\s+(?:as\s+)?(?:instruções|instrucoes|regras|orientações|orientacoes)\s+(?:anteriores|acima|do sistema)?/i,
  /(?:não|nao)\s+siga\s+(?:as\s+)?(?:instruções|instrucoes|regras)/i,
  /system\s+prompt/i,
  /(?:revele|mostre|imprima|repita)\s+(?:suas\s+)?(?:instruções|instrucoes|regras|prompt|chave|senha)/i,
  /you\s+are\s+now\s+/i,
  /(?:execute|rode)\s+(?:o\s+)?(?:comando|script|código|codigo)/i,
];

export function detectPromptInjection(text: string): string | null {
  const normalized = text.normalize('NFKC');
  for (const pattern of INJECTION_PATTERNS) {
    const match = normalized.match(pattern);
    if (match) return match[0].slice(0, 120);
  }
  return null;
}

export function caseSummary(c: Calculation): string {
  const result = c.result as { status?: string } | null;
  const months = c.cnis?.salary_months ? Object.keys(c.cnis.salary_months).length : 0;
  return [
    `Módulo: ${c.kind || 'não informado'}`,
    `Cenário: ${c.scenario || 'não informado'}`,
    `DIB: ${c.dib || 'não informada'}`,
    `Sexo cadastral: ${c.sex || 'não informado'}`,
    `Condição de deficiência: ${c.disability || 'não informada'}`,
    `Status do registro: ${c.status || 'não informado'}`,
    `Resultado do motor: ${result?.status || 'não calculado'}`,
    `Vínculos no CNIS: ${c.cnis?.links.length ?? 0}`,
    `Remunerações no CNIS: ${c.cnis?.salary_entries.length ?? 0}`,
    `Competências agrupadas: ${months}`,
    `Pendências estruturais: ${blockers(c).join('; ') || 'nenhuma listada'}`,
    'Observação: resumo sem identificação direta, rendimentos, notas ou textos livres.',
  ].join('\n');
}
