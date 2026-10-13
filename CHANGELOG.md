# Changelog

## 0.8.1-beta.3 - 2026-10-13

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
