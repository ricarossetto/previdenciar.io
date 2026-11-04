import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import { CnisTemporalView } from '../src/components/CnisTemporalView';
import type { Cnis, ManualLink } from '../src/model';

function cnisBase(): Cnis {
  return {
    metadata: { page_count: 1, source_sha256: null, extractor: 'synthetic' },
    identity: { full_name: null, cpf: null, nit: null, birth_date: null, mother_name: null },
    salary_entries: [{ competence: '2023-05', amount_centavos: 400000, indicators: [], link_sequence: 1, source: { page: 1, line: 1, index: 1 }, excluded_from_validated_domain: false }],
    salary_months: { '2023-05': { count: 1, amount_sum_centavos: 400000, contains_unresolved_prpps: false } },
    links: [{ sequence: 1, raw_employer: 'Origem sintetica', start_date: '2023-01-01', end_date: '2023-05-31', prpps: false, indicators: [], reported_last_salary_month: null, inferred_last_salary_end: null, source: { page: 1 } }],
    benefits: [], warnings: [], findings: [],
  };
}

describe('CnisTemporalView', () => {
  it('sem vinculos confirmados nao exibe zero; mostra Nao apurado', () => {
    const { container } = render(<CnisTemporalView cnis={cnisBase()} dib='2023-06-30' />);
    expect(screen.getByTestId('temporal-unconfirmed').textContent).toBe('Nao apurado');
    expect(container.querySelector('[data-group="sem_periodo"]')).toBeNull();
  });
  it('vinculo nao confirmado nao ativa contagem', () => {
    const draft: ManualLink[] = [{ confirmed: false, start_date: '2023-01-01', end_date: '2023-05-31' } as ManualLink];
    render(<CnisTemporalView cnis={cnisBase()} dib='2023-06-30' confirmedLinks={draft} />);
    expect(screen.getByTestId('temporal-unconfirmed').textContent).toBe('Nao apurado');
  });

  it('com vinculo confirmado calcula a contagem real', () => {
    const confirmed: ManualLink[] = [{ confirmed: true, start_date: '2023-01-01', end_date: '2023-05-31' } as ManualLink];
    render(<CnisTemporalView cnis={cnisBase()} dib='2023-06-30' confirmedLinks={confirmed} />);
    expect(screen.getByTestId('temporal-unconfirmed').textContent).toBe('0');
    expect(screen.queryByText('Nao apurado')).toBeNull();
  });
  it('DIB anterior a 07/1994 nao e rotulada como invalida', () => {
    render(<CnisTemporalView cnis={cnisBase()} dib='1990-01-01' />);
    expect(screen.getByText('Anterior a 07/1994 (fora do PBC)')).toBeTruthy();
    expect(screen.queryByText('DIB invalida')).toBeNull();
  });
  it('reconhece DIB no formato brasileiro', () => {
    render(<CnisTemporalView cnis={cnisBase()} dib='30/06/1990' />);
    expect(screen.getByText('Anterior a 07/1994 (fora do PBC)')).toBeTruthy();
  });
});
