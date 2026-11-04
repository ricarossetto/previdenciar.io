import { describe, expect, it, vi } from 'vitest';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { JourneyGuide } from '../src/components/JourneyGuide';
import { blankCase } from '../src/model';
import type { Calculation, Cnis } from '../src/model';

function fixtureCnis(): Cnis {
  return {
    metadata: { page_count: 3, source_sha256: 'abcdef0123456789abcdef', extractor: 'synthetic' },
    identity: { full_name: 'Pessoa Fictícia', cpf: null, nit: null, birth_date: '1960-01-01', mother_name: null },
    salary_entries: [], salary_months: {}, links: [], benefits: [], warnings: [], findings: [],
  };
}

function blockedCase(): Calculation {
  return { ...blankCase(), name: 'Hipótese sintética', cnis: fixtureCnis(), result: { status: 'blocked', blockers: [{ code: 'sex_missing', message: 'Sintético' }] } } as Calculation;
}

describe('indicador da jornada no editor', () => {
  it('mostra etapa atual, ação principal e as três camadas de dados', async () => {
    const user = userEvent.setup();
    const onPrimary = vi.fn(), onStep = vi.fn();
    render(<JourneyGuide draft={blankCase()} step={1} onStep={onStep} onPrimary={onPrimary} />);
    const guide = screen.getByRole('region', { name: 'Etapa atual do cálculo' });
    expect(within(guide).getByRole('heading', { level: 2, name: 'CNIS' })).toBeInTheDocument();
    expect(within(guide).getByText('Etapa atual · 2 de 8')).toBeInTheDocument();
    const action = within(guide).getByRole('button', { name: 'Importar o CNIS' });
    await user.click(action);
    expect(onPrimary).toHaveBeenCalledTimes(1);
    const layers = within(guide).getByRole('region', { name: 'Dados da hipótese' });
    expect(within(layers).getByRole('heading', { name: 'Importado do CNIS' })).toBeInTheDocument();
    expect(within(layers).getByRole('heading', { name: 'Confirmado por você' })).toBeInTheDocument();
    expect(within(layers).getByRole('heading', { name: 'Calculado pelo motor' })).toBeInTheDocument();
    expect(within(layers).getAllByText('Ainda vazio').length).toBeGreaterThanOrEqual(2);
  });

  it('agrupa pendências por próxima ação e leva à etapa correspondente', async () => {
    const user = userEvent.setup();
    const onStep = vi.fn();
    render(<JourneyGuide draft={blankCase()} step={1} onStep={onStep} onPrimary={() => {}} />);
    const pendencies = screen.getByRole('region', { name: 'Pendências por próxima ação' });
    expect(within(pendencies).getByText(/item\(ns\) de conferência em/)).toBeInTheDocument();
    await user.click(within(pendencies).getByRole('button', { name: 'Abrir CNIS: Aplicar a origem CNIS' }));
    expect(onStep).toHaveBeenCalledWith(1);
    await user.click(within(pendencies).getByRole('button', { name: 'Abrir Cenário: Completar identificação' }));
    expect(onStep).toHaveBeenCalledWith(2);
  });

  it('mantém os detalhes técnicos recolhidos por padrão', () => {
    render(<JourneyGuide draft={blankCase()} step={1} onStep={() => {}} onPrimary={() => {}} />);
    const summary = screen.getByText('Detalhes técnicos da origem e do cálculo');
    expect(summary.closest('details')).not.toHaveAttribute('open');
    expect(screen.getByText('não executado')).toBeInTheDocument();
  });

  it('no resultado bloqueado a ação principal é executar e o motor aparece como bloqueado', () => {
    render(<JourneyGuide draft={blockedCase()} step={7} onStep={() => {}} onPrimary={() => {}} primaryDisabled />);
    expect(screen.getByRole('button', { name: 'Executar a simulação' })).toBeDisabled();
    expect(screen.getAllByText('Bloqueado').length).toBeGreaterThanOrEqual(2);
    expect(screen.getByRole('button', { name: 'Abrir Resultado: Executar a simulação' })).toBeInTheDocument();
  });
});
