import { describe, expect, it } from 'vitest';
import { blankCase } from '../src/model';
import type { Calculation, Cnis } from '../src/model';
import { journeyLayers, journeyPendencies, journeyStage, journeyTechnical, primaryJourneyAction } from '../src/journey';

/// Fixture sintética: nenhum dado real de segurado. O objetivo é fixar a regra da
/// jornada (etapa, camadas e grupos de pendência), não validar o motor Rust.
function fixtureCnis(overrides: Partial<Cnis> = {}): Cnis {
  return {
    metadata: { page_count: 2, source_sha256: 'abcdef0123456789abcdef', extractor: 'synthetic' },
    identity: { full_name: 'Pessoa Fictícia', cpf: null, nit: null, birth_date: '1960-01-01', mother_name: null },
    salary_entries: [
      { competence: '2020-01', amount_centavos: 200000, indicators: [], link_sequence: 1, source: { page: 1, line: 1, index: 1 }, excluded_from_validated_domain: false },
    ],
    salary_months: { '2020-01': { count: 1, amount_sum_centavos: 200000, contains_unresolved_prpps: false } },
    links: [{ sequence: 1, raw_employer: 'Empregador Fictício', start_date: '2019-01-01', end_date: '2020-02-29', prpps: false, indicators: [], reported_last_salary_month: '2020-02', inferred_last_salary_end: null, source: { page: 1 } }],
    benefits: [],
    warnings: [{ code: 'unresolved_prpps', message: 'Sintética', count: 1 }],
    findings: [],
    ...overrides,
  };
}

function completeCase(): Calculation {
  return {
    ...blankCase(), name: 'Hipótese sintética', cnis: fixtureCnis(),
    identity: { full_name: 'Pessoa Fictícia', cpf: null, nit: null, birth_date: '1960-01-01', mother_name: null },
    sex: 'Feminino', disability: 'Sem deficiência declarada', dib: '2024-01-01',
    manual_links: [{ id: 1, employer: 'Empregador Fictício', start_date: '2019-01-01', end_date: '2020-02-29', regime: 'RGPS', indicators: '', confirmed: true, source_review: null }],
    manual_salaries: [{ id: 2, competence: '2020-01', amount: '2000.00', indicators: '', confirmed: true }],
    options: { ...blankCase().options, correction_index: 'INPC' },
  };
}

describe('jornada: etapa e ação principal', () => {
  it('traduz o índice do editor nas oito etapas canônicas', () => {
    expect(journeyStage(0)).toMatchObject({ index: 0, title: 'Identificação', action: 'identificacao' });
    expect(journeyStage(3)).toMatchObject({ index: 3, title: 'Vínculos', action: 'vinculos' });
    expect(journeyStage(7)).toMatchObject({ index: 7, title: 'Resultado', action: 'execucao' });
    expect(journeyStage(-4).index).toBe(0);
    expect(journeyStage(99).index).toBe(7);
  });

  it('escolhe importar, navegar ou executar conforme a etapa e a origem', () => {
    expect(primaryJourneyAction(blankCase(), 1)).toMatchObject({ kind: 'importar', targetStep: 1 });
    expect(primaryJourneyAction({ ...blankCase(), cnis: fixtureCnis() }, 1)).toMatchObject({ kind: 'navegar', targetStep: 3 });
    expect(primaryJourneyAction(blankCase(), 6)).toMatchObject({ kind: 'navegar', targetStep: 7 });
    expect(primaryJourneyAction(blankCase(), 7)).toMatchObject({ kind: 'executar', targetStep: 7 });
  });
});

