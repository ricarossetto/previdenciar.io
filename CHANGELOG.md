# Changelog

## 0.13.0 - 2026-10-26

Redesign do fluxo guiado: da casca de painel para uma mesa de trabalho.

### Changed
- Nova direção visual no assistente: superfície de papel, tinta escura, títulos em serifa e uma única cor de ação. Tokens declarados em `flow-v2`, documentados em `DESIGN.md`.
- Uma pergunta por tela, com tipografia maior, opções em linhas e um único caminho de saída. "Passo X de 7" em linguagem simples, sem caixa alta e sem rótulos técnicos.
- O contexto do caso, o bloco da pergunta, as listas de etapas e os painéis perderam o fundo escuro herdado da casca; contraste corrigido em todos os campos e cartões.
- Resultado abre com uma faixa de estado: "Cálculo bloqueado" com o motivo em uma frase, ou "Resultado experimental" com o aviso de revisão.
- Quando o bloqueio é a LC 142/2013, a ação primária passa a ser "Abrir o cenário PcD no editor"; o botão de reexecutar sai do caminho.
- A etapa troca de tela já no topo, sem rolar para o meio do conteúdo anterior.

### Added
- `PRODUCT.md` com a verdade do produto e `DESIGN.md` com a direção, tokens e regras de acessibilidade.
- Ajuda curta nos campos de DIB, sexo e deficiência, e finalidades secundárias recolhidas em "Outras finalidades".

### Tests
- 486 testes Rust, 108 de interface e 36 de navegador.

## 0.12.1 - 2026-10-26

Teste de usabilidade completo com o CNIS real do caso PcD, no binário nativo, em perfil isolado.

### Fixed
- **Identificação pessoal no PDF**: com a opção de identificação desmarcada, nome e CPF ainda apareciam nos relatórios Typst. O renderer passou a receber a opção e oculta nome, CPF e a origem do vínculo.
- **Beco sem saída do cenário PcD**: o resultado bloqueado pela LC 142 agora explica o motivo e oferece o botão que abre o cenário PcD no editor, na etapa correta.
- **Prévia do PDF sem resposta visível**: ao gerar a prévia, o documento é trazido para a tela. O texto de estado não manda mais escolher um cálculo que já está escolhido.
- **"Pronto para o motor" com execução bloqueada** passou a "Revisado; execução bloqueada".
- **Reexecução**: com resultado na tela, o botão passa a "Executar novamente" e o subtítulo avisa que o resultado é da última execução.

### Changed
- Etapa 3 mostra três finalidades principais e deixa as outras três em "Outras finalidades". A segunda pergunta virou "O que você quer obter?".
- Etapa 4 ganhou ajuda curta em DIB, sexo e deficiência, em linguagem de quem não usa o programa todos os dias.
- No cenário PcD, a etapa do editor se chama "Cenário PcD (LC 142/2013)" e não oferece o preparo genérico do CNIS, que apagaria a política declarada.

### Tests
- Teste Rust garante que a identificação é ocultada no dado do relatório.
- Teste de interface garante o caminho do bloqueio PcD para o módulo LC 142 e o rótulo de reexecução.

## 0.12.0 - 2026-10-26

### Changed
- **Conferência do assistente (Etapa 5) em dois passos visíveis**: primeiro as respostas da entrevista, com o botão que volta à etapa certa; depois o preparo a partir do CNIS, na própria página.
- **Preparo sem modal**: a referência documental, o que o clique faz e o que ele não faz ficam visíveis, com um único botão "Preparar e calcular". A duplicidade de botões com o mesmo nome deixou de existir.
- **Pendências com um caminho só**: quando as pendências são fonte, política, vínculos e remunerações, o painel informa que um clique as resolve e não oferece mais atalhos para as abas.
- **CNIS explicado como fonte, não como entrada do motor**: o texto diz que o documento é conferido, que a cópia é revisada, que a referência fica registrada e que o cálculo parte dali.
- **Respostas da entrevista não são mais inventadas pelo preparo**: benefício atual, finalidade e objetivo sem resposta continuam pendentes, com aviso e correção na conferência.
- **Períodos concomitantes deixaram de ser pendência decisiva**: o motor soma apenas a parcela não sobreposta e avisa no resultado. O texto anterior dizia que o motor bloqueava a sobreposição, o que não correspondia ao adaptador, e travava o CNIS real com vínculos paralelos.
- **Cenário PcD avisa antes de executar**: ao escolher grau de deficiência, a etapa de dados informa que a LC 142/2013 ainda não tem motor neste adaptador e que a execução será bloqueada com motivo, em vez de deixar o bloqueio aparecer só no resultado.

