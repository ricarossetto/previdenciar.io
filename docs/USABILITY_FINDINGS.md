# Teste de usabilidade — advogado idoso (Araken Maica)

Caso: CNIS `cnis araken 03-26.pdf`, deficiência grave, aposentadoria por tempo
de contribuição. Persona: advogado sênior, ~70 anos, pouca familiaridade com
computador. Método: Playwright sobre o WebView2 real (CDP), print e texto de
cada tela, fluxo guiado do início ao relatório.

## Fricções encontradas

### Críticas (travam o usuário)

1. **Etapa 5 (Conferência) é um labirinto.** Aparecem 5 "pendências decisivas"
   que impedem o cálculo, cada uma com um botão "Corrigir":
   - Situação de benefício atual não confirmada
   - Fonte CNIS ainda não excluída com justificativa
   - Política versionada ausente ou ilegível
   - Nenhum vínculo manual selecionado
   - Nenhuma remuneração manual selecionada
   A saída real é o botão "Preparar e calcular", que abre um modal pedindo
   "referência documental da conferência". No teste, mesmo preenchendo a
   referência, as pendências permaneceram — o usuário fica sem caminho.

2. **A resposta da Etapa 2 não propaga.** O usuário responde "Não, o segurado
   não está recebendo benefício" na Etapa 2, mas a Etapa 5 continua marcando
   "Situação de benefício atual não confirmada" e o passo "Situação e origem"
   fica como "Conferência pendente" no cabeçalho de etapas.

3. **Beco sem saída do CNIS.** O motor não calcula sobre a fonte importada:
   exige excluí-la com justificativa e informar vínculos/remunerações manuais.
   O fluxo guiado importa o CNIS mas não deixa claro como transformá-lo em
   entrada do motor.

### Moderadas (corrigidas nesta rodada)

4. **Versão errada no rodapé** — mostrava "Motor Rust · versão 0.9.2" fixo.
   Corrigido para `__APP_VERSION__` (versão real do build).
5. **Etapa 3 — opções não respondem à pergunta.** A pergunta é "O que este
   cálculo precisa resolver?", mas as opções começavam com "Não, pretendo..." e
   "Sim, há...". Reescritas para nomear o objetivo diretamente.
6. **Etapa 4 — rótulo de avanço inconsistente.** O botão para seguir era
   "Conferir períodos", diferente do "Prosseguir" das outras etapas. Renomeado
   para "Prosseguir para a conferência".

### Menores (não corrigidas)

7. Etapa 3 oferece 6 opções de uma vez (Hick's Law).
8. "Informar caminho do arquivo" fica recolhido por padrão na importação.
9. A tabela de remunerações mostra 331 competências de uma vez, cada uma como
   "Conferência pendente" — denso para leitura.

## Recomendações

- **Etapa 5**: transformar as pendências num assistente passo a passo, com o
  botão "Preparar e calcular" em destaque e a explicação de que ele resolve as
  pendências de fonte/vínculo/remuneração.
- **Propagação**: garantir que as respostas da entrevista sobrevivam à
  aplicação do CNIS e apareçam confirmadas na conferência.
- **CNIS → motor**: deixar explícito, na própria conferência, que o CNIS é
  conferido e a hipótese é montada a partir dele em um clique.
