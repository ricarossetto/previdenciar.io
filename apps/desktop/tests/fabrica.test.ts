import {afterEach,describe,expect,it,vi} from 'vitest';
import {blankCase,blankWorkspace} from '../src/model';
import {buildFabricaInput,buildFabricaInputDirect,cents,difference,FABRICA_ESPECIES,localRmi} from '../src/fabrica';
import {LINK_KINDS} from '../src/assistantReview';
import {getWebFabricaKey,webApi} from '../src/web-api';

const invokeMock = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/api/core', () => ({invoke: invokeMock}));

function case41() {
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
  c.manual_links = [
    {id: 1, employer: 'Empresa Fictícia', start_date: '1994-07-01', end_date: '2019-06-30', regime: 'RGPS', indicators: '', confirmed: true},
  ];
  c.manual_salaries = [
    {id: 1, competence: '1994-07', amount: '2000.00', indicators: '', confirmed: true},
    {id: 2, competence: '2019-06', amount: '3000.00', indicators: '', confirmed: true},
  ];
  return c;
}

function withKind(kind: string) {
  const c = case41();
  const policy = JSON.parse(c.options.rule_policy);
  policy.manual_link_kind = kind;
  c.options.rule_policy = JSON.stringify(policy);
  return c;
}

function withSource(affiliation: string | null, prpps = false, indicators: string[] = []) {
  const c = case41();
  c.manual_links[0] = {
    ...c.manual_links[0],
    regime: 'RGPS',
    source_review: {sequence: 1, source_sha256: 'synthetic', excluded: false, history: []},
  };
  c.cnis = {
    metadata: {page_count: 1, source_sha256: 'synthetic', extractor: 'synthetic'},
    identity: {...c.identity},
    salary_entries: [],
    salary_months: {},
    links: [{
      sequence: 1,
      raw_employer: 'Origem 1',
      affiliation,
      start_date: '1994-07-01',
      end_date: '2019-06-30',
      prpps,
      indicators,
      reported_last_salary_month: null,
      inferred_last_salary_end: null,
      source: {page: 1},
    }],
    benefits: [],
    warnings: [],
    findings: [],
  };
  return c;
}

function withRuralSources(affiliations: string[]) {
  const c = withKind('rural');
  const years = affiliations.length === 1
    ? [['1994-07-01','2019-06-30']]
    : [['1994-07-01','1999-12-31'],['2000-01-01','2019-06-30']];
  c.cnis = {
    metadata: {page_count: 1, source_sha256: 'synthetic', extractor: 'synthetic'},
    identity: {...c.identity},
    salary_entries: [],
    salary_months: {},
    links: affiliations.map((affiliation, index) => ({
      sequence: index + 1,
      raw_employer: 'Origem ' + (index + 1),
      affiliation,
      start_date: years[index][0],
      end_date: years[index][1],
      prpps: false,
      indicators: [],
      reported_last_salary_month: null,
      inferred_last_salary_end: null,
      source: {page: 1},
    })),
    benefits: [],
    warnings: [],
    findings: [],
  };
  c.manual_links = affiliations.map((_, index) => ({
    id: index + 1,
    employer: 'Origem ' + (index + 1),
    start_date: years[index][0],
    end_date: years[index][1],
    regime: 'RGPS',
    indicators: '',
    confirmed: true,
    source_review: {sequence: index + 1, source_sha256: 'synthetic', excluded: false, history: []},
  }));
  c.manual_salaries = [
    {id: 1, competence: '1994-07', amount: '2000.00', indicators: '', confirmed: true},
    {id: 2, competence: '2019-06', amount: '3000.00', indicators: '', confirmed: true},
  ];
  return c;
}

