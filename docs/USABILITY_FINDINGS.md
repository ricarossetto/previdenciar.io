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

## Segunda rodada: 0.12.1 (teste completo como advogado sênior)

Método: binário nativo publicado, perfil isolado (sem abrir a base do escritório), CNIS real do caso com deficiência grave, roteiro do início ao PDF com captura de tela e texto de cada etapa.

### O que travava

1. **Identificação pessoal vazando no PDF.** A opção "Incluir identificação pessoal" estava desmarcada e o texto prometia nome e identificadores ocultos, mas o PDF mostrava nome e CPF do segurado. O caminho Typst nunca recebia essa opção.
2. **Cenário PcD sem saída.** O motor bloqueava com "LC 142/2013 ainda não está implementada neste motor local" e não havia botão, link ou instrução para o módulo LC 142, que existe no editor. Para o caso do teste, o programa terminava sem caminho.
3. **Prévia do PDF sem resposta visível.** O botão gerava o documento abaixo da dobra e nada mudava na região do clique. O texto de estado ainda mandava "escolher um cálculo" mesmo com o cálculo já selecionado.
4. **"Pronto para o motor" com execução bloqueada.** O painel de rastreabilidade dizia que a hipótese estava pronta enquanto o resultado dizia bloqueado.
5. **Botão de executar depois de já ter executado.** Depois do preparo em um clique, a tela ainda oferecia "Executar cálculo da hipótese", sem dizer que a última execução já tinha ocorrido.
6. **Etapa 3 com seis cartões e duas perguntas parecidas.** "O que este cálculo precisa resolver?" e "Qual cálculo deseja organizar?" pareciam a mesma pergunta, uma abaixo da outra.
7. **Jargão nos campos.** DIB, "sexo cadastral" e "deficiência na DIB" não tinham explicação curta.

### O que foi corrigido nesta versão

- O renderer Typst recebe a opção de identificação e oculta nome, CPF e a origem do vínculo quando ela está desmarcada.
- O resultado bloqueado pela LC 142 explica o motivo e oferece "Abrir o cenário PcD no editor", que cai na etapa "Cenário PcD (LC 142/2013)", com as premissas já abertas (antes o módulo ficava dentro de um acordeão fechado).
- A prévia rola até o documento gerado e o texto de estado segue o que está na tela.
- O selo virou "Revisado; execução bloqueada" e o botão virou "Executar novamente" quando já existe resultado.
- Etapa 3 mostra três finalidades principais e deixa as outras em "Outras finalidades"; a segunda pergunta virou "O que você quer obter?".
- DIB, sexo e deficiência ganharam ajuda curta.

### O que continua aberto

- O menu lateral tem 14 itens e três entradas parecidas (Cálculos, Cálculos do RGPS, Assistente de cálculos, Editor de dados).
- A central de relatórios concentra lote, título e três opções de apresentação na mesma tela; o caminho principal está no topo, mas a densidade continua alta para um novato.
- A tabela de remunerações lista 331 competências de uma vez.
- O aviso flutuante pode cobrir conteúdo no canto inferior direito.
- O módulo LC 142 pede várias declarações e evidências; é o preço de não inventar premissa, mas ainda é denso.

## Terceira rodada: 0.13.0 (redesign do fluxo)

O diagnóstico de UI era justo: a tela era um painel escuro com cartões, rótulos em caixa alta e blocos herdados que quebravam o contraste. A direção nova chama-se "mesa de trabalho": papel, tinta, uma pergunta por tela, opções em linhas e um único próximo passo.

O que mudou:

- Superfície de papel com tokens próprios (`flow-v2`), títulos em serifa e uma cor de ação. `PRODUCT.md` e `DESIGN.md` registram a verdade do produto e a direção.
- Passo a passo em linguagem simples, com progresso visível e sem caixa alta.
- Contexto do caso, pergunta, listas e painéis perderam o fundo escuro herdado; contraste e foco visível corrigidos.
- Resultado com faixa de estado e motivo em uma frase; bloqueio PcD traz a ação certa como principal.
- A troca de etapa começa no topo da tela.

O que continua fora do escopo desta rodada: a casca do aplicativo (menu lateral, cabeçalho, demais páginas), a tabela de 331 competências e a densidade da central de relatórios.

## Recomendações originais (implementadas nesta rodada)

- **Etapa 5**: transformar as pendências num assistente passo a passo, com o
  botão "Preparar e calcular" em destaque e a explicação de que ele resolve as
  pendências de fonte/vínculo/remuneração.
- **Propagação**: garantir que as respostas da entrevista sobrevivam à
  aplicação do CNIS e apareçam confirmadas na conferência.
- **CNIS → motor**: deixar explícito, na própria conferência, que o CNIS é
  conferido e a hipótese é montada a partir dele em um clique.

## Rodada 2 — cobertura de espécies (0.14.0)

Depois do teste PcD, o motor ainda só aceitava a espécie 42. A rodada de
26/10/2026 atacou esse beco sem saída.

