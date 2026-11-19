const TODAY = '2026-10-26';

const NAV = [
  { id: 'home', label: 'Início', icon: 'home' },
  { id: 'clients', label: 'Clientes', icon: 'users' },
  { id: 'cases', label: 'Cálculos', icon: 'calc' },
  { id: 'rgps', label: 'Cálculos do RGPS', icon: 'scale', group: 'Cálculo previdenciário' },
  { id: 'engine', label: 'Motor local (WASM)', icon: 'chip' },
  { id: 'fabrica', label: 'Fábrica de Cálculos', icon: 'scale' },
  { id: 'assistant', label: 'Assistente de cálculos', icon: 'compass' },
  { id: 'editor', label: 'Editor de dados', icon: 'file' },
  { id: 'tables', label: 'Tabelas e índices', icon: 'table' },
  { id: 'modules', label: 'Módulos de apoio', icon: 'tool' },
  { id: 'documents', label: 'Documentos', icon: 'book', group: 'Escritório' },
  { id: 'reports', label: 'Relatórios', icon: 'report' },
  { id: 'agenda', label: 'Agenda', icon: 'calendar' },
  { id: 'settings', label: 'Configurações', icon: 'settings' },
  { id: 'coverage', label: 'REA e validação', icon: 'shield' }
];

let CLIENTS = [
  { id: 1, name: 'Helena Fagundes', cpf: '⬢⬢⬢.412.880-⬢⬢', birth: '1968-02-14', sex: 'Feminino', email: 'helena@exemplo.com', phone: '(51) 99999-0142', city: 'Porto Alegre', notes: 'Cliente desde 2024. CNIS de 23 páginas conferido em 12/10/2026.' },
  { id: 2, name: 'Joaquim Ferreira Lima', cpf: '⬢⬢⬢.208.117-⬢⬢', birth: '1963-06-02', sex: 'Masculino', email: 'joaquim@exemplo.com', phone: '(51) 98888-2210', city: 'Canoas', notes: 'Aguardando CTC do período de 1994 a 2005.' },
  { id: 3, name: 'Marta Nogueira', cpf: '⬢⬢⬢.771.309-⬢⬢', birth: '1971-11-23', sex: 'Feminino', email: 'marta@exemplo.com', phone: '(51) 97777-4081', city: 'Porto Alegre', notes: 'Cenário PcD de grau moderado pela LC 142/2013.' },
  { id: 4, name: 'Antônio Sales', cpf: '⬢⬢⬢.055.624-⬢⬢', birth: '1966-01-09', sex: 'Masculino', email: 'antonio@exemplo.com', phone: '(51) 96666-3122', city: 'Viamão', notes: 'CNIS recebido sem a página de contribuições.' },
  { id: 5, name: 'Rita Albuquerque', cpf: '⬢⬢⬢.930.452-⬢⬢', birth: '1977-04-17', sex: 'Feminino', email: 'rita@exemplo.com', phone: '(51) 95555-8890', city: 'Gravataí', notes: 'Revisão de RMI concluída; memória revisada em 20/10.' },
  { id: 6, name: 'Sebastião Moura', cpf: '⬢⬢⬢.144.087-⬢⬢', birth: '1960-08-30', sex: 'Masculino', email: 'sebastiao@exemplo.com', phone: '(51) 94444-1102', city: 'Alvorada', notes: 'PPP do período especial pendente.' },
  { id: 7, name: 'Clara Bittencourt', cpf: '⬢⬢⬢.682.551-⬢⬢', birth: '1982-12-05', sex: 'Feminino', email: 'clara@exemplo.com', phone: '(51) 93333-7745', city: 'Porto Alegre', notes: 'Simulação de cenário para planejamento.' },
  { id: 8, name: 'Expedito Rocha', cpf: '⬢⬢⬢.317.946-⬢⬢', birth: '1959-03-12', sex: 'Masculino', email: 'expedito@exemplo.com', phone: '(51) 92222-6608', city: 'Cachoeirinha', notes: 'Pensão por morte; dependente em conferência.' }
];