### Added
- Teste de fluxo guiado com IPC sintético cobrindo entrevista, importação, conferência e preparo em um clique (`apps/desktop/tests/guided-flow.test.tsx`).
- Conferência mostra a resposta sobre benefício atual por extenso, com correção no mesmo lugar.

### Fixed
- Testes de interface que renderizam o aplicativo voltaram a rodar: `__APP_VERSION__` passou a ser definido no `vitest.config.ts`.
- Rótulos desatualizados nos testes do assistente (opções da Etapa 3 e botão de avanço da Etapa 4), herdados da correção de texto anterior.
- Suíte Rust voltou a compilar: o teste de workspace inicializa `Settings.logo_data_url` e o teste de capacidades reconhece as permissões `updater:default` e `process:allow-restart`/`allow-exit` exigidas pelo atualizador assinado. A descrição da capacidade deixou de negar os dois plugins.
- Documentação: removidos nome de cliente e valor de RMI derivado do diagnóstico de usabilidade; o CPF de exemplo do formato de relatório passou a ser sintético.
- Teste Rust garante que o preparo guiado não preenche respostas da entrevista.


## 0.11.1 - 2026-10-26

### Added
- **Regras de Transição - EC 103/2019** (formato detalhado): card por regra com Possível Aposentadoria, Provável RMI, idade e tempo possuídos vs exigidos, e Retorno do Investimento.
- **Migração completa**: Resumo, Salários de contribuição e Indicadores passam ao motor Typst. Todos os 11 tipos de relatório usam agora o layout novo; o genpdf deixa de ser usado na exportação.
## 0.11.0 - 2026-10-26

### Added
- **Novo motor de relatórios PDF (Typst)**: layout idêntico ao produto de referência — logo do escritório, título centralizado, campos em caixa, tabelas com cabeçalho azul-marinho e linhas alternadas, barras de status, rodapé com data e numeração `n/N`.
- **6 relatórios migrados**: Análise da Situação Atual, Tempo de Contribuição, Cálculo da Qualidade de Segurado, Cálculo da R.M.I., Análise das Regras de Transição e Cálculo do Retorno Sobre o Investimento.
- **Documento consolidado** (`detailed`) como merge dos seis, na ordem do produto de referência.
- **Formatação brasileira** centralizada: moeda (`R$ 1.632.377,53`), datas (`22/05/2026`), períodos (`22 anos, 1 mês e 4 dias`).
- **Escala tipográfica única** nos templates (fim do drift de tamanhos ad-hoc).
- ROI com 13º salário, IRRF oficial (tabela embutida) e projeção por regra.

### Changed
- `ReportKind::Roi` adicionado; `ReportKind::slug` e `has_typst_template`.
- `render_report(case, kind, outcome)` é o novo caminho; tipos migrados deixam de usar o genpdf.
- Contribuição mensal (INSS) calculada como 20% do salário de benefício.

### Notes
- Typst adiciona ~15 MB ao binário; a compilação do workspace exige memória (use `-j 1` em máquinas com pouca RAM).
## 0.10.6 - 2026-10-25

