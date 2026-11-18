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

it('habilita e envia na aba RMI com dados diretos do CNIS mesmo quando draft.result é null', async () => {
  invoke.mockResolvedValue(summary);
  const workspace: Workspace = blankWorkspace();
  workspace.settings.fabrica_api_key = 'test-key';
  const c = blankCase();
  c.identity = {...c.identity, full_name: 'Pessoa CNIS Direto', cpf: '11122233344', birth_date: '1960-05-10'};
  c.sex = 'Masculino';
  c.dib = '2026-01-10';
  c.especie = '42';
  c.result = null;
  c.cnis = {
    metadata: {page_count: 1, source_sha256: 'hash-direto', extractor: 'synthetic'},
    identity: {...c.identity},
    links: [{
      sequence: 1,
      raw_employer: 'EMPREGADOR DIRETO',
      affiliation: 'Empregado',
      start_date: '2010-01-01',
      end_date: '2025-12-31',
      prpps: false,
      indicators: [],
      reported_last_salary_month: '2025-12',
      inferred_last_salary_end: null,
      source: {page: 1},
    }],
    salary_entries: [{
      competence: '2024-06',
      amount_centavos: 350000,
      indicators: [],
      link_sequence: 1,
      source: {page: 1, line: 1, index: 1},
      excluded_from_validated_domain: false,
    }],
    salary_months: {'2024-06': {count: 1, amount_sum_centavos: 350000, contains_unresolved_prpps: false}},
    benefits: [],
    warnings: [],
    findings: [],
  };

  function Harness() {
    const [draft, setDraft] = useState(c);
    return <FabricaCheck draft={draft} workspace={workspace} onChange={setDraft} />;
  }
  render(<Harness />);
  expect(screen.getByText('Entrada direta (CNIS/vínculos)')).toBeInTheDocument();
  const btn = screen.getByRole('button', {name: 'Conferir na Fábrica de Cálculos'});
  expect(btn).toBeEnabled();
  fireEvent.click(btn);
  expect(screen.getByRole('dialog')).toBeInTheDocument();
  await act(async () => {
    fireEvent.click(screen.getByRole('button', {name: 'Enviar e conferir'}));
  });
  await waitFor(() => expect(screen.getAllByText('R$ 1.700,00').length).toBeGreaterThan(0));
  expect(invoke).toHaveBeenCalledWith('conferir_fabrica_rmi', {
    input: expect.objectContaining({
      dataApuracao: '2026-01-10',
      dataNascimento: '1960-05-10',
      sexo: 'M',
      especie: 42,
    }),
    apiKey: 'test-key',
  });
  expect(JSON.stringify(invoke.mock.calls[0])).not.toContain('Pessoa CNIS Direto');
  expect(JSON.stringify(invoke.mock.calls[0])).not.toContain('11122233344');
});

it('cobre todas as 5 abas TRF3 e converte RMI de atrasados sem multiplicar 1600.00 por 100', async () => {
  invoke.mockImplementation(async (cmd: string) => {
    if (cmd === 'calcular_fabrica_tempo') {
      return {
        result: {
          totais: { simples: 7300, convertido: 7300, carencia: 240, idade: 70 },
          beneficios: { cumpridos: [{ descricao: 'Aposentadoria por Idade', fundamento: 'EC 103' }], naoCumpridos: [] },
        },
      };
    }
    if (cmd === 'calcular_fabrica_atrasados') {
      return { result: { totalGeral: '19200.00', totalPrincipal: '16000.00', totalCorrecao: '2000.00', totalJuros: '1200.00' } };
    }
    if (cmd === 'ler_documento_fabrica') {
      return { result: { tipo: 'cnis', vinculos: 1 }, messages: [] };
    }
    if (cmd === 'consultar_fabrica_indices') {
      return [{ valor: 'EC_103_ART_18', rotulo: 'EC 103, art. 18' }];
    }
    return null;
  });

  const workspace: Workspace = blankWorkspace();
  workspace.settings.fabrica_api_key = 'test-key';
  render(<FabricaCheck draft={case41()} workspace={workspace} onChange={() => {}} />);

  // Aba 2: Tempo de Contribuição
  fireEvent.click(screen.getByRole('tab', { name: 'Tempo de Contribuição' }));
  await act(async () => {
    fireEvent.click(screen.getByRole('button', { name: 'Calcular Tempo na Fábrica TRF3' }));
  });
  await waitFor(() => expect(screen.getByText('240 meses')).toBeInTheDocument());
  expect(invoke).toHaveBeenCalledWith('calcular_fabrica_tempo', expect.objectContaining({ apiKey: 'test-key' }));

  // Aba 3: Atrasados / Liquidação (RMI auto-preenchida "1600.00" deve virar 1600, nunca 160000)
  fireEvent.click(screen.getByRole('tab', { name: 'Atrasados / Liquidação' }));
  const rmiInput = screen.getByLabelText('RMI inicial (R$)');
  expect(rmiInput).toHaveValue('1600.00');
  await act(async () => {
    fireEvent.click(screen.getByRole('button', { name: 'Calcular Atrasados / Liquidação' }));
  });
  await waitFor(() => expect(screen.getByText('R$ 19.200,00')).toBeInTheDocument());
  const atrasadosCall = invoke.mock.calls.find(c => c[0] === 'calcular_fabrica_atrasados');
  expect(atrasadosCall?.[1].body.beneficiosDevidos[0].rmi).toBe(1600);

  // Formato BR e limpeza manual sem re-injeção
  fireEvent.change(rmiInput, { target: { value: '' } });
  expect(rmiInput).toHaveValue('');
  fireEvent.change(rmiInput, { target: { value: '1.750,50' } });
  await act(async () => {
    fireEvent.click(screen.getByRole('button', { name: 'Calcular Atrasados / Liquidação' }));
  });
  await waitFor(() => {
    const calls = invoke.mock.calls.filter(c => c[0] === 'calcular_fabrica_atrasados');
    expect(calls).toHaveLength(2);
    expect(calls[1][1].body.beneficiosDevidos[0].rmi).toBe(1750.5);
  });

  // Aba 4: Leitor Documental
  fireEvent.click(screen.getByRole('tab', { name: 'Leitor Documental' }));
  await act(async () => {
    fireEvent.click(screen.getByRole('button', { name: /Selecionar PDF e Ler/ }));
  });
  await waitFor(() => expect(screen.getByText(/Documento processado/)).toBeInTheDocument());
  expect(invoke).toHaveBeenCalledWith('ler_documento_fabrica', { tipo: 'cnis', apiKey: 'test-key' });

  // Aba 5: Índices e Tabelas
  fireEvent.click(screen.getByRole('tab', { name: 'Índices e Tabelas' }));
  await act(async () => {
    fireEvent.click(screen.getByRole('button', { name: 'Consultar Fundamentos' }));
  });
  await waitFor(() => expect(screen.getByText('EC_103_ART_18')).toBeInTheDocument());
  expect(invoke).toHaveBeenCalledWith('consultar_fabrica_indices', {
    endpoint: 'fundamentos',
    nome: null,
    query: null,
    apiKey: 'test-key',
  });
});