let CASES = [
  { id: 1, client: 1, name: 'Aposentadoria por tempo de contribuição', kind: 'RGPS', rule: 'Tempo de contribuição, EC 103/2019', dib: '2026-08-01', status: 'Calculado', rmi: 4812.37, months: 431, contribs: 431, start: '1990-03', end: '2026-06', updated: '2026-10-24', cnisCount: 610, adjustments: 4, used: 606, pending: 1, cnisPages: 23, importedAt: '2026-10-12', disability: 'Não declarada', ctc: 'Não se aplica', special: 'Não considerado', rural: 'Não declarada', pendencies: [{ tone: 'warn', t: 'Competência 2010-11 ajustada', d: 'Ajuste manual sobre o CNIS; revise a fonte antes de exportar.' }, { tone: 'ok', t: 'CNIS conferido em 12/10/2026', d: '23 páginas e 610 lançamentos; nenhuma diferença de salário por competência.' }] },
  { id: 2, client: 2, name: 'Regra de transição por pontos', kind: 'RGPS', rule: 'Pontos, EC 103/2019', dib: '2026-11-01', status: 'Em revisão', rmi: null, months: 481, contribs: 481, start: '1986-05', end: '2026-07', updated: '2026-10-22' },
  { id: 3, client: 3, name: 'Aposentadoria da pessoa com deficiência', kind: 'PcD', rule: 'LC 142/2013, grau moderado', dib: '2026-09-01', status: 'Calculado', rmi: 3964.10, months: 327, contribs: 327, start: '1999-04', end: '2026-06', updated: '2026-10-21' },
  { id: 4, client: 4, name: 'Aposentadoria por idade', kind: 'RGPS', rule: 'Idade, regra permanente', dib: '2026-12-01', status: 'Pendente', rmi: null, months: 280, contribs: 280, start: '2002-01', end: '2025-04', updated: '2026-10-19' },
  { id: 5, client: 5, name: 'Revisão de RMI', kind: 'RGPS', rule: 'Revisão administrativa', dib: '2024-03-01', status: 'Calculado', rmi: 5208.89, months: 507, contribs: 507, start: '1984-01', end: '2026-06', updated: '2026-10-20' },
  { id: 6, client: 6, name: 'Aposentadoria por tempo especial', kind: 'RGPS', rule: 'Tempo especial, ruído e calor', dib: '2026-10-01', status: 'Bloqueado', rmi: null, months: 377, contribs: 377, start: '1995-01', end: '2026-05', updated: '2026-10-18' },
  { id: 7, client: 7, name: 'Simulação de planejamento', kind: 'RGPS', rule: 'Cenário projetado', dib: '2031-01-01', status: 'Rascunho', rmi: null, months: 91, contribs: 91, start: '2019-01', end: '2026-07', updated: '2026-10-16' },
  { id: 8, client: 8, name: 'Pensão por morte', kind: 'RGPS', rule: 'Dependente, art. 16', dib: '2026-07-01', status: 'Em revisão', rmi: null, months: 258, contribs: 258, start: '2005-01', end: '2026-06', updated: '2026-10-15' },
  { id: 9, client: 1, name: 'Aposentadoria híbrida, cenário 2', kind: 'RGPS', rule: 'Híbrida, EC 103/2019', dib: '2026-08-01', status: 'Calculado', rmi: 4390.12, months: 431, contribs: 431, start: '1990-03', end: '2026-06', updated: '2026-10-24' }
];

let PERIODS = {
  1: [
    { employer: 'Metalúrgica Andrade S/A', kind: 'Empregado', start: '1990-03', end: '1997-12', contribs: 94, status: 'conferido', source: 'CNIS p. 4 a 6', cnpj: '12.345.678/0001-90' },
    { employer: 'Comércio Silva & Cia.', kind: 'Empregado', start: '1998-02', end: '2003-09', contribs: 68, status: 'conferido', source: 'CNIS p. 7 a 9', cnpj: '98.765.432/0001-10' },
    { employer: 'Contribuinte individual', kind: 'Autônomo', start: '2004-01', end: '2010-11', contribs: 83, status: 'ajustado', source: 'CNIS p. 11 a 13 + ajuste', cnpj: '—' },
    { employer: 'Tecelagem Bandeirante Ltda.', kind: 'Empregado', start: '2011-01', end: '2026-06', contribs: 186, status: 'conferido', source: 'CNIS p. 14 a 23', cnpj: '45.678.901/0001-23' }
  ],
  2: [
    { employer: 'Companhia Têxtil do Sul', kind: 'Empregado', start: '1986-05', end: '1994-03', contribs: 95, status: 'conferido', source: 'CNIS p. 3 a 7', cnpj: '11.222.333/0001-44' },
    { employer: 'Transportes Lima', kind: 'Empregado', start: '1994-05', end: '2005-09', contribs: 137, status: 'pendente', source: 'CNIS p. 9 a 14', cnpj: '55.666.777/0001-88' },
    { employer: 'Autônomo', kind: 'Autônomo', start: '2005-11', end: '2026-07', contribs: 249, status: 'conferido', source: 'CNIS p. 16 a 22', cnpj: '—' }
  ],
  3: [
    { employer: 'Confecções Sul Ltda.', kind: 'Empregado', start: '1999-04', end: '2007-08', contribs: 101, status: 'conferido', source: 'CNIS p. 4 a 8', cnpj: '22.333.444/0001-55' },
    { employer: 'Contribuinte individual', kind: 'Autônomo', start: '2007-09', end: '2016-02', contribs: 102, status: 'conferido', source: 'CNIS p. 10 a 15', cnpj: '—' },
    { employer: 'Farmácia Popular Centro', kind: 'Empregado', start: '2016-03', end: '2026-06', contribs: 124, status: 'conferido', source: 'CNIS p. 17 a 21', cnpj: '77.888.999/0001-66' }
  ],
  5: [
    { employer: 'Banco Regional Sul', kind: 'Empregado', start: '1984-01', end: '1991-12', contribs: 96, status: 'conferido', source: 'CNIS p. 3 a 6', cnpj: '33.444.555/0001-77' },
    { employer: 'Autônomo', kind: 'Autônomo', start: '1992-02', end: '2000-10', contribs: 105, status: 'conferido', source: 'CNIS p. 8 a 12', cnpj: '—' },
    { employer: 'Serviços Financeiros Alfa', kind: 'Empregado', start: '2001-01', end: '2018-12', contribs: 216, status: 'ajustado', source: 'CNIS p. 14 a 22 + ajuste', cnpj: '66.777.888/0001-99' },
    { employer: 'Consultoria Previdenciária', kind: 'Empregado', start: '2019-01', end: '2026-06', contribs: 90, status: 'conferido', source: 'CNIS p. 23 a 26', cnpj: '10.203.040/0001-50' }
  ]
};
