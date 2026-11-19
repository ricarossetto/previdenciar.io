# REA: bundles públicos do IEPREV — 28/10/2026

## Método e limites

O REA MCP 6.7.0 foi iniciado localmente e executou
`analyze_javascript_application` com `format: directory`, um arquivo de cada
vez, seguido de `inspect_analysis_view` para os candidatos de rota. Os arquivos
foram obtidos por requisição pública sem autenticação e guardados em diretório
temporário fora do repositório. Nenhum documento de cliente, cookie ou token foi
utilizado. A análise é sintática, não executa os bundles, e teve limites de
2.000 nós e 1.500 ocorrências desconhecidas por arquivo: a cobertura semântica
foi parcial e truncada. O resultado não demonstra disponibilidade de uma rota,
um fluxo completo, comunicação real ou fórmula do backend.

## Calc: arquivo principal

- Fonte pública: `https://calculos.ieprev.com.br/main.dcdce9140566b0d57194.bundle.js`
- SHA-256: `cb0fb5cc71972db3eb9d6898b590651001e2382dcc3b50d1af42c84af3752d48`.
- O REA leu 239.051 bytes, analisou um arquivo JavaScript e informou 56.845
  nós AST visitados, sem falha de análise sintática. Foram selecionados apenas
  os candidatos encontrados neste arquivo; outros bundles não foram analisados.
- A inspeção retornou 18 observações de endpoint, incluindo segmentos literais
  de rotas `importador-cnis`, `rgps`, `contagem-tempo`, `contribuicoes`,
  `beneficios`, `valores-em-atraso` e `transicao`. Um candidato de rede é
  `/user/login-info.json`. A hierarquia completa e o uso em execução não foram
  comprovados por esses segmentos isolados.
- O ID da evidência-pai nesta execução foi
  `ev_17bc3b6b5642bbe54c8e71fa4c869c3d8730044e7632f1c94f7177a488fcc755`.
  O identificador é referência da análise, não uma garantia de retenção futura.

### Módulo de importação CNIS

O mapeamento de módulos do bundle principal aponta o importador para o chunk 1;
o runtime público identifica seu arquivo como
`https://calculos.ieprev.com.br/1.b170f8cac4bdd6a52184.chunk.js`.
O arquivo tem 1.504.055 bytes e SHA-256
`ad577a283ef7e128d78fd742d7733d698f41e73984b6a2c9c2e4d15657bec4c2`.

Na inspeção textual estática desse chunk, o caminho de seleção de PDF chama
`processPdfFile`: ele abre o arquivo com PDF.js, percorre as páginas, chama
`getTextContent` e concatena os itens textuais; depois reúne os textos das
páginas antes de iniciar `newImportText`. Há código para reconhecer o
indicador `PREC-MENOR-MIN`. Essas são observações do código público, não de
uma importação executada. A concatenação observada não demonstra que outras
etapas preservem ou descartem a origem por página.

Não foi observado se o texto ou o PDF é transmitido, quais documentos são
aceitos, nem como imagens, tabelas difíceis e resultados são tratados. A
primeira tentativa de análise integral do chunk pelo REA excedeu 22 segundos;
uma segunda tentativa, com limites menores, excedeu 50 segundos. Para avançar,
extraiu-se apenas o trecho contínuo de `getPageText` e `processPdfFile`,
encapsulado em uma expressão sintaticamente válida, em arquivo temporário.
O fragmento tem 569 bytes e SHA-256
`55680a27b5b3a5ac38dd1aad8f918ef1bd09f94b3f1dd1d61aa26b2a4eb6ed34`.
O REA processou o fragmento sem erro de sintaxe (186 nós AST), com evidência
`ev_33df1810ce6e0b1e6774b023e19e17c77668451e190cd254406741759496bd94`.
Não retornou candidatos de endpoint nesse recorte; isso não demonstra ausência
de rede no restante da aplicação. A leitura textual das chamadas descritas
acima complementa essa análise e não é um grafo completo do chunk. O código
proprietário não foi incorporado ao projeto.

