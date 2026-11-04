import type { Calculation, Cnis, Json, ManualLink } from './model';
import { blockers, hasFinancialResult, isoDate } from './model';

export type RelativeToDib = 'before' | 'equal' | 'after' | 'unknown';
export type SalaryStatus = 'present' | 'zero' | 'missing' | 'dib_month' | 'post_dib';
export type PeriodStatus = 'confirmed' | 'unconfirmed' | 'missing' | 'unknown';
export type PbcStatus = 'in_pbc' | 'post_dib' | 'before_window' | 'excluded_missing_salary' | 'excluded_below_minimum' | 'excluded_dib_month' | 'discarded';

export interface CompetenceLayer {
  competence: string;
  inPbcWindow: boolean;
  relativeToDib: RelativeToDib;
  salaryStatus: SalaryStatus;
  periodStatus: PeriodStatus;
  pbcStatus: PbcStatus;
  amountCentavos: number | null;
  entryCount: number;
  sourcePages: number[];
  pending: boolean;
}

export interface PbcWindow { from: string; to: string }
export interface ConfirmedRange { start: string; end: string | null }
export interface TemporalOptions {
  confirmedRanges?: readonly ConfirmedRange[];
  discarded?: readonly string[];
  belowMinimum?: readonly string[];
}
export type TemporalIssueKey = 'pos_dib' | 'sem_periodo' | 'sem_salario' | 'mes_dib' | 'abaixo_minimo';
export interface TemporalIssueGroup { key: TemporalIssueKey; title: string; count: number; competences: string[]; nextAction: string }
export interface TemporalSummary {
  total: number; inPbc: number; postDib: number; missingSalary: number; unconfirmed: number; dibMonth: number;
  firstCompetence: string | null; lastCompetence: string | null; window: PbcWindow | null;
}

const FIRST_PBC_MONTH = '1994-07';
const MAX_MONTHS = 1400;

export function monthKey(value: string | null | undefined): string | null {
  if (!value) return null;
  const iso = isoDate(value);
  return iso ? iso.slice(0, 7) : null;
}
export function previousMonth(key: string): string {
  const [year, month] = key.split('-').map(Number);
  return month === 1 ? String(year - 1).padStart(4, '0') + '-12' : String(year) + '-' + String(month - 1).padStart(2, '0');
}
export function nextMonth(key: string): string {
  const [year, month] = key.split('-').map(Number);
  return month === 12 ? String(year + 1).padStart(4, '0') + '-01' : String(year) + '-' + String(month + 1).padStart(2, '0');
}
export function monthsBetween(from: string, to: string): string[] {
  const out: string[] = [];
  let cursor = from;
  while (cursor <= to && out.length < MAX_MONTHS) { out.push(cursor); cursor = nextMonth(cursor); }
  return out;
}
export function pbcWindow(dib: string): PbcWindow | null {
  const key = monthKey(dib);
  if (!key) return null;
  const to = previousMonth(key);
  return to < FIRST_PBC_MONTH ? null : { from: FIRST_PBC_MONTH, to };
}
export function confirmedRangesOf(links: readonly ManualLink[]): ConfirmedRange[] {
  return links.flatMap(link => {
    const start = monthKey(link.start_date);
    return link.confirmed && start ? [{ start, end: monthKey(link.end_date) }] : [];
  });
}
function covered(competence: string, ranges: readonly ConfirmedRange[]): boolean {
  return ranges.some(range => range.start <= competence && range.end !== null && competence <= range.end);
}
function relativeOf(key: string, dibMonth: string | null): RelativeToDib {
  if (!dibMonth) return 'unknown';
  return key < dibMonth ? 'before' : key === dibMonth ? 'equal' : 'after';
}

/** Camada derivada e aditiva sobre a fonte do CNIS: nao altera salary_entries, salary_months nem links.
 * Competencia sem remuneracao permanece null (desconhecida), nunca zero. */
