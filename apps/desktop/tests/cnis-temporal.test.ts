import { describe, expect, it } from 'vitest';
import { blankCase } from '../src/model';
import type { Cnis, Json, ManualLink } from '../src/model';
import { calculationNotDone, competenceLayers, confirmedRangesOf, groupEngineBlockers, pbcWindow, temporalIssues, temporalSummary } from '../src/temporal';

const DIB = '2023-06-30';
const CONFIRMED = [{ start: '2022-01', end: '2023-05' }];

function addMonth(key: string): string {
  const parts = key.split('-').map(Number);
  return parts[1] === 12 ? parts[0] + 1 + '-01' : parts[0] + '-' + String(parts[1] + 1).padStart(2, '0');
}
function monthRange(from: string, to: string): string[] {
  const out: string[] = [];
  let key = from;
  while (key <= to) { out.push(key); key = addMonth(key); }
  return out;
}
const POST_DIB = monthRange('2023-07', '2026-05');
const PRE_DIB = monthRange('2022-01', '2023-05');
const GAP = monthRange('2018-09', '2019-08');

function syntheticCnis(withGap: boolean): Cnis {
  const entries = [...PRE_DIB, ...POST_DIB].map((competence, index) => ({
    competence, amount_centavos: 400000, indicators: [], link_sequence: 1,
    source: { page: 1, line: index + 1, index: index + 1 }, excluded_from_validated_domain: false,
  }));
  const salary_months: Cnis['salary_months'] = {};
  for (const competence of [...PRE_DIB, ...POST_DIB]) salary_months[competence] = { count: 1, amount_sum_centavos: 400000, contains_unresolved_prpps: false };
  const links: Cnis['links'] = [{ sequence: 1, raw_employer: 'Origem sintetica', start_date: '2022-01-01', end_date: '2026-05-31', prpps: false, indicators: [], reported_last_salary_month: null, inferred_last_salary_end: null, source: { page: 1 } }];
  if (withGap) links.push({ sequence: 2, raw_employer: 'Afastamento sintetico', start_date: '2018-09-01', end_date: '2019-08-31', prpps: false, indicators: [], reported_last_salary_month: null, inferred_last_salary_end: null, source: { page: 2 } });
  return { metadata: { page_count: 2, source_sha256: null, extractor: 'synthetic' }, identity: { full_name: null, cpf: null, nit: null, birth_date: null, mother_name: null }, salary_entries: entries, salary_months, links, benefits: [], warnings: [], findings: [] };
}