### Added
- **Conversao de tempo comum para PcD** (fator 0,93/0,89/0,83 por grau): `disability_start_date` + `fator_conversao_comum` em `Lc142Policy`, aplicado automaticamente a periodos anteriores ao inicio da deficiencia. Testes cobrem conversao, config incompleta e fator fora de faixa.
- **Merge de PDFs**: comando `merge_pdfs` com seletor de arquivos, funcao `previus_reports::merge_pdfs` que combina N PDFs preservando ordem das paginas. 4 testes unitarios.
- **Paginacao n/N no rodape do PDF**: renderizacao em duas passadas (primeira conta paginas, segunda renderiza com total). Rodape agora mostra "Pagina X/Y".
- **Planejamento (projecao de DER)**: comando Tauri `project_rgps` expoe `rgps_core::projecao::projetar` — simula DER futura com contribuicoes continuas ate a primeira regra elegivel.
- **Analise da Situacao Atual**: novo tipo de relatorio PDF (`pdf_analysis`) com requisitos apurados, cenario financeiro (A PAGAR vs A RECEBER) e ROI (projecao 371 meses, 13o, IRRF simplificado, payback).

### Changed
- `Lc142Policy` aceita `disability_start_date` e `fator_conversao_comum` para conversao de tempo comum.
- `duration()` em `pcd.rs` agora aceita parametros de conversao opcionais.
- Relatorio PDF agora inclui secao de ROI quando o tipo `analysis` e selecionado.
## 0.10.5 - 2026-10-24

Preparação guiada no ponto de decisão.

- O bloco "Preparar e calcular a partir do CNIS" virou componente único, usado na conferência do assistente e nos parâmetros do cálculo.
- Com CNIS importado, a política manual fica recolhida em "Ajustar manualmente"; o caminho guiado aplica correção INPC até a DIB, sem descarte e com teto vigente.
- Sem CNIS ou em cenário PcD, a política permanece aberta.
- Nenhuma mudança de cálculo; testes de interface e navegador mantidos.

## 0.10.4 - 2026-10-23

Correções da revisão de código.

- Comandos `local_economic_snapshot` e `prepare_case_from_cnis` registrados no handler do Tauri.
- Modal da preparação guiada movido para o retorno principal; o botão abre o diálogo, prepara e calcula.
- Fator previdenciário com alíquota fixa de 0,31 e acréscimo de cinco anos ao tempo da mulher.
- Tábua de expectativa gerada pelo nome das colunas; idades 78 a 90 corrigidas.
- Descarte da EC 103 recusa mínimo zero; IRRF sem faixa silenciosa; preparação guiada ignora valores de contribuição não remuneratórios.
- Auditoria de UI sem hash do executável, endereços ou evidências locais.

## 0.10.3 - 2026-10-23

Preparação guiada a partir do CNIS e cálculo em um clique.

- A tela de parâmetros prepara a hipótese a partir do CNIS com decisões explícitas: exclui vínculos PRPPS, fecha vínculo sem data final pelo último salário, soma remunerações concomitantes do RGPS, limpa indicadores informativos e registra tudo em notas. Em seguida calcula e abre o resultado.
- Vínculos sobrepostos deixam de bloquear: o motor deduplica totais e marcos desde 0.9.5.
- Sem INPC para a DIB, a correção é desativada e o resultado é nominal, com aviso.
- Textos da tela de parâmetros reduzidos; explicações longas movidas para um bloco recolhível.

## 0.10.2 - 2026-10-23

Salário mínimo no PBC do RGPS comum.

- Competências com valor abaixo do salário mínimo vigente (PREC-MENOR-MIN) deixam de entrar no PBC também no caminho RGPS comum, com aviso e lista das competências. A regra já valia no cenário PcD.

## 0.10.1 - 2026-10-23

Revisão de texto em todo o projeto.

- Travessões substituídos por vírgulas, dois-pontos ou pontos; aspas curvas convertidas em aspas retas; reticências em três pontos; títulos em caixa de frase. Nenhum cálculo, código de bloqueio ou dado foi alterado.

## 0.10.0 - 2026-10-23

Abertura da base fora da thread principal e documentação das regras novas.