export function competenceLayers(cnis: Cnis, dib: string, options: TemporalOptions = {}): CompetenceLayer[] {
  const window = pbcWindow(dib);
  const dibMonth = monthKey(dib);
  const salaries = new Map<string, { count: number; amount: number | null; pages: number[] }>();
  for (const [key, value] of Object.entries(cnis.salary_months ?? {})) {
    if (key.length !== 7 || key[4] !== '-') continue;
    salaries.set(key, { count: value.count, amount: Number.isSafeInteger(value.amount_sum_centavos) ? value.amount_sum_centavos : null, pages: [] });
  }
  for (const entry of cnis.salary_entries ?? []) {
    const row = salaries.get(entry.competence);
    if (row && !row.pages.includes(entry.source.page)) row.pages.push(entry.source.page);
  }
  const sourceRanges = (cnis.links ?? []).flatMap(link => {
    const start = monthKey(link.start_date), end = monthKey(link.end_date);
    return start && end ? [{ start, end }] : [];
  });
  const gaps = new Set<string>();
  if (window) for (const range of sourceRanges) {
    const from = range.start > window.from ? range.start : window.from;
    const to = range.end < window.to ? range.end : window.to;
    if (from > to) continue;
    for (const key of monthsBetween(from, to)) if (!salaries.has(key)) gaps.add(key);
  }
  const discarded = new Set(options.discarded ?? []);
  const belowMinimum = new Set(options.belowMinimum ?? []);
  const competencies = [...new Set([...salaries.keys(), ...gaps])].sort((a, b) => a.localeCompare(b));
  return competencies.map(competence => {
    const salary = salaries.get(competence);
    const amount = salary ? salary.amount : null;
    const relativeToDib = relativeOf(competence, dibMonth);
    const inPbcWindow = Boolean(window && competence >= window.from && competence <= window.to);
    const salaryStatus: SalaryStatus = relativeToDib === 'after' ? 'post_dib'
      : relativeToDib === 'equal' ? 'dib_month'
      : amount === null ? 'missing' : amount === 0 ? 'zero' : 'present';
    const periodStatus: PeriodStatus = options.confirmedRanges
      ? covered(competence, options.confirmedRanges) ? 'confirmed' : covered(competence, sourceRanges) ? 'unconfirmed' : 'missing'
      : 'unknown';
    const pbcStatus: PbcStatus = relativeToDib === 'after' ? 'post_dib'
      : relativeToDib === 'equal' ? 'excluded_dib_month'
      : !inPbcWindow ? 'before_window'
      : amount === null ? 'excluded_missing_salary'
      : belowMinimum.has(competence) ? 'excluded_below_minimum'
      : discarded.has(competence) ? 'discarded'
      : 'in_pbc';
    const pending = relativeToDib === 'after' || salaryStatus === 'missing' || salaryStatus === 'dib_month' || salaryStatus === 'zero'
      || (amount !== null && (periodStatus === 'unconfirmed' || periodStatus === 'missing'));
    return { competence, inPbcWindow, relativeToDib, salaryStatus, periodStatus, pbcStatus,
      amountCentavos: amount, entryCount: salary ? salary.count : 0, sourcePages: salary ? [...salary.pages] : [], pending };
  });
}

/** Pendencias temporais agrupadas por familia, com a proxima acao. */
export function temporalIssues(layers: readonly CompetenceLayer[]): TemporalIssueGroup[] {
  const groups: TemporalIssueGroup[] = [];
  const add = (key: TemporalIssueKey, title: string, nextAction: string, list: CompetenceLayer[]) => {
    if (list.length) groups.push({ key, title, count: list.length, competences: list.map(item => item.competence), nextAction });
  };
  add('pos_dib', 'Remuneracao posterior a DIB', 'Manter a fonte integral e excluir a faixa posterior do PBC; nao apagar competencias.', layers.filter(item => item.relativeToDib === 'after'));
  add('sem_periodo', 'Competencia sem periodo confirmado', 'Confirmar o vinculo que cobre a competencia ou registrar decisao documental; o valor permanece pendente, nao zero.', layers.filter(item => item.amountCentavos !== null && (item.periodStatus === 'unconfirmed' || item.periodStatus === 'missing')));
  add('sem_salario', 'Competencia sem remuneracao lancada', 'Conferir a origem (ex.: afastamento por auxilio-doenca). O valor permanece desconhecido; nao preencher zero.', layers.filter(item => item.salaryStatus === 'missing'));
  add('mes_dib', 'Competencia do mes da DIB', 'Decidir documentalmente sobre a competencia da DIB, sem presumir inclusao ou exclusao.', layers.filter(item => item.salaryStatus === 'dib_month'));
  add('abaixo_minimo', 'Competencia dispensada por valor minimo', 'Registrar a dispensa calculada e manter o lancamento na fonte para conferencia.', layers.filter(item => item.pbcStatus === 'excluded_below_minimum' || item.salaryStatus === 'zero'));
  return groups;
}
export function temporalSummary(layers: readonly CompetenceLayer[], window: PbcWindow | null): TemporalSummary {
  return {
    total: layers.length,
    inPbc: layers.filter(item => item.pbcStatus === 'in_pbc' || item.pbcStatus === 'discarded' || item.pbcStatus === 'excluded_below_minimum').length,
    postDib: layers.filter(item => item.relativeToDib === 'after').length,
    missingSalary: layers.filter(item => item.salaryStatus === 'missing').length,
    unconfirmed: layers.filter(item => item.amountCentavos !== null && (item.periodStatus === 'unconfirmed' || item.periodStatus === 'missing')).length,
    dibMonth: layers.filter(item => item.salaryStatus === 'dib_month').length,
    firstCompetence: layers[0]?.competence ?? null,
    lastCompetence: layers[layers.length - 1]?.competence ?? null,
    window,
  };
}