### Fricções encontradas

1. **Uma única espécie no seletor.** Quem tinha um caso de idade, invalidez ou
   auxílio-doença não conseguia nem simular: o adaptador bloqueava com
   "espécie não suportada". O seletor da entrevista também não perguntava a
   espécie.
2. **Resultado sem requisitos.** O número aparecia, mas não dizia idade,
   carência, tempo, origem da incapacidade ou o que faltava. Para um advogado,
   um valor sem requisito não serve para orientar o cliente.
3. **Incapacidade confundida com deficiência.** O campo "Condição de
   deficiência" parecia o lugar de declarar invalidez. Marcar "Grave" levava ao
   bloqueio da LC 142, não ao cálculo da espécie 32.
4. **Origem acidentária inacessível.** Sem um campo de origem, a regra de 100%
   da incapacidade permanente acidentária nunca podia ser aplicada.
5. **Regra técnica no resultado.** O identificador `idade-permanente` aparecia
   no lugar do nome da regra.

### O que mudou

- Seletor com 42, 41, 32 e 31 na entrevista e no editor. A 46 e a 57 continuam
  bloqueadas, mas com motivo específico (PPP/enquadramento e magistério).
- Tabela de requisitos no resultado e no PDF, com exigido, apurado, meses
  faltantes e a marca "declarado, não verificado pelo motor".
- Aviso curto separando incapacidade (31/32) de deficiência (LC 142), e campo
  "Origem da incapacidade" no editor de política.
- Nome legível da regra no resultado; o identificador continua na trilha
  técnica.
- Nenhum dado real foi usado nesta rodada: os casos são sintéticos. O CNIS real
  da rodada anterior continua fora do repositório.

### O que continua em aberto

- Direito adquirido da espécie 42 sem recálculo da RMI antiga.
- 46 e 57 sem motor próprio.
- A perícia, o nexo e a DII continuam declarados, não verificados.

## Rodada 3 — conferência online na Fábrica de Cálculos (0.15.0)

O usuário forneceu a chave da API e pediu para usar a Fábrica de Cálculos/TRF3
dentro do sistema, lendo toda a documentação.

### O que foi lido

- O portal VitePress em `/docs-tec/` e o `hashmap.json` com as 54 páginas.
- A spec unificada em `/docs-api/_openapi.json` (v1.5.20, 47 rotas): `calculos-tempo`,
  `calculos-rmi`, `calculos-rmi/por-fundamento`, `calculos-atrasados`,
  `documentos/*`, `fundamentos`, `marcos-temporais` e `indices/*`.
- Autenticação: `x-api-key` nas rotas protegidas; `GET` de fundamentos e marcos
  é público; datas em ISO (`AAAA-MM-DD`/`AAAA-MM`).
- Contrato do RMI: `dataApuracao`, `dadosSegurado`, `opcoesContagem`,
  `periodosContribuicao`; resposta com `maiorRmi`, `beneficiosMaisVantajosos`,
  `dadosRmi`, requisitos e conclusão.

### O que foi integrado

- Botão de conferência no resultado, com confirmação e resumo do envio.
- Chave em Configurações, cifrada na base local, enviada só no `x-api-key`.
- Whitelist de payload no Rust; sem nome, CPF, NIT, CNIS, PDF ou anotações.
- Resumo, fonte, horário e hashes no caso; seção no PDF detalhado.
- Testes de payload, parser, cliente HTTP local e interface.

### O que a API respondeu

- 41 e 42: HTTP 200 com RMI e regra.
- 31 e 32: HTTP 200 sem RMI apurada com o payload atual; o app mostra a
  resposta e mantém o motor local.

### O que continua em aberto

- Descobrir o campo que falta para o provedor apurar 31/32.
- Avaliar `por-fundamento` para recalcular a RMI antiga do direito adquirido.
- A conferência não substitui o motor local, a perícia ou a revisão.

## Rodada atual — 0.14.0/0.15.0 (27/10/2026)

Caso: o mesmo CNIS real de cliente, mantido fora do repositório, com deficiência
grave e aposentadoria por tempo de contribuição. Persona: advogado sênior,
~70 anos, pouca familiaridade com computador. Método: binário 0.14.0 em perfil
isolado, Playwright sobre o WebView2 real (CDP), 17 telas do assistente e, em
seguida, o cenário PcD preenchido até a tentativa de execução, que revelou a
trava descrita abaixo. Dados temporários apagados no fim;
nenhum CNIS, nome, CPF ou print de cliente entrou no repositório.

### Resultado por etapa

- Abertura, escolha do caso, importação do CNIS e aplicação: **sem trava**.
  O CNIS real foi lido (4 vínculos, 357 lançamentos, 331 competências, 7 avisos).
- Sexo, deficiência grave, espécie, conferência e resultado: **sem trava**.
  A conferência tem um único botão de preparo e o resultado bloqueado é
  explicado; nenhum valor foi inventado.