- A abertura da base local (DPAPI, validação e serialização) roda em `spawn_blocking`. Uma falha mantém a gravação bloqueada com mensagem, em vez de congelar a interface.
- As limitações passam a documentar a espécie 42, o fator previdenciário com tábua de expectativa embutida e o IRRF mensal oficial.

## 0.9.9 - 2026-10-23

Descarte dos menores salários da EC 103.

- Opção explícita de descarte (art. 26, § 6º) com o mínimo de competências que preserva o direito declarado pelo operador. O motor remove as menores remunerações corrigidas até esse piso e lista as competências descartadas.
- A opção exige correção ativa e não se combina com o descarte fixo de 20%.

## 0.9.8 - 2026-10-23

IRRF mensal oficial.

- Tabela por competência de 1989 a 2026, com a vigência atual conferida contra a Lei 14.848/2024.
- Ferramenta `irrf_oficial` com dedução de dependentes e isenção de 65 anos declarada pelo operador. Competências anteriores a 07/1994 são recusadas para não converter moeda histórica.

## 0.9.7 - 2026-10-23

Fator previdenciário e tábua de expectativa.

- Alíquota fixa de 0,31 e acréscimo de cinco anos ao tempo da mulher (Lei 9.876/1999), sem piso artificial em 1.
- Tábua de expectativa de sobrevida embutida, com vigências de 1992 a 2025 por idade e sexo. O pedágio de 50% calcula sem expectativa declarada.

## 0.9.6 - 2026-10-23

Espécie de benefício no contrato.

- Apenas a espécie 42 tem motor. As demais bloqueiam com motivo no motor, no adaptador e na interface, em vez de aproximar coeficiente.

## 0.9.5 - 2026-10-23

Transições e qualidade.

- Transições medidas por tempo apurado, com corte em 13/11/2019 e marcos sem dupla contagem de períodos sobrepostos.
- Qualidade de segurado unificada em 12 meses, com a prorrogação para 24 meses dependente do histórico.

## 0.9.4 - 2026-10-23

Salário mínimo e snapshot PcD.

- Série de salário mínimo por competência embutida (1994-07 a 2026-09).
- Snapshot local usa piso e teto de cada competência, corrigindo o caminho offline.

## 0.9.3 - 2026-10-23

Correções P0 e paridade PcD.

- Índice global dos valores consolidados do CNIS, sem rejeitar a importação inteira.
- Duplicação de cálculo com linhas manuais, com identificadores novos.
- Origem dos valores na revisão de contribuições, impedindo que contribuição paga vire salário.
- Remoção do comando de restauração que sempre falhava.
- Paridade PcD com o motor público: PBC 357, divisor 285 e R$ 5.200,25 contra R$ 5.200,88 (0,012%).

## 0.9.2 - 2026-10-22

Carência do contribuinte individual e snapshot oficial da Fábrica de Cálculos.

- **Art. 27, II, da Lei nº 8.213/1991 implementado**: para contribuinte individual, especial e facultativo a carência conta a partir do **primeiro pagamento sem atraso**, e as competências anteriores marcadas na origem como informadas fora do prazo (indicador `PREM-EXT`) não entram. Para empregado, doméstico e avulso vale o art. 27, I (desde a filiação). O relatório passou a imprimir o **fundamento da contagem por vínculo**.
- **Snapshot da consulta pública aceita a vigência**: o piso e o teto usam a última competência informada até o mês de referência (o teto é anual) e o snapshot publicado até o mês anterior à correção é aceito. Antes, o mês exato era exigido e o snapshot oficial de setembro era recusado para uma DIB de outubro.
- **Consulta pública exercitada de ponta a ponta no aplicativo**: `fetch_economic_snapshot` retornou 387 competências com INPC, INPC-INSS, IGP-DI, IPC-r, piso e teto oficiais em menos de um segundo, com as fontes declaradas por competência (ex.: piso de 2026 pelo Decreto nº 12.797/2025 e teto pela Portaria Interministerial MPS/MF nº 13/2026).
- **Comparação com o relatório de referência com os índices oficiais**: PBC de 364 parcelas, 80% = 291, soma R$ 1.501.880,78 e **RMI de R$ 5.161,10** contra R$ 5.200,89 do software de referência, diferença de **0,77%**, atribuída às sete competências que o CNIS lista apenas na visão "Contribuições" (ver a limitação declarada).

