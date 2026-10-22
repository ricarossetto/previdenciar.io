# Changelog

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
