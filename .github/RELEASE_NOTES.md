## previdenciar.io 0.12.1

Correções do teste de usabilidade completo com um CNIS real de caso PcD, executado no binário nativo em perfil isolado. A base de recursos está na 0.12.0.

### Correções

- Identificação pessoal: com a opção desmarcada, o PDF deixa de mostrar nome e CPF do segurado e a origem do vínculo.
- Cenário PcD: o resultado bloqueado pela LC 142/2013 agora explica o motivo e abre o módulo LC 142 do editor na etapa certa.
- Prévia do PDF: o documento gerado é trazido para a tela, e o texto de estado deixa de pedir um cálculo que já está escolhido.
- Resultado bloqueado: o selo "Pronto para o motor" virou "Revisado; execução bloqueada".
- Reexecução: com resultado na tela, o botão diz "Executar novamente".

### Clareza para quem usa pouco o computador

- Etapa 3 com três finalidades principais e "Outras finalidades" para as demais.
- Ajuda curta em DIB, sexo e deficiência.
- No modo PcD, a etapa do editor se chama "Cenário PcD (LC 142/2013)" e não mostra o preparo genérico do CNIS.

### Verificação

- 486 testes Rust, 108 de interface e 36 de navegador.
- Fluxo nativo repetido com o CNIS real: conferência, preparo, resultado bloqueado e prévia em PDF com identificação oculta.

### Limites

- A LC 142/2013 continua como simulação com premissas declaradas, não como cálculo de concessão.
- O motor RGPS cobre a espécie 42; as demais bloqueiam com motivo.
- A distribuição não tem assinatura de código. Confira o SHA-256 publicado.
