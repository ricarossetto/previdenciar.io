## previdenciar.io 0.13.0

O fluxo guiado virou uma mesa de trabalho: papel, tinta, uma pergunta por tela e um único próximo passo. A base de cálculo e relatórios é a mesma da 0.12.1.

### Nova direção no assistente

- Superfície de papel, títulos em serifa e uma única cor de ação. Tokens documentados em `DESIGN.md`.
- Uma pergunta por tela, opções em linhas e tipografia maior. "Passo X de 7" em linguagem simples, sem caixa alta.
- Contraste corrigido em contexto do caso, pergunta, listas de etapas, campos e painéis.
- A etapa abre no topo, sem herdar a rolagem da tela anterior.

### Clareza no resultado

- Faixa de estado no topo: "Cálculo bloqueado", com o motivo em uma frase, ou "Resultado experimental", com o aviso de revisão.
- Bloqueio da LC 142/2013 coloca "Abrir o cenário PcD no editor" como ação principal; reexecutar sai do caminho.
- Ajuda curta em DIB, sexo e deficiência; finalidades secundárias recolhidas.

### Correções da rodada anterior

- Identificação pessoal oculta no PDF quando a opção está desmarcada.
- Prévia do PDF traz o documento para a tela e o texto de estado acompanha o que existe.
- "Pronto para o motor" deixa de aparecer quando a execução está bloqueada.

### Verificação

- 486 testes Rust, 108 de interface e 36 de navegador.
- Capturas em 1440 e 390 do fluxo redesenhado.

### Limites

- A direção nova cobre o assistente, a conferência e o resultado. As demais páginas mantêm a casca atual.
- A LC 142/2013 continua como simulação com premissas declaradas.
