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
  windowMonths: number; withSalaryInPbc: number;
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
  const remuneration = new Map<string, { count: number; amount: number | null; pages: number[] }>();
  for (const entry of cnis.salary_entries ?? []) {
    if (entry.amount_kind !== undefined && entry.amount_kind !== 'remuneration') continue;
    const row = remuneration.get(entry.competence) ?? { count: 0, amount: 0, pages: [] };
    row.count += 1;
    row.amount = row.amount === null || !Number.isSafeInteger(entry.amount_centavos) || !Number.isSafeInteger(row.amount + entry.amount_centavos)
      ? null
      : row.amount + entry.amount_centavos;
    if (!row.pages.includes(entry.source.page)) row.pages.push(entry.source.page);
    remuneration.set(entry.competence, row);
  }
  for (const [key, value] of Object.entries(cnis.salary_months ?? {})) {
    if (key.length !== 7 || key[4] !== '-') continue;
    const paid = remuneration.get(key);
    const hasEntries = (cnis.salary_entries ?? []).some(entry => entry.competence === key);
    salaries.set(key, paid
      ? { count: paid.count, amount: paid.amount, pages: paid.pages }
      : { count: hasEntries ? 0 : value.count, amount: hasEntries ? null : (Number.isSafeInteger(value.amount_sum_centavos) ? value.amount_sum_centavos : null), pages: [] });
  }
  for (const [key, paid] of remuneration) {
    if (!salaries.has(key)) salaries.set(key, { count: paid.count, amount: paid.amount, pages: paid.pages });
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

/** Pendências temporais agrupadas por família, com a próxima ação. */
export function temporalIssues(layers: readonly CompetenceLayer[]): TemporalIssueGroup[] {
  const groups: TemporalIssueGroup[] = [];
  const add = (key: TemporalIssueKey, title: string, nextAction: string, list: CompetenceLayer[]) => {
    if (list.length) groups.push({ key, title, count: list.length, competences: list.map(item => item.competence), nextAction });
  };
  add('pos_dib', 'Remuneração posterior a DIB', 'Manter a fonte integral e excluir a faixa posterior do PBC; não apagar competências.', layers.filter(item => item.relativeToDib === 'after'));
  add('sem_periodo', 'Competência sem período confirmado', 'Confirmar o vínculo que cobre a competência ou registrar decisão documental; o valor permanece pendente, não zero.', layers.filter(item => item.amountCentavos !== null && (item.periodStatus === 'unconfirmed' || item.periodStatus === 'missing')));
  add('sem_salario', 'Competência sem remuneração lançada', 'Conferir a origem (ex.: afastamento por auxílio-doença). O valor permanece desconhecido; não preencher zero.', layers.filter(item => item.salaryStatus === 'missing'));
  add('mes_dib', 'Competência do mês da DIB', 'Decidir documentalmente sobre a competência da DIB, sem presumir inclusão ou exclusão.', layers.filter(item => item.salaryStatus === 'dib_month'));
  add('abaixo_minimo', 'Competência dispensada por valor mínimo', 'Registrar a dispensa calculada e manter o lançamento na fonte para conferência.', layers.filter(item => item.pbcStatus === 'excluded_below_minimum' || item.salaryStatus === 'zero'));
  return groups;
}
export function temporalSummary(layers: readonly CompetenceLayer[], window: PbcWindow | null): TemporalSummary {
  const withSalaryInPbc = layers.filter(item => item.inPbcWindow && item.amountCentavos !== null && item.relativeToDib !== 'after').length;
  return {
    total: layers.length,
    inPbc: layers.filter(item => item.pbcStatus === 'in_pbc' || item.pbcStatus === 'discarded' || item.pbcStatus === 'excluded_below_minimum').length,
    postDib: layers.filter(item => item.relativeToDib === 'after').length,
    missingSalary: layers.filter(item => item.salaryStatus === 'missing').length,
    unconfirmed: layers.filter(item => item.amountCentavos !== null && (item.periodStatus === 'unconfirmed' || item.periodStatus === 'missing')).length,
    dibMonth: layers.filter(item => item.salaryStatus === 'dib_month').length,
    windowMonths: window ? monthsBetween(window.from, window.to).length : 0,
    withSalaryInPbc,
    firstCompetence: layers[0]?.competence ?? null,
    lastCompetence: layers[layers.length - 1]?.competence ?? null,
    window,
  };
}

export interface BlockerFamily { key: string; title: string; count: number; codes: string[]; messages: string[]; fields: string[]; nextAction: string }
const BLOCKER_FAMILIES: Record<string, { title: string; nextAction: string }> = {
  wage_after_dib: { title: 'Remuneração posterior a DIB', nextAction: 'Manter na fonte e excluir do PBC; não apagar a competência.' },
  wage_outside_confirmed_links: { title: 'Competência sem período confirmado', nextAction: 'Confirmar o vínculo que cobre a competência; o valor não vira zero.' },
  wage_in_dib_month: { title: 'Competência do mês da DIB', nextAction: 'Decidir documentalmente sobre a competência da DIB.' },
  source_end_after_dib: { title: 'Vínculo com fim posterior a DIB', nextAction: 'Confirmar o encerramento na fonte sem cortar por suposição.' },
  pbc_missing: { title: 'PBC sem competências', nextAction: 'Revisar a janela entre 07/1994 e a DIB.' },
};
/** Agrupa e deduplica bloqueios do motor por família: 35 itens x 2 regras viram 2 famílias com contagem. */
export function groupEngineBlockers(issues: readonly Json[]): BlockerFamily[] {
  const families = new Map<string, BlockerFamily>();
  for (const raw of issues) {
    const issue = raw !== null && typeof raw === 'object' && !Array.isArray(raw) ? raw as Record<string, Json> : {};
    const code = String(issue.code ?? '') || 'sem_codigo';
    const known = BLOCKER_FAMILIES[code];
    const key = known ? code : 'outros';
    const family = families.get(key) ?? { key, title: known ? known.title : 'Outras pendências do motor', count: 0, codes: [], messages: [], fields: [], nextAction: known ? known.nextAction : 'Conferir a trilha completa do bloqueio.' };
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
/** Sem resultado válido, explica a causa e a próxima ação em vez de exibir valor fictício. */
export function calculationNotDone(c: Calculation): PendingCause | null {
  if (hasFinancialResult(c)) return null;
  const result = c.result !== null && typeof c.result === 'object' && !Array.isArray(c.result) ? c.result as Record<string, Json> : null;
  if (result && result.status === 'blocked') {
    const families = groupEngineBlockers(Array.isArray(result.blockers) ? result.blockers : []);
    return {
      cause: 'O motor bloqueou a execução' + (families.length ? ': ' + families.map(f => f.count + ' x ' + f.title.toLowerCase()).join('; ') + '.' : '.'),
      nextAction: families.length ? families.map(f => f.nextAction).join(' ') : 'Conferir a trilha completa do bloqueio.',
    };
  }
  if (!monthKey(c.dib)) return { cause: 'DIB ausente ou inválida.', nextAction: 'Informe a DIB confirmada na etapa Segurado e cenário.' };
  if (!c.cnis && !(c.manual_salaries ?? []).length) return { cause: 'Nenhuma remuneração foi importada ou informada.', nextAction: 'Importar o CNIS e preparar a revisão das remunerações.' };
  if (c.contribution_review && !c.contribution_review.applied) return { cause: 'O livro de contribuições está preparado, mas não foi aplicado.', nextAction: 'Revisar e aplicar o livro de contribuições.' };
  const pending = blockers(c);
  if (pending.length) return { cause: 'Parâmetros obrigatórios pendentes: ' + pending.join('; ') + '.', nextAction: 'Completar os campos pendentes na revisão.' };
  return { cause: 'O motor ainda não produziu um resultado válido para esta hipótese.', nextAction: 'Executar o motor na etapa Resultado.' };
}