## 0.9.1 - 2026-10-22

Correções de leitura de origem e o cenário PcD executável sem rede.

- **Importação de CNIS corrigida em três defeitos**, todos achados num caso real de 23 páginas: a tabela de contribuições (competência, data de pagamento, contribuição e salário) continuava na página seguinte sem repetir o cabeçalho e as linhas seguintes eram descartadas; a nota de rodapé do CNIS era confundida com um cabeçalho de tabela e marcava salários válidos como "contribuição não resolvida"; e o contexto de tabela não atravessava a quebra de página. Com as correções, a leitura do caso real passou de 485 para 610 lançamentos e os salários por competência passaram a coincidir **um a um** com o relatório de referência.
- **Série oficial do IGP-DI/FGV embutida** (SGS/BCB, série 190) e INPC atualizado até 2026-09, gerados por `scripts/gerar-dados-economicos.mjs` a partir da fonte pública, com a fonte declarada no próprio arquivo.
- **Snapshot econômico local**: o cenário PcD passou a poder ser calculado sem a Fábrica de Cálculos do TRF3, usando as séries e tetos oficiais embutidos, com a proveniência declarada (URL local e SHA-256 das tabelas). A consulta pública continua disponível e tem precedência.
- **Comparação de índices com a referência**: de 2009 em diante os fatores de correção do motor coincidem com os do relatório de referência; nos meses anteriores a diferença residual (até 1,3%) vem da política declarada de variações negativas históricas.
- Competência sem lançamento na origem deixa de bloquear o cálculo: ela fica **fora do PBC** (não é tempo de contribuição) e a omissão é declarada na lista de avisos. Nunca vira zero.

## 0.9.0 - 2026-10-22

Primeira versão estável da linha 0.9. O motor continua experimental: resultados, carência e enquadramento exigem revisão profissional.

- Apuração da qualidade de segurado por vínculo com cálculo próprio e fonte primária citada no documento: período de graça de 12 meses (art. 15, II, da Lei nº 8.213/1991), prorrogação para até 24 meses com mais de 120 contribuições sem interrupção (art. 15, § 1º), carência de 180 contribuições (art. 25, II) ou faixa da tabela do art. 142, tempo em gozo de benefício fora da carência (art. 19-C, § 1º, do Decreto nº 3.048/1999) e competência contada uma única vez quando aparece em mais de um vínculo.
- Novo 8º relatório PDF "Cálculo da Qualidade de Segurado", na central, no assistente e no lote.
- Importação de CNIS corrigida: linhas de cooperativa com marcador "Não Cooperado" e a tabela de valores consolidados por ano civil, lida pela posição do valor na página. Em um CNIS real de 23 páginas, a leitura passou de 300 para 357 competências.
- Renderer: correção de perda silenciosa de texto em colunas estreitas, com a linha reconstruída a cada render. Vale para todos os tipos de relatório.
- Título do relatório editável e persistido, e opções de apresentação (anexo técnico, conferência de qualidade, premissas de planejamento).
- Assistente em sete etapas com hub de conferência, mapa de cobertura e revisão rastreável dos dados importados.
- Verificação: 454 testes Rust, 97 testes de interface, 36 testes de navegador, revisão de fonte pública sem achados, e verificação nativa em perfil isolado com os 8 PDFs.
- A regra de carência "4/5" (144 contribuições) foi refutada em dez fontes primárias e não é aplicada; a base legal da verdade é art. 22 da EC 103/2019 → art. 8º da LC 142/2013 → art. 29, II, da Lei nº 8.213/1991 (média dos 80% maiores salários).
- A REA (Ghidra) não encontra no executável original os rótulos do relatório de qualidade: as fórmulas de referência não são recuperáveis e nada foi inferido no lugar delas.

