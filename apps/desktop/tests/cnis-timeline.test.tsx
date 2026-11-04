import { describe, expect, it } from 'vitest';
import { render, screen, within } from '@testing-library/react';
import { buildCnisTimeline, PBC_START, type CnisTimelineModel } from '../src/timeline';
import { CnisTimeline } from '../src/components/CnisTimeline';
import type { Cnis } from '../src/model';

/// Série sintética curta: 1990 (antes do recorte), 1994 (início do recorte),
/// 2000 (com lacuna em 02/2000) e 2024 (depois da DIB de 30/06/2023).
function fixture(): Cnis {
  const month = (competence: string, amount: number) => ({ count: 1, amount_sum_centavos: amount, contains_unresolved_prpps: false });
  return {
    metadata: { page_count: 2, source_sha256: null, extractor: 'synthetic' },
    identity: { full_name: null, cpf: null, nit: null, birth_date: null, mother_name: null },
    salary_entries: [],
    salary_months: { '1990-01': month('1990-01', 100000), '1994-07': month('1994-07', 200000), '2000-01': month('2000-01', 300000), '2024-05': month('2024-05', 400000) },
    links: [], benefits: [], warnings: [], findings: [],
  };
}

function monthOf(model: CnisTimelineModel, competence: string) {
  for (const year of model.years) for (const month of year.months) if (month.competence === competence) return month;
  return null;
}

describe('visão temporal do CNIS', () => {
  it('camada a camada: recorte do PBC, lacuna, antes de 07/1994 e depois da DIB', () => {
    const model = buildCnisTimeline(fixture(), '30/06/2023');
    expect(PBC_START).toBe('1994-07');
    expect(model.dibMonth).toBe('2023-06');
    expect(model.pbcEnd).toBe('2023-05');
    expect(model.rangeStart).toBe('1990-01');
    expect(model.rangeEnd).toBe('2024-05');
    expect(model.years).toHaveLength(35);
    expect(model.years.every(year => year.months.length === 12)).toBe(true);
    expect(monthOf(model, '1990-01')).toMatchObject({ state: 'prePbc', amountCentavos: 100000 });
    expect(monthOf(model, '1994-07')).toMatchObject({ state: 'lancamento', amountCentavos: 200000 });
    expect(monthOf(model, '2000-02')).toMatchObject({ state: 'lacuna', amountCentavos: null });
    expect(monthOf(model, '2023-06')).toMatchObject({ state: 'posDib' });
    expect(monthOf(model, '2024-05')).toMatchObject({ state: 'posDib', amountCentavos: 400000 });
    expect(model.counts).toEqual({ prePbc: 54, lancamentos: 2, lacunas: 345, posDib: 12 });
  });

  it('trata ausência de competência como lacuna, nunca como salário zero', () => {
    const model = buildCnisTimeline(fixture(), '30/06/2023');
    const gap = monthOf(model, '2000-02');
    expect(gap?.state).toBe('lacuna');
    expect(gap?.amountCentavos).toBeNull();
    expect(gap?.count).toBe(0);
  });

  it('renderiza a régua com contagens, DIB e a explicação de que lacuna não é zero', () => {
    render(<CnisTimeline cnis={fixture()} dib="30/06/2023" />);
    const timeline = screen.getByRole('region', { name: 'Visão temporal da origem CNIS' });
    expect(within(timeline).getByRole('heading', { name: 'Visão temporal da origem' })).toBeInTheDocument();
    expect(within(timeline).getByText('2 no recorte · 54 antes · 12 depois · 345 lacunas')).toBeInTheDocument();
    expect(within(timeline).getByText('No mês da DIB ou depois')).toBeInTheDocument();
    expect(within(timeline).getByText('2023-05')).toBeInTheDocument();
    expect(within(timeline).getByText('Mês da DIB')).toBeInTheDocument();
    expect(within(timeline).getByText('2023-06')).toBeInTheDocument();
    expect(within(timeline).getByLabelText('2000-02: sem lançamento na origem')).toBeInTheDocument();
    expect(within(timeline).getByText(/não é salário zero/)).toBeInTheDocument();
  });

  it('mostra empty state quando a leitura não trouxe competência', () => {
    const empty = { ...fixture(), salary_months: {} };
    render(<CnisTimeline cnis={empty} dib="30/06/2023" />);
    expect(screen.getByText(/Nenhuma competência foi extraída/)).toBeInTheDocument();
  });
});