describe('jornada: separação importado, confirmado e calculado', () => {
  it('começa com as três camadas vazias', () => {
    const [imported, confirmed, calculated] = journeyLayers(blankCase());
    expect(imported).toMatchObject({ id: 'importado', state: 'vazio' });
    expect(confirmed).toMatchObject({ id: 'confirmado', state: 'vazio' });
    expect(calculated).toMatchObject({ id: 'calculado', state: 'vazio', stateLabel: 'Ainda vazio' });
  });

  it('importa contagens da origem sem confundir com validação', () => {
    const [imported] = journeyLayers({ ...blankCase(), cnis: fixtureCnis() });
    expect(imported.state).toBe('pronto');
    expect(imported.metrics).toEqual(expect.arrayContaining([
      { label: 'Vínculos lidos', value: '1' },
      { label: 'Competências', value: '1' },
      { label: 'Avisos da leitura', value: '1' },
    ]));
  });

  it('só marca confirmado como completo quando vínculos, remunerações e parâmetros estão conferidos', () => {
    const partial = { ...completeCase(), manual_links: [{ ...completeCase().manual_links[0], confirmed: false }] };
    expect(journeyLayers(partial)[1]).toMatchObject({ state: 'parcial' });
    const ready = completeCase();
    expect(journeyLayers(ready)[1]).toMatchObject({ state: 'pronto' });
    expect(journeyLayers(ready)[1].metrics).toEqual(expect.arrayContaining([{ label: 'Vínculos conferidos', value: '1/1' }]));
  });

  it('distingue execução bloqueada de resultado financeiro experimental', () => {
    const blocked = { ...blankCase(), result: { status: 'blocked', blockers: [{ code: 'sex_missing', message: 'Sintético' }] } } as Calculation;
    expect(journeyLayers(blocked)[2]).toMatchObject({ state: 'parcial', stateLabel: 'Bloqueado' });
    const experimental = { ...blankCase(), result: { status: 'experimental', data: { native_result: { resumo: { rmi: '1600.00' } } } } } as Calculation;
    expect(journeyLayers(experimental)[2]).toMatchObject({ state: 'pronto', stateLabel: 'Resultado experimental' });
  });
});

describe('jornada: pendências agrupadas pela próxima ação', () => {
  it('ordena os grupos pela etapa que resolve cada pendência', () => {
    const groups = journeyPendencies(blankCase());
    const steps = groups.map(group => group.step);
    expect(steps).toEqual([...steps].sort((left, right) => left - right));
    expect(groups[0]).toMatchObject({ action: 'origem', step: 1 });
    expect(groups.some(group => group.action === 'execucao' && group.step === 7)).toBe(true);
  });

  it('leva pendências de CNIS para vínculos, remunerações e opções da RMI', () => {
    const cnis = fixtureCnis({
      links: [{ sequence: 1, raw_employer: 'Empregador Fictício', start_date: '2019-01-01', end_date: null, prpps: true, indicators: ['PRPPS'], reported_last_salary_month: '2020-02', inferred_last_salary_end: null, source: { page: 1 } }],
    });
    const groups = journeyPendencies({ ...blankCase(), cnis });
    const byAction = new Map(groups.map(group => [group.action, group]));
    expect(byAction.get('vinculos')?.step).toBe(3);
    expect(byAction.get('vinculos')?.items.length).toBeGreaterThanOrEqual(2);
    expect(byAction.get('salarios')?.step).toBe(4);
    expect(byAction.get('parametros')?.items).toContain('Referência de correção das séries não declarada');
    expect(byAction.get('origem')).toBeUndefined();
  });

  it('não mantém pendência de execução depois de um resultado financeiro', () => {
    const ready = { ...completeCase(), result: { status: 'experimental', data: { native_result: { resumo: { rmi: '1600.00' } } } } } as Calculation;
    expect(journeyPendencies(ready).some(group => group.action === 'execucao')).toBe(false);
  });
});

describe('jornada: detalhes técnicos recolhidos', () => {
  it('resume a origem e o estado do motor para a seção recolhida', () => {
    const lines = journeyTechnical(completeCase());
    const byLabel = new Map(lines.map(line => [line.label, line.value]));
    expect(byLabel.get('Hash SHA-256 da origem')).toBe('abcdef0123456789…');
    expect(byLabel.get('Extrator')).toBe('synthetic');
    expect(byLabel.get('Estado do motor')).toBe('não executado');
    expect(byLabel.get('Livro de contribuições')).toBe('não preparado');
  });
});