describe('conferência na Fábrica de Cálculos', () => {
  it('monta o payload só com datas, sexo, espécie e salários revisados', () => {
    const built = buildFabricaInput(case41());
    expect(built.issues).toEqual([]);
    expect(built.input).not.toBeNull();
    const input = built.input!;
    expect(input).toMatchObject({
      dataApuracao: '2026-01-10',
      dataNascimento: '1956-01-10',
      sexo: 'M',
      especie: 41,
      isencaoCarencia: false,
      tipo: 'COMUM',
    });
    expect(input.periodos).toHaveLength(1);
    expect(input.periodos[0].salarios).toEqual([
      {competencia: '1994-07', valor: '2000.00'},
      {competencia: '2019-06', valor: '3000.00'},
    ]);
    const serialized = JSON.stringify(input);
    expect(serialized).not.toContain('Pessoa Fictícia');
    expect(serialized).not.toContain('00000000000');
    expect(serialized).not.toContain('Empresa Fictícia');
  });

  it('recusa vínculo ou salário não confirmado e remuneração fora do vínculo', () => {
    const unconfirmedLink = case41();
    unconfirmedLink.manual_links[0].confirmed = false;
    expect(buildFabricaInput(unconfirmedLink).input).toBeNull();

    const unconfirmedSalary = case41();
    unconfirmedSalary.manual_salaries[0].confirmed = false;
    expect(buildFabricaInput(unconfirmedSalary).input).toBeNull();

    const outside = case41();
    outside.manual_salaries.push({id: 3, competence: '2020-01', amount: '3000.00', indicators: '', confirmed: true});
    const built = buildFabricaInput(outside);
    expect(built.input).toBeNull();
    expect(built.issues.join(' ')).toContain('fora dos vínculos');
  });

  it('não converte categoria legada ou não suportada em COMUM', () => {
    for (const kind of ['normal','individual','especial','servidor']) {
      const built = buildFabricaInput(withKind(kind));
      expect(built.input).toBeNull();
      expect(built.issues.join(' ')).toContain('não suportada');
    }
  });

  it('aceita os cinco tipos locais como COMUM', () => {
    for (const kind of LINK_KINDS) {
      const built = buildFabricaInput(withKind(kind));
      expect(built.input?.tipo).toBe('COMUM');
    }
  });

  it('bloqueia indicador pendente ou desconhecido mesmo com confirmed=true', () => {
    for (const indicator of ['PREC-MENOR-MIN','PEXT','PREM-EMPR','INDICADOR-X']) {
      const link = case41();
      link.manual_links[0].indicators = indicator;
      const built = buildFabricaInput(link);
      expect(built.input).toBeNull();
      expect(built.issues.join(' ')).toContain(indicator === 'INDICADOR-X' ? 'não catalogado' : 'pendente');
    }
    const salary = case41();
    salary.manual_salaries[0].indicators = 'PREC-MENOR-MIN';
    const built = buildFabricaInput(salary);
    expect(built.input).toBeNull();
    expect(built.issues.join(' ')).toContain('pendente');
  });

  it('bloqueia AVRC-DEF no vínculo e no salário', () => {
    const link = case41();
    link.manual_links[0].indicators = 'AVRC-DEF';
    const linkBuilt = buildFabricaInput(link);
    expect(linkBuilt.input).toBeNull();
    expect(linkBuilt.issues.join(' ')).toContain('pendente');

    const salary = case41();
    salary.manual_salaries[0].indicators = 'AVRC-DEF';
    const salaryBuilt = buildFabricaInput(salary);
    expect(salaryBuilt.input).toBeNull();
    expect(salaryBuilt.issues.join(' ')).toContain('pendente');
  });

  it('bloqueia regime não RGPS ou ausente sem alterar o regime', () => {
    for (const regime of ['RPPS','PRPPS','A confirmar','']) {
      const link = case41();
      link.manual_links[0].regime = regime;
      const built = buildFabricaInput(link);
      expect(built.input).toBeNull();
      expect(built.issues.join(' ')).toContain('não é RGPS');
      expect(link.manual_links[0].regime).toBe(regime);
    }
  });

  it('bloqueia origem PRPPS/CTC e afiliação RPPS mesmo com regime RGPS', () => {
    const prpps = buildFabricaInput(withSource('Empregado', true, ['PRPPS']));
    expect(prpps.input).toBeNull();
    expect(prpps.issues.join(' ')).toContain('PRPPS/CTC');

    const servidor = buildFabricaInput(withSource('Servidor'));
    expect(servidor.input).toBeNull();
    expect(servidor.issues.join(' ')).toContain('RPPS');

    const missing = withSource(null);
    missing.cnis!.links = [];
    const built = buildFabricaInput(missing);
    expect(built.input).toBeNull();
    expect(built.issues.join(' ')).toContain('origem ausente');
  });

  it('preserva RURICOLA quando a declaração rural é explícita e os vínculos estão confirmados', () => {
    const built = buildFabricaInput(withKind('rural'));
    expect(built.issues).toEqual([]);
    expect(built.input?.tipo).toBe('RURICOLA');
  });

  it('recusa rural sem confirmação dos vínculos', () => {
    const rural = withKind('rural');
    rural.manual_links[0].confirmed = false;
    const built = buildFabricaInput(rural);
    expect(built.input).toBeNull();
    expect(built.issues.join(' ')).toContain('Confirme todos os vínculos');
  });

  it('aceita RURICOLA com origem rural comprovada e recusa mistura ambígua', () => {
    const rural = buildFabricaInput(withRuralSources(['Empregado rural']));
    expect(rural.issues).toEqual([]);
    expect(rural.input?.tipo).toBe('RURICOLA');

    const mixed = buildFabricaInput(withRuralSources(['Empregado rural','Empregado']));
    expect(mixed.input).toBeNull();
    expect(mixed.issues.join(' ')).toContain('rural');

    const missing = buildFabricaInput(withRuralSources(['Empregado rural','']));
    expect(missing.input).toBeNull();
    expect(missing.issues.join(' ')).toContain('rural');
  });

  it('recusa PcD e períodos sobrepostos na conferência comum', () => {
    const pcd = case41();
    pcd.disability = 'Grave';
    expect(buildFabricaInput(pcd).input).toBeNull();
    expect(buildFabricaInput(pcd).issues.join(' ')).toContain('LC 142');

    const overlap = case41();
    overlap.manual_links.push({
      id: 2, employer: 'Outra fictícia', start_date: '2019-01-01', end_date: '2019-12-31', regime: 'RGPS', indicators: '', confirmed: true,
    });
    expect(buildFabricaInput(overlap).input).toBeNull();
    expect(buildFabricaInput(overlap).issues.join(' ')).toContain('sobrepostos');
  });

  it('extrai a RMI local corrigida e compara em centavos', () => {
    const result = {
      status: 'experimental',
      data: {
        native_result: {
          resumo: {rmi: '1600.00'},
          transicoes: {rmi_escolhida: '1700.00'},
          parametros: {correcao_ativa: true},
        },
      },
    };
    const local = localRmi(result);
    expect(local.rmi).toBe('1700.00');
    expect(local.corrected).toBe(true);
    expect(cents('1700.00')).toBe(170000n);
    const delta = difference('1600.00', '1700.00');
    expect(delta?.cents).toBe(10000n);
    expect(delta?.percent).toBe('6.3');
  });
});

