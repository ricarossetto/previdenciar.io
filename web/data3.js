let DOCUMENTS = [
  { id: 1, name: 'Procuração previdenciária', category: 'Administrativo', updated: '2026-10-12', body: 'OUTORGANTE: {{nome}}, CPF {{cpf}}, nascido em {{nascimento}}. OUTORGADO: {{escritorio}}. Poderes para representar o outorgante perante o INSS, especialmente no cálculo {{calculo}}, DIB {{dib}}.' },
  { id: 2, name: 'Requerimento de aposentadoria', category: 'Administrativo', updated: '2026-10-08', body: 'Requerimento de aposentadoria em nome de {{nome}}, CPF {{cpf}}, com data de início do benefício em {{dib}}. Hipótese: {{calculo}}. Documentos anexos conforme relação.' },
  { id: 3, name: 'Recurso administrativo', category: 'Recurso', updated: '2026-09-27', body: 'Recurso contra decisão administrativa referente ao cálculo {{calculo}}. Razões de fato e de direito, com memória de cálculo anexa e pedido de revisão.' },
  { id: 4, name: 'Parecer técnico', category: 'Consultivo', updated: '2026-09-18', body: 'Parecer sobre a hipótese {{calculo}}, considerando os vínculos e as contribuições conferidas, as premissas declaradas e as limitações do motor experimental.' },
  { id: 5, name: 'Contrato de honorários', category: 'Contratual', updated: '2026-08-30', body: 'Contrato de prestação de serviços entre {{escritorio}} e {{nome}}, CPF {{cpf}}, para acompanhamento do cálculo {{calculo}} e dos atos administrativos correlatos.' },
  { id: 6, name: 'Declaração de hipossuficiência', category: 'Declaração', updated: '2026-08-21', body: 'Declaro, para os fins do art. 98 do CPC, que não possuo condições de arcar com as custas e honorários sem prejuízo do sustento próprio e da família.' }
];

const MODULES = [
  { name: 'Planejamento contributivo', desc: 'Compara cenários de contribuição e projeção de tempo. Depende de regras e índices que ainda não estão no motor.', status: 'Não implementado' },
  { name: 'Alíquota e contribuição', desc: 'Calcula alíquota efetiva e contribuição mensal por faixa. Falta validar o regime e a competência.', status: 'Não implementado' },
  { name: 'IRRF sobre benefício', desc: 'Simula retenção de imposto de renda com dependentes e deduções. Fora do escopo do motor atual.', status: 'Não implementado' },
  { name: 'Retorno e projeção', desc: 'Estima o retorno de diferentes idades de início. Precisa de premissas atuariais declaradas.', status: 'Não implementado' }
];

const ASSISTANT = [
  { title: 'Identificação', question: 'Quem é o segurado e qual benefício será analisado?', text: 'A identificação vem do cadastro do cliente e não é inferida pelo nome do arquivo. O assistente não confirma sexo, deficiência ou datas que o documento não informa.', evidence: ['CNIS p. 1', 'Cadastro do cliente'], choices: [] },
  { title: 'Vínculos', question: 'Os vínculos do CNIS batem com os documentos?', text: 'Cada vínculo é conferido com a página de origem, o empregador e o período. Divergências ficam marcadas para revisão.', evidence: ['4 vínculos', '23 páginas'], choices: [] },
  { title: 'Contribuições', question: 'As competências de 2004 a 2010 são de contribuinte individual?', text: 'O CNIS lista 83 competências nesse intervalo. A origem e o valor precisam ser conferidos antes de entrar no período básico de cálculo.', evidence: ['CNIS p. 11 a 13', '83 competências'], choices: ['Sim, todas são de contribuinte individual', 'Não, parte do período era empregado', 'Preciso conferir o CNIS antes de responder'] },
  { title: 'Benefício', question: 'Qual regra deve ser testada primeiro?', text: 'O assistente organiza a hipótese, mas não escolhe a regra pelo segurado. A comparação fica registrada na memória.', evidence: ['DIB 01/08/2026', 'Regra de transição'], choices: [] },
  { title: 'Premissas', question: 'Quais premissas precisam ser declaradas?', text: 'Sexo cadastral, deficiência, CTC e datas não são presumidos. Cada premissa recebe uma origem e uma consequência se mudar.', evidence: ['4 premissas'], choices: [] },
  { title: 'Conferência', question: 'O quadro de conferência está fechado?', text: 'O quadro mostra o que veio do CNIS, o que foi ajustado e o que o motor usou. Enquanto houver pendência, o resultado fica marcado.', evidence: ['606 usados', '4 ajustes'], choices: [] },
  { title: 'Resultado', question: 'O resultado pode ser levado ao relatório?', text: 'O resultado é experimental e não certifica direito. A revisão profissional e a conferência documental continuam obrigatórias.', evidence: ['RMI 4.812,37', '2 pendências'], choices: [] }
];

const RGPS_SCENARIOS = [
  { id: 1, name: 'Regra permanente', req: '62 anos e 15 anos de carência', pos: '58 anos e 35a 11m', status: 'Falta idade', tone: 'bad', rmi: '—' },
  { id: 2, name: 'Pedágio de 50%', req: '30 anos + pedágio sobre o tempo restante', pos: '35a 11m; pedágio cumprido', status: 'Elegível', tone: 'ok', rmi: 'R$ 4.812,37' },
  { id: 3, name: 'Pontos', req: '93 pontos em 2026 (mulher)', pos: '93 pontos', status: 'Elegível', tone: 'ok', rmi: 'R$ 4.690,12' },
  { id: 4, name: 'Idade progressiva', req: '63 anos e 6 meses em 2026', pos: '58 anos', status: 'Falta 5a 6m', tone: 'bad', rmi: '—' }
];

const COVERAGE = [
  { surface: 'Análise nativa', observed: '1.902 funções e 1.196 strings', inferred: '—', coverage: 'Observado', tone: 'ok' },
  { surface: 'Caminho do PDF', observed: '7 funções, 6 criações observadas', inferred: 'Fórmulas previdenciárias', coverage: 'Parcial', tone: 'warn' },
  { surface: 'Motor RGPS', observed: 'Adaptador restritivo e avisos', inferred: 'Regras não cobertas', coverage: 'Parcial', tone: 'warn' },
  { surface: 'Assistente', observed: 'GUI por acessibilidade', inferred: 'Fluxo interno do assistente', coverage: 'Inferido', tone: 'quiet' },
  { surface: 'INFOEXE', observed: '13.587.224 bytes', inferred: 'Conteúdo opaco', coverage: 'Sem cobertura', tone: 'bad' }
];

const PDF_PATH = ['Wrapper de arquivo', 'Stream', 'Configuração', 'Saída', 'Objetos', 'Laço de relatórios', 'Render'];
