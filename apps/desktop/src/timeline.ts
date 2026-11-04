import type { Cnis } from './model';
import { isoDate } from './model';

/// Recorte local usado pelo preparo das remunerações: competências anteriores a
/// 07/1994 ficam fora do PBC. O valor não é regra jurídica; é o limite do adaptador.
export const PBC_START = '1994-07';

export type TimelineMonthState = 'lancamento' | 'lacuna' | 'posDib' | 'prePbc' | 'foraDoIntervalo';

export interface TimelineMonth { competence: string; state: TimelineMonthState; amountCentavos: number | null; count: number }
export interface TimelineYear { year: number; months: TimelineMonth[] }
export interface TimelineCounts { prePbc: number; lancamentos: number; lacunas: number; posDib: number }
export interface CnisTimelineModel { rangeStart: string; rangeEnd: string; dibMonth: string | null; pbcEnd: string | null; years: TimelineYear[]; counts: TimelineCounts; firstCompetence: string | null; lastCompetence: string | null }

function monthOf(date: string | null | undefined): string | null {
  if (!date) return null;
  const iso = isoDate(date);
  if (iso) return iso.slice(0, 7);
  const trimmed = date.trim();
  return /^\d{4}-\d{2}$/.test(trimmed) ? trimmed : null;
}

function monthIndex(competence: string): number {
  const [year, month] = competence.split('-').map(Number);
  return year * 12 + (month - 1);
}

function fromIndex(index: number): string {
  const year = Math.floor(index / 12), month = (index % 12) + 1;
  return year + '-' + String(month).padStart(2, '0');
}

const MAX_MONTHS = 720;

/// Monta a régua de competências em camadas aditivas: o que foi lido, a janela do
/// PBC, as lacunas da origem e o que ficou depois da DIB. Nada é apagado ou
/// convertido em zero; ausência continua sendo ausência.
export function buildCnisTimeline(cnis: Cnis, dib: string | null): CnisTimelineModel {
  const months = new Map<string, { amountCentavos: number; count: number }>();
  for (const [competence, summary] of Object.entries(cnis.salary_months)) {
    months.set(competence, { amountCentavos: summary.amount_sum_centavos, count: summary.count });
  }
  for (const entry of cnis.salary_entries) {
    if (!months.has(entry.competence)) months.set(entry.competence, { amountCentavos: entry.amount_centavos, count: 1 });
  }
  const competences = [...months.keys()].filter(competence => /^\d{4}-\d{2}$/.test(competence)).sort();
  const first = competences[0] ?? null, last = competences[competences.length - 1] ?? null;
  const dibMonth = monthOf(dib);
  const pbcEnd = dibMonth ? fromIndex(monthIndex(dibMonth) - 1) : null;
  const noCounts: TimelineCounts = { prePbc: 0, lancamentos: 0, lacunas: 0, posDib: 0 };
  if (!first) return { rangeStart: '', rangeEnd: '', dibMonth, pbcEnd, years: [], counts: noCounts, firstCompetence: null, lastCompetence: null };
  let startIndex = monthIndex(first), endIndex = monthIndex(last);
  if (dibMonth) { startIndex = Math.min(startIndex, monthIndex(dibMonth)); endIndex = Math.max(endIndex, monthIndex(dibMonth)); }
  if (endIndex - startIndex + 1 > MAX_MONTHS) startIndex = endIndex - MAX_MONTHS + 1;
  const counts: TimelineCounts = { prePbc: 0, lancamentos: 0, lacunas: 0, posDib: 0 };
  const years: TimelineYear[] = [];
  for (let year = Math.floor(startIndex / 12); year <= Math.floor(endIndex / 12); year += 1) {
    const bucket: TimelineYear = { year, months: [] };
    for (let month = 1; month <= 12; month += 1) {
      const index = year * 12 + (month - 1), competence = fromIndex(index), summary = months.get(competence) ?? null;
      let state: TimelineMonthState;
      if (index < startIndex || index > endIndex) state = 'foraDoIntervalo';
      else if (competence < PBC_START) state = 'prePbc';
      else if (pbcEnd && competence > pbcEnd) state = 'posDib';
      else if (summary) state = 'lancamento';
      else state = 'lacuna';
      if (state === 'prePbc') counts.prePbc += 1;
      else if (state === 'lancamento') counts.lancamentos += 1;
      else if (state === 'lacuna') counts.lacunas += 1;
      else if (state === 'posDib') counts.posDib += 1;
      bucket.months.push({ competence, state, amountCentavos: summary?.amountCentavos ?? null, count: summary?.count ?? 0 });
    }
    years.push(bucket);
  }
  return { rangeStart: fromIndex(startIndex), rangeEnd: fromIndex(endIndex), dibMonth, pbcEnd, years, counts, firstCompetence: first, lastCompetence: last };
}
