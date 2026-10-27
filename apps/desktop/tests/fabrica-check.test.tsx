import {act,fireEvent,render,screen,waitFor} from '@testing-library/react';
import {beforeEach,expect,it,vi} from 'vitest';
import {useState} from 'react';
import {FabricaCheck} from '../src/components/FabricaCheck';
import {blankCase,blankWorkspace} from '../src/model';
import type {Calculation, Workspace} from '../src/model';

const invoke = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({invoke}));

function case41(): Calculation {
  const c = blankCase();
  c.identity = {...c.identity, full_name: 'Pessoa Fictícia', cpf: '00000000000', birth_date: '1956-01-10'};
  c.sex = 'Masculino';
  c.disability = 'Sem deficiência declarada';
  c.dib = '2026-01-10';
  c.especie = '41';
  c.options.rule_policy = JSON.stringify({
    version: 'rgps-core-observational-v1',
    manual_link_kind: 'empregado',
    native_options: {incapacidadeOrigem: 'comum'},
  });
  c.manual_links = [{id: 1, employer: 'Empresa Fictícia', start_date: '1994-07-01', end_date: '2019-06-30', regime: 'RGPS', indicators: '', confirmed: true}];
  c.manual_salaries = [{id: 1, competence: '1994-07', amount: '2000.00', indicators: '', confirmed: true}];
  c.result = {
    status: 'experimental',
    data: {native_result: {resumo: {rmi: '1600.00'}, parametros: {correcao_ativa: true}}},
  };
  return c;
}

const summary = {
  provider: {
    maiorRmi: '1700.00',
    moeda: {simbolo: 'R$', nome: 'Real'},
    beneficios: [{
      especie: 41,
      descricao: 'Aposentadoria por Idade',
      chaveFundamento: 'EC_103_ART_18',
      fundamento: 'EC 103, art. 18',
      dadosRmi: {rmi: '1700.00', salarioBeneficio: '2000.00', coeficiente: 0.7, divisor: 300},
      requisitos: [{nome: 'Idade', rotulo: 'idade', porExtenso: 'cumpriu o requisito idade', cumprido: true}],
    }],
    conclusao: {completo: true},
  },
  source_url: 'https://fabrica-de-calculos.app.trf3.jus.br/rgps/api/v1/calculos-rmi',
  retrieved_at: '2026-10-27T12:00:00Z',
  request_sha256: 'a'.repeat(64),
  response_sha256: 'b'.repeat(64),
  bytes: 1024,
  api: 'Fábrica de Cálculos/TRF3',
  version: 1,
};

beforeEach(() => { invoke.mockReset(); });

it('exige confirmação e mostra a RMI do provedor sem enviar identidade', async () => {
  invoke.mockResolvedValue(summary);
  const workspace: Workspace = blankWorkspace();
  workspace.settings.fabrica_api_key = 'test-key';
  function Harness() {
    const [draft, setDraft] = useState(case41());
    return <FabricaCheck draft={draft} workspace={workspace} onChange={setDraft} />;
  }
  render(<Harness />);
  expect(screen.getByText(/envia somente data de nascimento/)).toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', {name: 'Conferir na Fábrica de Cálculos'}));
  expect(screen.getByRole('dialog')).toBeInTheDocument();
  expect(screen.getByText(/Nome, CPF, NIT, CNIS, PDF e anotações ficam fora/)).toBeInTheDocument();
  await act(async () => {
    fireEvent.click(screen.getByRole('button', {name: 'Enviar e conferir'}));
  });
  await waitFor(() => expect(screen.getAllByText('R$ 1.700,00').length).toBeGreaterThan(0));
  expect(screen.getByText('Conferir novamente')).toBeInTheDocument();
  expect(invoke).toHaveBeenCalledWith('conferir_fabrica_rmi', expect.objectContaining({apiKey: 'test-key'}));
  expect(JSON.stringify(invoke.mock.calls[0])).not.toContain('Pessoa Fictícia');
  expect(JSON.stringify(invoke.mock.calls[0])).not.toContain('00000000000');
});

it('sem chave, não envia e explica onde informar', () => {
  const workspace = blankWorkspace();
  render(<FabricaCheck draft={case41()} workspace={workspace} onChange={() => {}} />);
  expect(screen.getByText(/Informe a chave da API em/)).toBeInTheDocument();
  expect(screen.getByRole('button', {name: 'Conferir na Fábrica de Cálculos'})).toBeDisabled();
  expect(invoke).not.toHaveBeenCalled();
});