Em uma observação autenticada posterior, o DevTools da aba confirmou que esse
mesmo chunk foi carregado pelo Calc após seleção do CNIS autorizado. O
assistente exibiu períodos em tabela com empresa, datas, condição especial,
fator, carência, salários e ações de edição. Após a escolha explícita do sexo
informado pelo usuário, uma tentativa de continuar permaneceu na etapa de
conferência porque o campo data de filiação foi exigido. O CNIS autorizado não
apresentou o rótulo explícito de data de filiação na extração textual; a data
aguarda indicação do usuário. Não se deve tratar o primeiro vínculo importado
automaticamente como prova da data: o [INSS distingue a filiação do segurado
obrigatório da do facultativo](https://www.gov.br/inss/pt-br/direitos-e-deveres/inscricao-e-contribuicao/tipos-de-filiacao).
Por pedido do usuário foi inserida uma data **simulada** correspondente aos
18 anos, sem atribuir validade à filiação real. Ao optar por continuar sem
alterar salários ausentes ou abaixo do mínimo, o Calc exibiu o relatório de
tempo, com colunas de períodos, carência e concomitâncias. Na transição para
RMI, uma nova advertência ofereceu limitar salários ao mínimo ou voltar à
edição; nenhum ajuste foi aplicado e a RMI do legado permanece pendente. As
observações de linhas e parâmetros do caso ficam em área privada. O REA não
estava conectado diretamente ao navegador nessa observação.

## Pro: bundle inicial

- Fonte pública: `https://app.ieprev.com.br/_nuxt/CEtOGaG2.js`
- SHA-256: `69a98e9761918749727749962caaa6a8919ef1f3217941263ba767062e86ca01`.
- O REA leu 437.975 bytes, analisou um arquivo JavaScript e informou 136.639
  nós AST visitados, sem falha de análise sintática. Os módulos carregados
  separadamente não foram analisados nesta etapa.
- Foram observados literais de rotas para cálculo de benefício e planejamento,
  incluindo `/calculos/planejamento-previdenciario`, suas variantes de resultado
  e relatório, além de gestão, clientes, assinador e laudos. A presença dessas
  rotas no bundle não prova que estejam acessíveis ou concluídas para a conta.
- Alguns candidatos classificados automaticamente como rede eram nomes de
  cabeçalhos ou propriedades, não URLs; não foram tomados como APIs reais.
- Evidência-pai desta execução:
  `ev_6b7c24b19c1cf79138bd4cee54801b8802726c83393e8272c7bf1b9b6c3502bf`.

### Tela de cálculo do Pro identificada na sessão autenticada

Na página de novo cálculo, a árvore de recursos acessada pelo DevTools da aba
do Codex identificou `/_nuxt/BHE7DoT2.js` como arquivo que contém rótulos da
tela. Uma cópia do mesmo arquivo foi obtida por requisição pública sem cookies,
fora do repositório; SHA-256
`1c69d9354b11e98889d0a39228efb554a8ce7325c8c54fc1d9f1d049a018c87d`.
O REA MCP analisou essa cópia: 114.826 bytes, 32.919 nós AST e nenhuma falha
de sintaxe, com cobertura parcial; evidência
`ev_c0ba9c08dcadf8cd2aac48630208c70a6eb94fce34a5a9571d042a9f5a1cc5e6`.
Não identificou candidatos de endpoint nesse recorte. Isso não demonstra
ausência de comunicação, inclusive porque outros módulos não foram analisados.
O REA não estava diretamente conectado ao DevTools desta aba; o DevTools fez
a observação e o REA analisou uma cópia pública do recurso.

### Tela de resultado do Pro

A observação da aba localizou `/_nuxt/DdFVLKe_.js` na tela de resultado.
Uma cópia pública obtida sem cookies possui 43.920 bytes e SHA-256
`ede49c913be979be5acc4fcfd0d0af6bab5c1d19caad1cf7d2130132c8214e7f`.
O REA MCP examinou esse arquivo: 12.327 nós AST, sem erro sintático e com
cobertura parcial; evidência
`ev_56e2829bf231b20bc07858c0dec1acd61a8abfaca47bb5a883e984908c93b62f`.
Não foram identificados candidatos de endpoint no recorte, o que não prova
ausência de comunicação em outros módulos ou na execução.

Em uma execução autorizada com CNIS privado, o Pro apresentou advertência
para vínculos sem contribuição ou com valor zero, oferecendo ajuste ao salário
mínimo. A opção escolhida foi prosseguir sem esse ajuste. O serviço respondeu
à criação com HTTP 201 e a tela abriu um resultado; a análise não valida a
conferência dos dados, o enquadramento ou a RMI. O menu de exportação observado
ofereceu relatório completo, RMI e tempo de contribuição. As três opções
retornaram PDFs, copiados para diretório privado fora do repositório e
conferidos por extração de texto. O completo contém seções de tempo, renda e carência; essa
verificação não é revisão visual de todas as páginas nem validação da memória
jurídica ou financeira. Os dados do caso, documentos e valores não constam
nesta área pública.

## Impedimento operacional

A conexão de navegador deste chat foi encerrada. A tentativa de abrir o perfil
autenticado existente com CDP foi rejeitada pela política automática do
ambiente, sem motivo detalhado além de `blocked by policy`. Uma tentativa de
remover o perfil temporário pertencente ao ensaio anterior também foi
rejeitada. O diretório temporário continua preservado. Não se deve considerar
os resultados acima uma exploração autenticada ou comparação de cálculos.