describe('visao temporal do CNIS sem apagar a fonte', () => {
  it('separa 35 competencias pos-DIB e agrupa em duas familias de 35', () => {
    const cnis = syntheticCnis(false);
    const before = JSON.stringify(cnis);
    expect(POST_DIB).toHaveLength(35);
    expect(PRE_DIB).toHaveLength(17);
    const layers = competenceLayers(cnis, DIB, { confirmedRanges: CONFIRMED });
    const post = layers.filter(layer => layer.relativeToDib === 'after');
    expect(post).toHaveLength(35);
    expect(post.every(layer => !layer.inPbcWindow && layer.pbcStatus === 'post_dib' && layer.amountCentavos === 400000 && layer.pending)).toBe(true);
    expect(post[0].competence).toBe('2023-07');
    expect(post[34].competence).toBe('2026-05');
    const issues = temporalIssues(layers);
    expect(issues.find(group => group.key === 'pos_dib')?.count).toBe(35);
    expect(issues.find(group => group.key === 'sem_periodo')?.count).toBe(35);
    expect(issues.every(group => group.nextAction.length > 0)).toBe(true);
    const window = pbcWindow(DIB);
    expect(window).toEqual({ from: '1994-07', to: '2023-05' });
    const summary = temporalSummary(layers, window);
    expect(summary.inPbc).toBe(17);
    expect(summary.postDib).toBe(35);
    expect(summary.unconfirmed).toBe(35);
    expect(JSON.stringify(cnis)).toBe(before);
    expect(cnis.salary_entries).toHaveLength(52);
  });
  it('70 bloqueios do motor viram 2 familias com contagem e proxima acao', () => {
    const issues: Json[] = [];
    for (let index = 0; index < 35; index++) {
      issues.push({ code: 'wage_after_dib', field: 'competencias[' + index + ']' });
      issues.push({ code: 'wage_outside_confirmed_links', field: 'competencias[' + index + ']' });
    }
    expect(issues).toHaveLength(70);
    const families = groupEngineBlockers(issues);
    expect(families).toHaveLength(2);
    expect(families.reduce((sum, family) => sum + family.count, 0)).toBe(70);
    expect(families.find(family => family.key === 'wage_after_dib')?.count).toBe(35);
    expect(families.find(family => family.key === 'wage_outside_confirmed_links')?.count).toBe(35);
    expect(families.every(family => family.nextAction.length > 0)).toBe(true);
  });
  it('borda da DIB: mes igual e pendente, anterior no PBC e posterior fora', () => {
    const cnis = syntheticCnis(false);
    cnis.salary_months['2023-06'] = { count: 1, amount_sum_centavos: 400000, contains_unresolved_prpps: false };
    cnis.salary_entries.push({ competence: '2023-06', amount_centavos: 400000, indicators: [], link_sequence: 1, source: { page: 1, line: 99, index: 99 }, excluded_from_validated_domain: false });
    const layers = competenceLayers(cnis, DIB, { confirmedRanges: CONFIRMED });
    const may = layers.find(layer => layer.competence === '2023-05')!;
    const june = layers.find(layer => layer.competence === '2023-06')!;
    const july = layers.find(layer => layer.competence === '2023-07')!;
    expect(may.relativeToDib).toBe('before');
    expect(may.inPbcWindow).toBe(true);
    expect(may.pbcStatus).toBe('in_pbc');
    expect(june.relativeToDib).toBe('equal');
    expect(june.inPbcWindow).toBe(false);
    expect(june.salaryStatus).toBe('dib_month');
    expect(june.pbcStatus).toBe('excluded_dib_month');
    expect(june.pending).toBe(true);
    expect(july.relativeToDib).toBe('after');
    expect(july.pbcStatus).toBe('post_dib');
    expect(pbcWindow('31/02/2023')).toBeNull();
    const invalid = competenceLayers(cnis, '31/02/2023');
    expect(invalid.every(layer => layer.relativeToDib === 'unknown' && !layer.inPbcWindow)).toBe(true);
  });
  it('competencia sem remuneracao permanece desconhecida e agrupada, nunca zero', () => {
    const cnis = syntheticCnis(true);
    const before = JSON.stringify(cnis);
    const layers = competenceLayers(cnis, DIB, { confirmedRanges: CONFIRMED });
    const gap = layers.filter(layer => layer.competence >= GAP[0] && layer.competence <= GAP[GAP.length - 1]);
    expect(gap).toHaveLength(12);
    expect(gap.every(layer => layer.amountCentavos === null && layer.salaryStatus === 'missing' && layer.pbcStatus === 'excluded_missing_salary' && layer.pending)).toBe(true);
    expect(layers.some(layer => layer.amountCentavos === 0)).toBe(false);
    expect(temporalIssues(layers).find(group => group.key === 'sem_salario')?.count).toBe(12);
    expect(JSON.stringify(cnis)).toBe(before);
    const links: ManualLink[] = [{ id: 1, employer: 'Sintetico', start_date: '01/01/2022', end_date: '31/12/2023', regime: 'A confirmar', indicators: '', confirmed: false }];
    expect(confirmedRangesOf(links)).toEqual([]);
    expect(calculationNotDone(blankCase())?.cause).toContain('Nenhuma remuneracao');
  });
  it('sem resultado valido informa Calculo nao concluido com causa e proxima acao', () => {
    const c = blankCase();
    c.dib = '30/06/2023';
    c.cnis = syntheticCnis(false);
    const pending = calculationNotDone(c);
    expect(pending).not.toBeNull();
    expect(pending!.cause).toContain('Parametros obrigatorios pendentes');
    expect(pending!.nextAction.length).toBeGreaterThan(0);
    c.result = { status: 'blocked', blockers: [{ code: 'wage_after_dib' }, { code: 'wage_outside_confirmed_links' }] };
    const blocked = calculationNotDone(c);
    expect(blocked!.cause).toContain('bloqueou');
    expect(blocked!.cause).toContain('remuneracao posterior');
    expect(blocked!.nextAction).toContain('fonte');
    const ok = blankCase();
    ok.result = { status: 'ok', data: { resumo: { rmi: '100,00' } } };
    expect(calculationNotDone(ok)).toBeNull();
  });
});