function cnisDirectCase() {
  const c = blankCase();
  c.identity = {
    full_name: 'Maria Sintética CNIS',
    cpf: '11122233344',
    nit: '12345678901',
    birth_date: '1965-05-20',
    mother_name: 'Mãe Sintética',
  };
  c.sex = 'Feminino';
  c.disability = 'A apurar';
  c.dib = '2026-03-01';
  c.especie = '42';
  c.result = null;
  c.manual_links = [];
  c.manual_salaries = [];
  c.cnis = {
    metadata: { page_count: 2, source_sha256: 'synthetic-sha', extractor: 'synthetic' },
    identity: { ...c.identity },
    links: [
      {
        sequence: 1,
        raw_employer: 'Empregador Sigiloso A',
        affiliation: 'Empregado',
        start_date: '2000-01-01',
        end_date: '2020-12-31',
        prpps: false,
        indicators: [],
        reported_last_salary_month: '2020-12',
        inferred_last_salary_end: null,
        source: { page: 1 },
      },
      {
        sequence: 2,
        raw_employer: 'Empregador Sigiloso B (Concomitante)',
        affiliation: 'Contribuinte Individual',
        start_date: '2015-01-01',
        end_date: null,
        prpps: false,
        indicators: ['PREC-MENOR-MIN'],
        reported_last_salary_month: '2025-12',
        inferred_last_salary_end: { date_candidate: '2025-12-31', requires_confirmation: true },
        source: { page: 1 },
      },
      {
        sequence: 3,
        raw_employer: 'Órgão PRPPS Excluído',
        affiliation: 'Servidor Público',
        start_date: '2010-01-01',
        end_date: '2014-12-31',
        prpps: true,
        indicators: ['PRPPS'],
        reported_last_salary_month: null,
        inferred_last_salary_end: null,
        source: { page: 2 },
      },
    ],
    salary_entries: [
      { competence: '2010-05', amount_centavos: 250000, indicators: [], link_sequence: 1, source: { page: 1, line: 10, index: 1 }, excluded_from_validated_domain: false, amount_kind: 'remuneration' },
      { competence: '2016-03', amount_centavos: 310000, indicators: [], link_sequence: 1, source: { page: 1, line: 11, index: 2 }, excluded_from_validated_domain: false, amount_kind: 'remuneration' },
      { competence: '2016-03', amount_centavos: 120000, indicators: [], link_sequence: 2, source: { page: 2, line: 4, index: 3 }, excluded_from_validated_domain: false, amount_kind: 'remuneration' },
      { competence: '2025-11', amount_centavos: 450000, indicators: [], link_sequence: 2, source: { page: 2, line: 5, index: 4 }, excluded_from_validated_domain: false, amount_kind: 'remuneration' },
      { competence: '2012-01', amount_centavos: 500000, indicators: ['PRPPS'], link_sequence: 3, source: { page: 2, line: 8, index: 5 }, excluded_from_validated_domain: true, amount_kind: 'remuneration' },
    ],
    salary_months: {},
    benefits: [],
    warnings: [],
    findings: [],
  };
  return c;
}

