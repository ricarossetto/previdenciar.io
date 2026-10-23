# Changelog

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

- Alíquotas corrigidas para 0,31 (homem) e 0,30 (mulher), sem piso artificial em 1.
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
