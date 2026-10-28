const DEMO_CONTRIBS = [
  { comp: '2026-05', link: 'Tecelagem Bandeirante', value: '4.120,00', origin: 'CNIS', status: 'conferido' },
  { comp: '2026-04', link: 'Tecelagem Bandeirante', value: '4.120,00', origin: 'CNIS', status: 'conferido' },
  { comp: '2026-03', link: 'Tecelagem Bandeirante', value: '3.980,00', origin: 'CNIS', status: 'conferido' },
  { comp: '2026-02', link: 'Tecelagem Bandeirante', value: '3.980,00', origin: 'CNIS', status: 'conferido' },
  { comp: '2026-01', link: 'Tecelagem Bandeirante', value: '3.980,00', origin: 'CNIS', status: 'conferido' },
  { comp: '2025-12', link: 'Tecelagem Bandeirante', value: '3.980,00', origin: 'CNIS', status: 'conferido' },
  { comp: '2025-11', link: 'Tecelagem Bandeirante', value: '3.850,00', origin: 'CNIS', status: 'conferido' },
  { comp: '2010-11', link: 'Contribuinte individual', value: '1.420,00', origin: 'Ajuste manual', status: 'ajustado' },
  { comp: '2010-10', link: 'Contribuinte individual', value: '1.420,00', origin: 'CNIS', status: 'conferido' },
  { comp: '2003-09', link: 'Comércio Silva & Cia.', value: '1.240,00', origin: 'CNIS', status: 'conferido' },
  { comp: '2003-08', link: 'Comércio Silva & Cia.', value: '1.240,00', origin: 'CNIS', status: 'conferido' },
  { comp: '2003-07', link: 'Comércio Silva & Cia.', value: '1.240,00', origin: 'CNIS', status: 'conferido' },
  { comp: '1997-12', link: 'Cooperativa Metalúrgica', value: '980,00', origin: 'Cooperativa', status: 'excluido' },
  { comp: '1997-11', link: 'Metalúrgica Andrade S/A', value: '⬢ pendente', origin: 'CNIS', status: 'pendente' }
];

let APPOINTMENTS = [
  { date: '2026-10-26', time: '09:00', title: 'Revisão de RMI', who: 'Rita Albuquerque' },
  { date: '2026-10-26', time: '11:30', title: 'Conferência de CNIS', who: 'Joaquim Ferreira Lima' },
  { date: '2026-10-26', time: '15:00', title: 'Retorno de premissas PcD', who: 'Marta Nogueira' },
  { date: '2026-10-28', time: '10:00', title: 'Protocolo administrativo', who: 'Antônio Sales' },
  { date: '2026-10-30', time: '14:30', title: 'Prazo de recurso', who: 'Sebastião Moura' },
  { date: '2026-11-03', time: '09:30', title: 'Entrevista inicial', who: 'Clara Bittencourt' }
];

let CONTRIBUTIONS = { 1: DEMO_CONTRIBS };

const REPORT_TYPES = [
  { id: 'completo', name: 'Relatório completo', desc: 'Todos os anexos, memória, premissas e conferência.' },
  { id: 'resumo', name: 'Resumo executivo', desc: 'Resultado, tempo e pendências em duas páginas.' },
  { id: 'tempo', name: 'Tempo de contribuição', desc: 'Períodos, lacunas e carência por vínculo.' },
  { id: 'remuneracoes', name: 'Remunerações', desc: 'Competências, valores e origem de cada lançamento.' },
  { id: 'rmi', name: 'Memória da RMI', desc: 'Base, coeficiente, teto e atualização.' },
  { id: 'transicao', name: 'Regras de transição', desc: 'Cenários, requisitos e elegibilidade.' },
  { id: 'pendencias', name: 'Pendências e indicadores', desc: 'O que falta e o que bloqueia o resultado.' },
  { id: 'qualidade', name: 'Qualidade de segurado', desc: 'Período de graça, carência e vínculos.' }
];

const SERIES = [
  { id: 'inpc', name: 'INPC', unit: '% mensal', periodicity: 'Mensal', provenance: 'Fábrica de Cálculos/TRF3 · snapshot local', verified: true, entries: [
    { period: '2026-09', value: '0,48', note: 'Variação mensal' }, { period: '2026-08', value: '-0,14', note: 'Variação mensal' },
    { period: '2026-07', value: '0,26', note: 'Variação mensal' }, { period: '2026-06', value: '0,35', note: 'Variação mensal' },
    { period: '2026-05', value: '0,41', note: 'Variação mensal' }, { period: '2026-04', value: '0,52', note: 'Variação mensal' },
    { period: '2026-03', value: '0,44', note: 'Variação mensal' }, { period: '2026-02', value: '0,39', note: 'Variação mensal' }
  ]},
  { id: 'igpdi', name: 'IGP-DI', unit: '% mensal', periodicity: 'Mensal', provenance: 'Fábrica de Cálculos/TRF3 · snapshot local', verified: true, entries: [
    { period: '2026-09', value: '0,71', note: 'Variação mensal' }, { period: '2026-08', value: '0,22', note: 'Variação mensal' },
    { period: '2026-07', value: '0,55', note: 'Variação mensal' }, { period: '2026-06', value: '0,68', note: 'Variação mensal' },
    { period: '2026-05', value: '0,60', note: 'Variação mensal' }, { period: '2026-04', value: '0,83', note: 'Variação mensal' }
  ]},
  { id: 'tetos', name: 'Tetos do RGPS', unit: 'R$', periodicity: 'Anual', provenance: 'Portaria interministerial · exemplo', verified: false, entries: [
    { period: '2026', value: '8.157,41', note: 'Teto vigente' }, { period: '2025', value: '8.157,41', note: 'Teto vigente' },
    { period: '2024', value: '7.786,02', note: 'Teto vigente' }, { period: '2023', value: '7.507,49', note: 'Teto vigente' },
    { period: '2022', value: '7.087,22', note: 'Teto vigente' }, { period: '2021', value: '6.433,57', note: 'Teto vigente' },
    { period: '2020', value: '6.101,06', note: 'Teto vigente' }, { period: '2019', value: '5.839,45', note: 'Teto vigente' }
  ]},
  { id: 'salario', name: 'Salário mínimo', unit: 'R$', periodicity: 'Anual', provenance: 'Norma anual · exemplo', verified: false, entries: [
    { period: '2026', value: '1.621,00', note: 'Valor de exemplo' }, { period: '2025', value: '1.518,00', note: 'Valor de exemplo' },
    { period: '2024', value: '1.412,00', note: 'Valor de exemplo' }, { period: '2023', value: '1.320,00', note: 'Valor de exemplo' },
    { period: '2022', value: '1.212,00', note: 'Valor de exemplo' }, { period: '2021', value: '1.100,00', note: 'Valor de exemplo' },
    { period: '2020', value: '1.045,00', note: 'Valor de exemplo' }, { period: '2019', value: '998,00', note: 'Valor de exemplo' }
  ]}
];