describe('buildFabricaInputDirect (entrada direta TRF3 sem cálculo local prévio)', () => {
  it('monta payload diretamente do CNIS com períodos concomitantes e sem vazar identidade', () => {
    const c = cnisDirectCase();
    expect(buildFabricaInput(c).input).toBeNull();
    const built = buildFabricaInputDirect(c);
    expect(built.issues).toEqual([]);
    expect(built.sourceMode).toBe('cnis_direct');
    expect(built.input).not.toBeNull();
    const input = built.input!;
    expect(input.dataApuracao).toBe('2026-03-01');
    expect(input.dataNascimento).toBe('1965-05-20');
    expect(input.sexo).toBe('F');
    expect(input.especie).toBe(42);
    expect(input.tipo).toBe('COMUM');
    expect(input.periodos).toHaveLength(2);
    expect(input.periodos[0].dataInicial).toBe('2000-01-01');
    expect(input.periodos[0].dataFinal).toBe('2020-12-31');
    expect(input.periodos[1].dataInicial).toBe('2015-01-01');
    expect(input.periodos[1].dataFinal).toBe('2025-12-31');
    expect(input.periodos[1].salarios).toEqual([
      { competencia: '2016-03', valor: '1200.00' },
      { competencia: '2025-11', valor: '4500.00' },
    ]);
    const serialized = JSON.stringify(input);
    expect(serialized).not.toContain('Maria Sintética CNIS');
    expect(serialized).not.toContain('11122233344');
    expect(serialized).not.toContain('12345678901');
    expect(serialized).not.toContain('Empregador Sigiloso');
    expect(serialized).not.toContain('Órgão PRPPS');
  });

  it('suporta todas as 17 espécies TRF3 (incluindo 46, 21 com fatoGerador e 87)', () => {
    const c = cnisDirectCase();
    for (const item of FABRICA_ESPECIES) {
      const built = buildFabricaInputDirect(c, { especie: item.code });
      expect(built.issues).toEqual([]);
      expect(built.input?.especie).toBe(item.code);
    }

    const esp46 = buildFabricaInputDirect(c, { especie: 46 });
    expect(esp46.input?.especie).toBe(46);
    expect(esp46.input?.fatoGerador).toBeUndefined();

    const esp21 = buildFabricaInputDirect(c, { especie: 21 });
    expect(esp21.input?.especie).toBe(21);
    expect(esp21.input?.fatoGerador).toBe('2026-03-01');

    const esp87 = buildFabricaInputDirect(c, { especie: 87 });
    expect(esp87.input?.especie).toBe(87);
    expect(esp87.input?.fatoGerador).toBeUndefined();

    const invalid = buildFabricaInputDirect(c, { especie: 999 });
    expect(invalid.input).toBeNull();
    expect(invalid.issues.join(' ')).toContain('999');
  });

  it('suporta PCD por deficiência declarada ou override e opções demonstrativo/permalink', () => {
    const c = cnisDirectCase();
    c.disability = 'Moderada';
    const autoPcd = buildFabricaInputDirect(c);
    expect(autoPcd.issues).toEqual([]);
    expect(autoPcd.input?.tipo).toBe('PCD');

    const withOptions = buildFabricaInputDirect(cnisDirectCase(), {
      tipo: 'PCD',
      demonstrativo: true,
      permalink: true,
      fundamento: 'EC_103_ART_22',
      der: '01/02/2026',
    });
    expect(withOptions.issues).toEqual([]);
    expect(withOptions.input?.tipo).toBe('PCD');
    expect(withOptions.input?.demonstrativo).toBe(true);
    expect(withOptions.input?.permalink).toBe(true);
    expect(withOptions.input?.fundamento).toBe('EC_103_ART_22');
    expect(withOptions.input?.der).toBe('2026-02-01');
  });

  it('aceita períodos manuais sobrepostos sem bloquear na entrada direta', () => {
    const overlap = case41();
    overlap.result = null;
    overlap.manual_links.push({
      id: 2,
      employer: 'Empresa Concomitante Fictícia',
      start_date: '2018-01-01',
      end_date: '2019-12-31',
      regime: 'RGPS',
      indicators: 'PREC-MENOR-MIN',
      confirmed: false,
    });
    expect(buildFabricaInput(overlap).input).toBeNull();
    const direct = buildFabricaInputDirect(overlap);
    expect(direct.issues).toEqual([]);
    expect(direct.sourceMode).toBe('manual_direct');
    expect(direct.input?.periodos).toHaveLength(2);
    const serialized = JSON.stringify(direct.input);
    expect(serialized).not.toContain('Pessoa Fictícia');
    expect(serialized).not.toContain('00000000000');
    expect(serialized).not.toContain('Empresa Fictícia');
    expect(serialized).not.toContain('Empresa Concomitante Fictícia');
  });

  it('manual_direct filtra vínculos RPPS/PRPPS e soma remunerações concomitantes sem vazar empregador', () => {
    const c = case41();
    c.result = null;
    c.manual_links = [
      {
        id: 1,
        employer: 'Empresa Secreta RGPS',
        start_date: '2015-01-01',
        end_date: '2020-12-31',
        regime: 'RGPS',
        indicators: '',
        confirmed: true,
        source_review: { sequence: 1, source_sha256: null, excluded: false, history: [] },
      },
      {
        id: 2,
        employer: 'Prefeitura RPPS Secreta',
        start_date: '2015-01-01',
        end_date: '2020-12-31',
        regime: 'RPPS',
        indicators: 'PRPPS',
        confirmed: true,
        source_review: { sequence: 2, source_sha256: null, excluded: false, history: [] },
      },
    ];
    c.contribution_review = {
      version: 1,
      source_sha256: null,
      applied: true,
      history: [],
      records: [
        {
          key: 'k1',
          source_indices: [1],
          competence: '2018-05',
          amount: '1500.25',
          currency: 'BRL',
          link_sequence: 1,
          indicators: '',
          confirmed: true,
          excluded: false,
          evidence: 'cnis',
          amount_kind: 'remuneration',
        },
        {
          key: 'k2',
          source_indices: [2],
          competence: '2018-05',
          amount: '900.75',
          currency: 'BRL',
          link_sequence: 1,
          indicators: '',
          confirmed: true,
          excluded: false,
          evidence: 'cnis',
          amount_kind: 'remuneration',
        },
        {
          key: 'k3-darf',
          source_indices: [3],
          competence: '2018-05',
          amount: '300.00',
          currency: 'BRL',
          link_sequence: 1,
          indicators: '',
          confirmed: true,
          excluded: false,
          evidence: 'cnis',
          amount_kind: 'contribution_column_unresolved',
        },
        {
          key: 'k4-rpps',
          source_indices: [4],
          competence: '2018-05',
          amount: '5000.00',
          currency: 'BRL',
          link_sequence: 2,
          indicators: 'PRPPS',
          confirmed: true,
          excluded: false,
          evidence: 'cnis',
          amount_kind: 'remuneration',
        },
      ],
    };
    const direct = buildFabricaInputDirect(c);
    expect(direct.issues).toEqual([]);
    expect(direct.input?.periodos).toHaveLength(1);
    expect(direct.input?.periodos[0].salarios).toEqual([
      { competencia: '2018-05', valor: '2401.00' },
    ]);
    const serialized = JSON.stringify(direct.input);
    expect(serialized).not.toContain('Empresa Secreta RGPS');
    expect(serialized).not.toContain('Prefeitura RPPS Secreta');

    const fallbackManual = case41();
    fallbackManual.result = null;
    fallbackManual.manual_links = [c.manual_links[0]];
    fallbackManual.manual_salaries = [
      { id: 10, competence: '2019-03', amount: '1200.10', indicators: '', confirmed: false },
      { id: 11, competence: '2019-03', amount: '800.20', indicators: '', confirmed: false },
      { id: 12, competence: '2019-04', amount: '-100,00', indicators: '', confirmed: false },
      { id: 13, competence: '2019-05', amount: '-100,50', indicators: '', confirmed: false },
    ];
    const directManual = buildFabricaInputDirect(fallbackManual);
    expect(directManual.issues).toEqual([]);
    expect(directManual.input?.periodos[0].salarios).toEqual([
      { competencia: '2019-03', valor: '2000.30' },
    ]);

    const reviewNeg = case41();
    reviewNeg.result = null;
    reviewNeg.manual_links = [c.manual_links[0]];
    reviewNeg.contribution_review = {
      version: 1,
      source_sha256: null,
      applied: true,
      history: [],
      records: [
        {
          key: 'neg-1',
          source_indices: [1],
          competence: '2019-04',
          amount: '-100,00',
          currency: 'BRL',
          link_sequence: 1,
          indicators: '',
          confirmed: true,
          excluded: false,
          evidence: 'cnis',
          amount_kind: 'remuneration',
        },
        {
          key: 'neg-2',
          source_indices: [2],
          competence: '2019-05',
          amount: '-100,50',
          currency: 'BRL',
          link_sequence: 1,
          indicators: '',
          confirmed: true,
          excluded: false,
          evidence: 'cnis',
          amount_kind: 'remuneration',
        },
        {
          key: 'pos-1',
          source_indices: [3],
          competence: '2019-06',
          amount: '2500.00',
          currency: 'BRL',
          link_sequence: 1,
          indicators: '',
          confirmed: true,
          excluded: false,
          evidence: 'cnis',
          amount_kind: 'remuneration',
        },
      ],
    };
    const directReviewNeg = buildFabricaInputDirect(reviewNeg);
    expect(directReviewNeg.issues).toEqual([]);
    expect(directReviewNeg.input?.periodos[0].salarios).toEqual([
      { competencia: '2019-06', valor: '2500.00' },
    ]);
  });
});

