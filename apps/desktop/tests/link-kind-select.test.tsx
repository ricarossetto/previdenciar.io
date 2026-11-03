import {useState} from 'react';
import {describe,expect,it,vi} from 'vitest';
import {render,screen,within} from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {CaseWorkbench,RgpsPolicy} from '../src/components/CaseWorkbench';
import {LINK_KINDS} from '../src/assistantReview';
import {blankCase,blankWorkspace} from '../src/model';
import type {Calculation} from '../src/model';

const basePolicy = {
  version: 'rgps-core-observational-v1',
  review_evidence: 'DOC-SINTETICO',
  manual_link_kind: null as string | null,
  manual_currency: 'BRL',
  native_options: {
    corrigirSalarios: false,
    descarte: false,
    divisorMinimo: false,
    aplicarTetoVigenteDer: false,
    referenciaCorrecao: null,
    expectativaSobrevida: null,
    regraAlvo: null,
    incapacidadeOrigem: null,
    grauEspecial: null,
    tempoEspecialMeses: null,
    especialAntesReforma: null,
    magisterioMeses: null,
    magisterioExclusivo: null,
    magisterioAntesReforma: null,
  },
  exclude_imported_source: false,
  source_exclusion_evidence: null,
  identity_confirmed: false,
};

function policyValue(kind: string | null = null) {
  return JSON.stringify({...basePolicy, manual_link_kind: kind});
}

function withoutKind(policy: Record<string, unknown>) {
  const copy = {...policy};
  delete copy.manual_link_kind;
  return copy;
}

function Harness({initial}:{initial:string | null}) {
  const [value,setValue] = useState(() => policyValue(initial));
  return <RgpsPolicy value={value} onChange={setValue} hasSource={false}/>;
}

function WorkbenchHarness() {
  const [draft,setDraft] = useState<Calculation>(() => ({
    ...blankCase(),
    options: {...blankCase().options, rule_policy: policyValue('normal')},
    manual_links: [{
      id: 7,
      employer: 'Empregador sintético',
      start_date: '2020-01-01',
      end_date: '2020-12-31',
      regime: 'RGPS',
      indicators: 'IREM-INDPEND',
      confirmed: true,
    }],
  }));
  return <>
    <CaseWorkbench draft={draft} setDraft={setDraft} workspace={blankWorkspace()} onSave={async () => {}} onNotify={() => {}} initialStep={5} embedded/>
    <pre data-testid="links">{JSON.stringify(draft.manual_links)}</pre>
    <pre data-testid="policy">{draft.options.rule_policy}</pre>
  </>;
}

describe('categoria dos períodos manuais', () => {
  it('oferece exatamente as cinco categorias aceitas pelo adaptador local', () => {
    render(<RgpsPolicy value={policyValue()} onChange={() => {}} hasSource={false}/>);
    const select = screen.getByLabelText('Categoria dos períodos declarados');
    const values = within(select).getAllByRole('option')
      .map(option => (option as HTMLOptionElement).value)
      .filter(Boolean);
    expect(values).toEqual([...LINK_KINDS]);
    for (const unsupported of ['normal','individual','rural','especial','servidor']) {
      expect(values).not.toContain(unsupported);
    }
    expect(screen.getByRole('option',{name:'Empregado doméstico'})).toBeInTheDocument();
    expect(screen.getByRole('option',{name:'Contribuinte individual'})).toBeInTheDocument();
  });

  it.each(LINK_KINDS)('selecionar %s altera somente manual_link_kind', async kind => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<RgpsPolicy value={policyValue()} onChange={onChange} hasSource={false}/>);
    await user.selectOptions(screen.getByLabelText('Categoria dos períodos declarados'), kind);
    const raw = onChange.mock.calls.at(-1)?.[0] as string;
    const next = JSON.parse(raw) as Record<string, unknown>;
    expect(next.manual_link_kind).toBe(kind);
    expect(withoutKind(next)).toEqual(withoutKind(basePolicy));
  });

  it.each(['normal','individual','rural','especial','servidor'])('preserva o legado %s visível e pendente até escolha humana', async legacy => {
    const user = userEvent.setup();
    render(<Harness initial={legacy}/>);
    const select = screen.getByLabelText('Categoria dos períodos declarados') as HTMLSelectElement;
    expect(select.value).toBe(legacy);
    const option = within(select).getByRole('option',{name:new RegExp('^'+legacy+' — legado')});
    expect(option).toBeDisabled();
    expect(screen.getByText(/Categoria legada preservada e pendente/)).toBeInTheDocument();
    expect(screen.getByText(/nada foi convertido automaticamente/)).toBeInTheDocument();
    expect(screen.getByText(/Rural, tempo especial e servidor sem CTC/)).toBeInTheDocument();
    await user.selectOptions(select, 'empregado');
    expect(select.value).toBe('empregado');
    expect(screen.queryByText(/Categoria legada preservada e pendente/)).not.toBeInTheDocument();
  });

  it('preserva vínculo, indicadores e confirmação ao trocar a categoria no cálculo', async () => {
    const user = userEvent.setup();
    render(<WorkbenchHarness/>);
    const linksBefore = screen.getByTestId('links').textContent;
    const select = screen.getByLabelText('Categoria dos períodos declarados') as HTMLSelectElement;
    expect(select.value).toBe('normal');
    await user.selectOptions(select, 'empregado');
    expect(screen.getByTestId('links').textContent).toBe(linksBefore);
    const policy = JSON.parse(screen.getByTestId('policy').textContent ?? '{}') as {manual_link_kind?:string};
    expect(policy.manual_link_kind).toBe('empregado');
  });
});
