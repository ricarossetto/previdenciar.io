# Changelog

## 0.8.2-beta.1 - 2026-10-13

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
