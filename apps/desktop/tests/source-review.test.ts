import { describe, expect, it } from 'vitest';
import { blankCase, prepareEffectiveLinksForReview, prepareEffectiveWagesForReview } from '../src/model';
import type { Cnis, SalaryEntry } from '../src/model';

function entry(competence: string, amount: number, link: number, kind?: SalaryEntry['amount_kind']): SalaryEntry {
  return {
    competence,
    amount_centavos: amount,
    indicators: [],
    link_sequence: link,
    source: { page: 1, line: 1, index: 1 },
    excluded_from_validated_domain: false,
    ...(kind ? { amount_kind: kind } : {}),
  };
}

function syntheticCnis(): Cnis {
  const entries: SalaryEntry[] = [
    entry('2019-01', 90000, 1),
    entry('2020-06', 50000, 2),
    entry('2022-01', 100000, 1),
    entry('2022-01', 50000, 1, 'contribution_column_unresolved'),
    entry('2022-02', 70000, 1, 'contribution_column_unresolved'),
    entry('2023-05', 200000, 1),
    entry('2023-06', 210000, 1),
    entry('2023-07', 220000, 1),
  ];
  return {
    metadata: { page_count: 1, source_sha256: 'synthetic', extractor: 'synthetic' },
    identity: { full_name: null, cpf: null, nit: null, birth_date: null, mother_name: null },
    salary_entries: entries,
    salary_months: {
      '2019-01': { count: 1, amount_sum_centavos: 90000, contains_unresolved_prpps: false },
      '2020-06': { count: 1, amount_sum_centavos: 50000, contains_unresolved_prpps: true },
      '2022-01': { count: 2, amount_sum_centavos: 150000, contains_unresolved_prpps: false },
      '2022-02': { count: 1, amount_sum_centavos: 70000, contains_unresolved_prpps: false },
      '2023-05': { count: 1, amount_sum_centavos: 200000, contains_unresolved_prpps: false },
      '2023-06': { count: 1, amount_sum_centavos: 210000, contains_unresolved_prpps: false },
      '2023-07': { count: 1, amount_sum_centavos: 220000, contains_unresolved_prpps: false },
    },
    links: [
      {
        sequence: 1,
        raw_employer: 'Empregador RGPS',
        affiliation: 'Empregado',
        start_date: '2022-01-01',
        end_date: '2026-05-31',
        prpps: false,
        indicators: [],
        reported_last_salary_month: null,
        inferred_last_salary_end: null,
        source: { page: 1 },
      },
      {
        sequence: 2,
        raw_employer: 'Regime proprio',
        affiliation: 'Servidor',
        start_date: '2020-01-01',
        end_date: '2020-12-31',
        prpps: true,
        indicators: ['PRPPS'],
        reported_last_salary_month: null,
        inferred_last_salary_end: null,
        source: { page: 1 },
      },
    ],
    benefits: [],
    warnings: [],
    findings: [],
  };
}

describe('preparacao efetiva a partir do CNIS', () => {
  it('limita o vinculo a DIB e preserva o fim original na proveniencia', () => {
    const c = blankCase();
    c.dib = '30/06/2023';
    c.cnis = syntheticCnis();
    const links = prepareEffectiveLinksForReview(c);
    expect(links).toHaveLength(2);
    expect(links[0].end_date).toBe('2023-06-30');
    expect(links[0].confirmed).toBe(false);
    expect(links[0].source_review?.excluded).toBe(false);
    expect(links[0].source_review?.history[0].before.end_date).toBe('2026-05-31');
    expect(links[0].source_review?.history[0].after.end_date).toBe('2023-06-30');
    expect(links[1].source_review?.excluded).toBe(true);
  });

  it('leva ao PBC somente remuneracao RGPS coberta e anterior a DIB', () => {
    const c = blankCase();
    c.dib = '30/06/2023';
    c.cnis = syntheticCnis();
    const wages = prepareEffectiveWagesForReview(c);
    expect(wages.map(wage => wage.competence)).toEqual(['2022-01', '2023-05']);
    expect(wages.map(wage => wage.amount)).toEqual(['1000,00', '2000,00']);
    expect(wages.every(wage => wage.confirmed === false)).toBe(true);
  });

  it('nao transforma competencia sem remuneracao em zero', () => {
    const c = blankCase();
    c.dib = '30/06/2023';
    c.cnis = syntheticCnis();
    const wages = prepareEffectiveWagesForReview(c);
    expect(wages.some(wage => wage.competence === '2022-02')).toBe(false);
    expect(wages.some(wage => wage.amount === '0,00')).toBe(false);
  });
});
