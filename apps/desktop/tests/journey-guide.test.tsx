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
  it('mostra etapa atual e as três camadas de dados', async () => {
    const user = userEvent.setup();
    const onStep = vi.fn();
    render(<JourneyGuide draft={blankCase()} step={1} onStep={onStep} />);
    const guide = screen.getByRole('region', { name: 'Etapa atual do cálculo' });
    expect(within(guide).getByRole('heading', { level: 2, name: 'CNIS' })).toBeInTheDocument();
    expect(within(guide).getByText('Etapa atual · 2 de 8')).toBeInTheDocument();
    expect(within(guide).queryByRole('button', { name: 'Importar o CNIS' })).not.toBeInTheDocument();
    const layers = within(guide).getByRole('region', { name: 'Dados da hipótese' });
    expect(within(layers).getByRole('heading', { name: 'Importado do CNIS' })).toBeInTheDocument();
    expect(within(layers).getByRole('heading', { name: 'Confirmado por você' })).toBeInTheDocument();
    expect(within(layers).getByRole('heading', { name: 'Calculado pelo motor' })).toBeInTheDocument();
    expect(within(layers).getAllByText('Ainda vazio').length).toBeGreaterThanOrEqual(2);
  });

  it('agrupa pendências por próxima ação e leva à etapa correspondente', async () => {
    const user = userEvent.setup();
    const onStep = vi.fn();
    render(<JourneyGuide draft={blankCase()} step={1} onStep={onStep} />);
    const pendencies = screen.getByRole('region', { name: 'Pendências por próxima ação' });
    expect(within(pendencies).getByText(/item\(ns\) de conferência em/)).toBeInTheDocument();
    await user.click(within(pendencies).getByRole('button', { name: 'Abrir CNIS: Aplicar a origem CNIS' }));
    expect(onStep).toHaveBeenCalledWith(1);
    await user.click(within(pendencies).getByRole('button', { name: 'Abrir Identificação: Completar identificação' }));
    expect(onStep).toHaveBeenCalledWith(0);
  });

  it('mantém os detalhes técnicos recolhidos por padrão', () => {
    render(<JourneyGuide draft={blankCase()} step={1} onStep={() => {}} />);
    const summary = screen.getByText('Detalhes técnicos da origem e do cálculo');
    expect(summary.closest('details')).not.toHaveAttribute('open');
    expect(screen.getByText('não executado')).toBeInTheDocument();
  });

  it('no resultado bloqueado o motor aparece como bloqueado e a pendência leva ao resultado', () => {
    render(<JourneyGuide draft={blockedCase()} step={7} onStep={() => {}} />);
    expect(screen.getAllByText('Bloqueado').length).toBeGreaterThanOrEqual(2);
    expect(screen.getByRole('button', { name: 'Abrir Resultado: Executar a simulação' })).toBeInTheDocument();
  });
});