describe('paridade Desktop (tauriApi) e Web (webApi) para Fábrica TRF3 e IA', () => {
  afterEach(() => {
    invokeMock.mockReset();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
    sessionStorage.clear();
    localStorage.clear();
  });

  it('tauriApi.conferirFabricaRmi mantém 2 chaves quando options é omitido e espalha options quando informado', async () => {
    const {api} = await import('../src/api');
    const input = buildFabricaInput(case41()).input!;
    invokeMock.mockResolvedValue({provider: {maiorRmi: '1700.00', beneficios: []}});

    await api.conferirFabricaRmi(input, 'chave-trf3-123456');
    expect(invokeMock).toHaveBeenLastCalledWith('conferir_fabrica_rmi', {input, apiKey: 'chave-trf3-123456'});
    expect(Object.keys(invokeMock.mock.calls[0][1])).toEqual(['input', 'apiKey']);

    await api.conferirFabricaRmi(input, 'chave-trf3-123456', {demonstrativo: true, permalink: true, fundamento: false});
    expect(invokeMock).toHaveBeenLastCalledWith('conferir_fabrica_rmi', {
      input,
      apiKey: 'chave-trf3-123456',
      demonstrativo: true,
      permalink: true,
      fundamento: false,
    });
  });

  it('tauriApi expõe todos os comandos da Fábrica TRF3', async () => {
    const {api} = await import('../src/api');
    const input = buildFabricaInput(case41()).input!;
    invokeMock.mockResolvedValue({ok: true});

    await api.fabricaStatus();
    expect(invokeMock).toHaveBeenLastCalledWith('fabrica_status');

    await api.fabricaSaveKey('chave-trf3-123456');
    expect(invokeMock).toHaveBeenLastCalledWith('fabrica_save_key', {apiKey: 'chave-trf3-123456'});

    await api.fabricaClearKey();
    expect(invokeMock).toHaveBeenLastCalledWith('fabrica_clear_key');

    await api.calcularFabricaTempo(input, 'chave-trf3-123456', {porGrupos: true, demonstrativo: true});
    expect(invokeMock).toHaveBeenLastCalledWith('calcular_fabrica_tempo', {
      input,
      apiKey: 'chave-trf3-123456',
      porGrupos: true,
      demonstrativo: true,
      permalink: null,
    });

    await api.calcularFabricaAtrasados({valor: '1000.00'}, undefined, {permalink: true});
    expect(invokeMock).toHaveBeenLastCalledWith('calcular_fabrica_atrasados', {
      body: {valor: '1000.00'},
      apiKey: null,
      demonstrativo: false,
      permalink: true,
    });

    await api.lerDocumentoFabrica('cnis', 'chave-trf3-123456');
    expect(invokeMock).toHaveBeenLastCalledWith('ler_documento_fabrica', {tipo: 'cnis', apiKey: 'chave-trf3-123456'});

    await api.consultarFabricaIndices('indice', 'tabela', {competenciaInicial: '2020-01'}, 'chave-trf3-123456');
    expect(invokeMock).toHaveBeenLastCalledWith('consultar_fabrica_indices', {
      endpoint: 'indice',
      nome: 'tabela',
      query: {competenciaInicial: '2020-01'},
      apiKey: 'chave-trf3-123456',
    });
  });

  it('webApi guarda a chave da Fábrica somente em sessionStorage e nunca a envia no workspace', async () => {
    await webApi.fabricaSaveKey('  chave-web-trf3-987654  ');
    expect(getWebFabricaKey()).toBe('chave-web-trf3-987654');
    expect(sessionStorage.getItem('previdenciar.web.fabricaKey')).toBe('chave-web-trf3-987654');
    expect(localStorage.getItem('previdenciar.web.fabricaKey')).toBeNull();

    await expect(webApi.fabricaSaveKey('curta')).rejects.toThrow(/chave válida/i);
    await expect(webApi.fabricaSaveKey('chave com espaco 1234')).rejects.toThrow(/chave válida/i);

    const fetchMock = vi.fn(async (_url: string, init?: RequestInit) => {
      const parsed = JSON.parse(String(init?.body ?? '{}')) as {workspace: {settings: {fabrica_api_key: string}}};
      expect(parsed.workspace.settings.fabrica_api_key).toBe('');
      return {ok: true, status: 200, text: async () => JSON.stringify(parsed.workspace)} as unknown as Response;
    });
    vi.stubGlobal('fetch', fetchMock);
    const ws = blankWorkspace();
    ws.settings.fabrica_api_key = 'nao-deve-ir-para-postgres';
    await webApi.save(ws);
    expect(fetchMock).toHaveBeenCalledTimes(1);

    await webApi.fabricaClearKey();
    expect(getWebFabricaKey()).toBe('');
  });

  it('webApi envia x-fabrica-key nos endpoints /api/fabrica/* e conecta base de conhecimento e exportação', async () => {
    await webApi.fabricaSaveKey('chave-web-trf3-987654');
    const calls: Array<{url: string; method: string; headers: Headers; body: unknown}> = [];
    const fetchMock = vi.fn(async (url: string, init?: RequestInit) => {
      const headers = new Headers(init?.headers);
      calls.push({url, method: init?.method ?? 'GET', headers, body: init?.body});
      if (url.startsWith('/api/fabrica/status')) {
        return {
          ok: true,
          status: 200,
          text: async () => JSON.stringify({configured: true, serverKeyAvailable: false, sessionKeyConfigured: true, upstream: 'https://fabrica-de-calculos.app.trf3.jus.br'}),
        } as unknown as Response;
      }
      if (url.startsWith('/api/fabrica/fundamentos')) {
        return {ok: true, status: 200, text: async () => JSON.stringify([{chave: 'EC_103_ART_18'}])} as unknown as Response;
      }
      if (url.startsWith('/api/reports/export')) {
        return {
          ok: true,
          status: 200,
          text: async () => JSON.stringify({content: 'col1,col2\n1,2\n', mime: 'text/csv; charset=utf-8', extension: 'csv'}),
        } as unknown as Response;
      }
      return {ok: true, status: 200, text: async () => JSON.stringify({ok: true, skills: 1, legislation: 2, natjus: 3, natjusText: 1, jurisprudence: 4, ready: true, message: 'ok'})} as unknown as Response;
    });
    vi.stubGlobal('fetch', fetchMock);
    vi.stubGlobal('URL', {createObjectURL: vi.fn(() => 'blob:mock'), revokeObjectURL: vi.fn()});
    vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(() => {});

    const status = await webApi.fabricaStatus();
    expect(status.configured).toBe(true);
    expect(status.sessionKeyConfigured).toBe(true);
    expect(calls.at(-1)?.headers.get('x-fabrica-key')).toBe('chave-web-trf3-987654');

    const input = buildFabricaInput(case41()).input!;
    await webApi.conferirFabricaRmi(input, '', {demonstrativo: true, permalink: true});
    expect(calls.at(-1)?.url).toBe('/api/fabrica/rmi?demonstrativo=true&permalink=true');
    expect(calls.at(-1)?.headers.get('x-fabrica-key')).toBe('chave-web-trf3-987654');

    await webApi.calcularFabricaTempo(input, undefined, {porGrupos: true, demonstrativo: true});
    expect(calls.at(-1)?.url).toBe('/api/fabrica/tempo/por-grupos?demonstrativo=true');

    await webApi.calcularFabricaAtrasados({valor: '1000.00'}, undefined, {permalink: true});
    expect(calls.at(-1)?.url).toBe('/api/fabrica/atrasados?permalink=true');

    const fundamentos = await webApi.consultarFabricaIndices('fundamentos');
    expect(fundamentos).toEqual([{chave: 'EC_103_ART_18'}]);

    const pdfFile = new File(['%PDF-1.4'], 'extrato.pdf', {type: 'application/pdf'});
    await webApi.lerDocumentoFabrica('cnis', undefined, pdfFile);
    expect(calls.at(-1)?.url).toBe('/api/fabrica/documentos/cnis');
    expect(calls.at(-1)?.body).toBeInstanceOf(FormData);
    expect(calls.at(-1)?.headers.get('x-fabrica-key')).toBe('chave-web-trf3-987654');

    const aiStatus = await webApi.aiDataStatus();
    expect(aiStatus.ready).toBe(true);
    expect(calls.at(-1)?.url).toBe('/api/ai/data-status');

    await webApi.aiKnowledgeSearch('legislation', 'Lei 8.213', 5);
    expect(calls.at(-1)?.url).toBe('/api/ai/knowledge/search');

    await webApi.aiKnowledgeRead('legislation', 'lei-8213');
    expect(calls.at(-1)?.url).toBe('/api/ai/knowledge/read');

    const exported = await webApi.exportReport(case41(), blankWorkspace(), 'csv', false);
    expect(exported).toBe('previdenciar-relatorio.csv');
    expect(calls.at(-1)?.url).toBe('/api/reports/export');
  });
});