## 0.8.2-beta.3 - 2026-10-13

- Ajustado tratamento de exit code do smoke nativo para runners headless em CI.

- Adicionado gate estático de verificação de assets embutidos no binário de release para ambientes de CI.

- As tabelas Vínculos do CNIS e Competências e remunerações agora abrem edição ao clicar nas células/linhas, com botões nomeados e acesso por teclado.
- Ajustes de vínculos declaram empregador, início, fim, regime, indicadores e conferência; mantêm a origem e um histórico documental validado no Rust.
- Fim candidato não é preenchido automaticamente; PRPPS continua exigindo CTC específica, mesmo se o campo de regime for editado para RGPS.
- Valores de competências históricas podem ser revisados sem conversão monetária presumida. O seletor do mês lista os lançamentos separados e abre o ajuste individual.
- A tabela de origem continua visível após iniciar o livro e mostra os ajustes separados. Alterações invalidam o resultado e a revisão anterior.
- PDFs reais de períodos/detalhado incluem a revisão dos vínculos, ocultando motivos livres no modo privado.

A interface é própria, não uma reprodução certificada do painel proprietário. Após salvar os campos novos, use esta versão ou posterior e preserve backups.


## 0.8.1-beta.5 - 2026-10-13

- Livro de contribuições com edição da hipótese, origem preservada, moeda/vínculo, filtros, seleção, ações em lote, consolidação documentada, histórico e múltiplos desfazeres.
- Aplicação explícita aos salários com validação equivalente no Rust: origem, histórico e projeção divergentes são recusados.
- Alterações invalidam resultado, status e revisão do assistente; origem já revisada não é substituída por nova importação.
- PDFs reais incorporam ajustes e histórico, ocultando justificativas/motivos livres no modo privado.
- Correção do diálogo aninhado que ficava inerte com o painel; foco, Escape e pilha modal cobertos por regressão.
- Valores/somas em centavos exatos, sem preparação monetária por ponto flutuante.
- Contraste do diálogo Novo cálculo e do pico do brilho dos botões verificável sem ignorar regras; testes axe-core e luminância adicionados. A beta.1 foi substituída após apontamento do CI no botão animado. A beta.2 foi marcada como inadequada após detectar assets incompletos no empacotamento concorrente; a beta.3 exige build sequencial e smoke nativo antes da publicação.
- Release passa a exigir testes de navegador, revisão pública e coerência entre tag e versões, e publica BUILDINFO.json.

A identidade definitiva, assinatura de código e cobertura normativa universal não são entregues nem presumidas nesta correção. Após salvar o novo livro, use esta versão ou posterior para abrir a base.


## 0.8.0-beta.1 - 2026-10-13

Primeira árvore de código pública de previdenciar.io, com identidade textual provisória.

- Interface desktop local em React/Tauri e motores Rust experimentais.
- Importação CNIS textual, entrevista, editor manual e PDFs reais.
- Animações adaptadas do Amicro com modo reduzido e notices MIT.
- Cópia pública por lista permitida, sem documentos pessoais, evidências privadas, ferramentas de investigação ou binários proprietários.
- Licença MIT, notices das fontes/componentes, documentação, CI, release Windows e site estático.
- Editor completo de contribuições importadas registrado como prioridade futura, não anunciado como pronto.
- Logo/identidade final aguardam pacote aprovado do OpenDesign.

Consulte docs/LIMITATIONS.md. Esta versão não é certificada juridicamente nem equivalente a outro produto.

A beta.4 reforça o contraste do status neutro e remove fade de texto em diálogos; as transições geométricas são mantidas.

A beta.5 remove a causa do fade residual: @starting-style herdado aplicava opacity:0 além da animação Motion. O diálogo agora mantém opacidade 1 desde a montagem, com um único responsável pelo movimento geométrico. O teste mede a opacidade imediatamente, sem esperar a animação acabar ou ignorar contraste.