- O ponto de ruptura apareceu **depois** do resultado PcD, no editor: a revisão
  do assistente e o livro de contribuições.

### Achados e correções desta rodada

1. **Crítico — o cenário PcD não podia ser executado pelo caminho do assistente.**
   O botão **Abrir o cenário PcD no editor** abria o editor, mas qualquer
   alteração de premissa invalidava a revisão do assistente. O editor não tinha
   onde reconfirmar; o botão **Executar motor RGPS** devolvia “Complete as
   perguntas... confirme a revisão do assistente” e o usuário ficava sem saída.
   **Corrigido**: a etapa **Revisão** do editor ganhou o checkbox de confirmação,
   a etapa **Resultado** mostra a pendência e o atalho **Ir para revisão**, e um
   teste de interface cobre o caminho.
2. **Maior — aviso contraditório.** A etapa de parâmetros dizia que a LC 142
   “ainda não tem motor neste adaptador”, embora o aplicativo tenha o módulo
   PcD. **Corrigido**: o aviso agora diz que o motor RGPS não calcula PcD, que o
   módulo é próprio e que o botão do resultado o abre.
3. **Maior — labirinto do PcD.** O usuário precisa passar pela conferência do
   RGPS (que prepara vínculos e salários), receber o bloqueio esperado e só então
   abrir o PcD. Não é um cálculo RGPS válido; é a preparação. **Mitigado** com o
   aviso que nomeia o botão e o motivo do bloqueio; o atalho para o PcD sem
   preparar os dados produziria uma hipótese vazia, então não foi criado.
4. **Menor — status e DIB do PcD.** A etapa mandava para uma “conferência” que
   não existe no editor e a DIB apontava para uma tela com outro nome.
   **Corrigido**: o status aponta **Revisão** e **Resultado**; a DIB aponta
   **Segurado e cenário**.
5. **Menor — data-base técnica em branco.** O campo “Data-base dos valores e da
   projeção” exige um mês que o advogado pode não saber escolher.
   **Corrigido**: botão **Usar o mês anterior à DIB**, com confirmação explícita
   antes de consultar as tabelas.
6. **Menor — menu denso.** A lateral reúne *Cálculo previdenciário*, *Cálculos
   do RGPS*, *Assistente de cálculos* e *Editor de dados*. O caminho guiado é o
   Assistente; os outros competem com ele. **Recomendação**: renomear para
   *Assistente (passo a passo)* e *Editor avançado* em uma rodada com atualização
   dos testes. Não foi alterado agora para não quebrar seletores e o histórico.
7. **Menor — tabela de 331 competências.** Continua densa; o filtro e a
   conferência por vínculo ajudam, mas não foi redesenhada nesta rodada.
8. **Crítico — livro de contribuições no PcD.** O preparo do assistente cria
   salários manuais; o motor PcD exige o livro revisado e bloqueia com
   "Salários manuais exigem um livro de contribuições aplicado e rastreável".
   **Corrigido**: a etapa Parâmetros avisa, explica que os salários não são
   usados sem revisão e abre o livro; o usuário confere e aplica a revisão antes
   de executar. O caminho resultado bloqueado → editor PcD → livro → revisão →
   execução foi coberto por teste de interface.
9. **Crítico — 331 lançamentos para conferir no livro.** A conferência manual do
   livro exigia confirmar, justificar e declarar BRL em cada um dos 331
   lançamentos antes de o PcD executar. Para o advogado de 70 anos isso é um
   labirinto inviável. **Corrigido**: a etapa Parâmetros passou a oferecer
   **Usar os salários do CNIS no PcD**; o motor usa a fonte importada, que é
   traceável, e o livro fica disponível apenas para quem quiser ajustar os
   salários manualmente.

### O que foi verificado no código

- O gate do backend exige respostas da entrevista e `review_confirmed`; não é
  contornado por etapa ou por clique no editor.
- O PcD continua usando as premissas declaradas, os 80% maiores salários e o
  coeficiente de 100%; nenhum valor do RGPS é reaproveitado como PcD.
- O PDF do resultado bloqueado não é tratado como resultado financeiro; o PDF
  do PcD é gerado somente depois da execução do módulo.
- A correção do PcD foi coberta por `pcd-editor-review.test.tsx`; a suíte de
  interface fechou em 120 testes e os 36 testes de navegador continuam verdes.

### Validação final no binário 0.15.2

- O PcD real, com o CNIS importado e deficiência grave, executou usando
  **Usar os salários do CNIS no PcD**: RMI de **R$ 6.987,37**, sem livro manual e
  sem conferir os 331 lançamentos. O resultado continua marcado como simulação.
- O caminho do livro revisado permanece disponível, mas deixou de ser obrigatório
  para quem quer usar a fonte importada.
- A prévia do PDF do PcD não aparece no editor manual; o relatório fica na página
  **Relatórios**. Para o usuário idoso, isso ainda é um desvio: o próximo ajuste
  é levar a prévia/exportação para o próprio resultado do PcD.


