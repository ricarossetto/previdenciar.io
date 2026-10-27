# Teste de usabilidade: advogado idoso com CNIS real

Caso: CNIS real de um cliente, mantido fora do repositório, deficiência grave, aposentadoria por tempo
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

## Estado em 0.12.0 (revisão desta rodada)

Uma revisão do código e a repetição do fluxo com IPC sintético (mesmo estado de
interface, sem transporte nativo) mudaram o diagnóstico dos itens críticos. O
que segue foi verificado, não presumido.

### 1. Etapa 5 (Conferência): corrigida

- A conferência agora abre com **1 de 2: respostas da entrevista** (benefício
  atual, finalidade, objetivo, segurado, sexo/deficiência e DIB), cada bloco com
  o botão que volta à etapa certa.
- **2 de 2** é o preparo a partir do CNIS, aberto na própria página, sem modal:
  a referência documental, a lista do que o clique faz (exclui a fonte como
  entrada, monta vínculos e remunerações, aplica a política e calcula) e o que
  ele não faz (não altera o CNIS, não comprova direito).
- Quando as pendências são exatamente fonte, política, vínculos e remunerações,
  o painel de rastreabilidade diz que **um clique** as resolve e deixa de
  oferecer os atalhos "Corrigir" que levavam às abas. O caminho de saída passa a
  ser um só e visível.
- O botão deixou de existir duas vezes (um na página, outro no modal). Isso
  também eliminou a ambiguidade que quebrou a automação do teste anterior.

### 2. Resposta da Etapa 2: não se reproduziu, coberta por teste

- O fluxo completo foi repetido na interface atual, com CNIS sintético e com o
  mesmo CNIS real usado no teste original (dados fora do repositório). Em ambos,
  a resposta "não recebe benefício" chega à conferência: a etapa aparece
  confirmada e a pendência "Situação de benefício atual não confirmada" não é
  emitida.
- As evidências originais são compatíveis com um artefato de automação: o print
  da Etapa 5 mostra a lista de cinco pendências, que é exatamente o estado
  **anterior** a qualquer preparo, e o print seguinte mostra o modal ainda
  aberto, com a referência digitada e **sem submissão**. O rodapé dessas telas
  exibia "versão 0.9.2" fixo, então a versão do binário não pode ser inferida
  pelo rodapé.
- Para não depender de conclusão manual, `apps/desktop/tests/guided-flow.test.tsx`
  reproduz o fluxo inteiro e falha se a resposta não chegar ao preparo, ao
  cálculo ou à conferência.
- A conferência passou a mostrar a resposta por extenso, de modo que uma perda
  futura fique visível e corrigível no mesmo lugar.

### 3. CNIS para o motor: explicado no ponto de decisão

- O texto do preparo diz que o CNIS é documento conferido, não entrada do
  motor, e que o clique revisa a cópia, registra a referência e calcula.
- O preparo com o CNIS real do teste produz quatro vínculos e 329 competências e
  chega ao motor como resultado experimental. Não há necessidade de excluir a
  fonte nem digitar vínculo e remuneração à mão.
- A preparação não inventa mais respostas da entrevista: se o benefício atual
  estiver sem resposta, ele continua pendente e a conferência mostra qual etapa
  corrigir. Antes, o adaptador gravava "não recebe benefício", "Simulação do
  benefício" e "Calcular um novo benefício" por conta própria.
- Períodos concomitantes deixaram de ser pendência decisiva. O motor soma apenas
  a parcela não sobreposta e avisa no resultado; o texto anterior afirmava que o
  motor bloqueava a sobreposição, o que não correspondia ao adaptador e travava
  o CNIS real, que tem vínculos paralelos.
- Ao escolher grau de deficiência, a etapa de dados avisa que a aposentadoria
  PcD pela LC 142/2013 ainda não tem motor neste adaptador. A execução bloqueia
  com motivo explícito, e a conferência e os relatórios continuam disponíveis.

### Verificação nativa com o binário publicado

- O executável publicado da 0.12.0, com SHA-256 conferido contra `SHA256SUMS.txt`, repetiu o fluxo com o CNIS real em perfil isolado (sem tocar na base do escritório): entrevista, importação, conferência, preparo e cálculo. O resultado foi experimental, com valor financeiro gerado para sexo masculino e sem deficiência declarada, e os avisos do motor apareceram sem corrupção de acentuação.
- O mesmo fluxo com deficiência grave bloqueia com o motivo da LC 142/2013, já avisado na etapa de dados.
- O instalador NSIS foi gerado a partir do mesmo commit e assinado pela chave do projeto; o `latest.json` publicado aponta para ele.

### Menores ainda abertas

As três pendências menores listadas acima continuam abertas, assim como `genpdf`
como dependência sem uso na exportação e os relatórios `Summary`, `Salaries` e
`Indicators` no template genérico.

## Recomendações originais (implementadas nesta rodada)

- **Etapa 5**: transformar as pendências num assistente passo a passo, com o
  botão "Preparar e calcular" em destaque e a explicação de que ele resolve as
  pendências de fonte/vínculo/remuneração.
- **Propagação**: garantir que as respostas da entrevista sobrevivam à
  aplicação do CNIS e apareçam confirmadas na conferência.
- **CNIS → motor**: deixar explícito, na própria conferência, que o CNIS é
  conferido e a hipótese é montada a partir dele em um clique.