export interface BlockerFamily { key: string; title: string; count: number; codes: string[]; messages: string[]; fields: string[]; nextAction: string }
const BLOCKER_FAMILIES: Record<string, { title: string; nextAction: string }> = {
  wage_after_dib: { title: 'Remuneracao posterior a DIB', nextAction: 'Manter na fonte e excluir do PBC; nao apagar a competencia.' },
  wage_outside_confirmed_links: { title: 'Competencia sem periodo confirmado', nextAction: 'Confirmar o vinculo que cobre a competencia; o valor nao vira zero.' },
  wage_in_dib_month: { title: 'Competencia do mes da DIB', nextAction: 'Decidir documentalmente sobre a competencia da DIB.' },
  source_end_after_dib: { title: 'Vinculo com fim posterior a DIB', nextAction: 'Confirmar o encerramento na fonte sem cortar por suposicao.' },
  pbc_missing: { title: 'PBC sem competencias', nextAction: 'Revisar a janela entre 07/1994 e a DIB.' },
};
/** Agrupa e deduplica bloqueios do motor por familia: 35 itens x 2 regras viram 2 familias com contagem. */
export function groupEngineBlockers(issues: readonly Json[]): BlockerFamily[] {
  const families = new Map<string, BlockerFamily>();
  for (const raw of issues) {
    const issue = raw !== null && typeof raw === 'object' && !Array.isArray(raw) ? raw as Record<string, Json> : {};
    const code = String(issue.code ?? '') || 'sem_codigo';
    const known = BLOCKER_FAMILIES[code];
    const key = known ? code : 'outros';
    const family = families.get(key) ?? { key, title: known ? known.title : 'Outras pendencias do motor', count: 0, codes: [], messages: [], fields: [], nextAction: known ? known.nextAction : 'Conferir a trilha completa do bloqueio.' };
    family.count += 1;
    if (!family.codes.includes(code)) family.codes.push(code);
    const message = typeof issue.message === 'string' && issue.message.trim() ? issue.message.trim() : '';
    if (message && !family.messages.includes(message)) family.messages.push(message);
    const field = typeof issue.field === 'string' ? issue.field : '';
    if (field && !family.fields.includes(field)) family.fields.push(field);
    families.set(key, family);
  }
  return [...families.values()];
}

export interface PendingCause { cause: string; nextAction: string }
/** Sem resultado valido, explica a causa e a proxima acao em vez de exibir valor ficticio. */
export function calculationNotDone(c: Calculation): PendingCause | null {
  if (hasFinancialResult(c)) return null;
  const result = c.result !== null && typeof c.result === 'object' && !Array.isArray(c.result) ? c.result as Record<string, Json> : null;
  if (result && result.status === 'blocked') {
    const families = groupEngineBlockers(Array.isArray(result.blockers) ? result.blockers : []);
    return {
      cause: 'O motor bloqueou a execucao' + (families.length ? ': ' + families.map(f => f.count + ' x ' + f.title.toLowerCase()).join('; ') + '.' : '.'),
      nextAction: families.length ? families.map(f => f.nextAction).join(' ') : 'Conferir a trilha completa do bloqueio.',
    };
  }
  if (!monthKey(c.dib)) return { cause: 'DIB ausente ou invalida.', nextAction: 'Informe a DIB confirmada na etapa Segurado e cenario.' };
  if (!c.cnis && !(c.manual_salaries ?? []).length) return { cause: 'Nenhuma remuneracao foi importada ou informada.', nextAction: 'Importar o CNIS e preparar a revisao das remuneracoes.' };
  if (c.contribution_review && !c.contribution_review.applied) return { cause: 'O livro de contribuicoes esta preparado, mas nao foi aplicado.', nextAction: 'Revisar e aplicar o livro de contribuicoes.' };
  const pending = blockers(c);
  if (pending.length) return { cause: 'Parametros obrigatorios pendentes: ' + pending.join('; ') + '.', nextAction: 'Completar os campos pendentes na revisao.' };
  return { cause: 'O motor ainda nao produziu um resultado valido para esta hipotese.', nextAction: 'Executar o motor na etapa Resultado.' };
}
