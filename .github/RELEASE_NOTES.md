## previdenciar.io 0.16.0

As seis aposentadorias do RGPS agora têm motor: 42, 41, 32, 31, 46 e 57.

### O que passou a funcionar

- **46 — especial**: informe o grau/agente (15, 20 ou 25 anos), o tempo especial,
  a referência do PPP e se o tempo foi completo até 13/11/2019. O motor devolve
  direito adquirido de 100%, regra permanente de 55/58/60 + 60% + 2% e transição
  por pontos 66/76/86.
- **57 — professor**: informe o tempo de magistério, a exclusividade e se o tempo
  foi completo até 13/11/2019. O motor devolve pedágio de 100% (52/55 + 25/30),
  pontos 81/91 e regra permanente de 62/65 + 25/30 + 60% + 2%.
- O editor e o seletor mostram as seis espécies; a tabela de requisitos aparece na
  tela e no PDF.

### O que continua declarado

- PPP, enquadramento e magistério exclusivo **não** são verificados pelo app; sem
  o dado declarado, a regra não fica elegível. Nenhuma atividade comum é
  transformada em especial.
- O direito adquirido do professor (80% × fator) não é calculado; o app explica e
  não inventa valor.
- 46/57 não são enviados à conferência online.

### Validação

- 509 testes Rust, 120 de interface e 36 de navegador.
- Nenhum CNIS, nome, CPF, PDF ou chave entra no repositório ou no pacote.

## previdenciar.io 0.15.2

O cenário PcD com o CNIS importado deixou de exigir a conferência manual de 331
lançamentos. A hipótese pode usar a fonte importada diretamente, que continua
traceável, ou o livro revisado para quem quiser ajustar os salários.

### O que mudou

- A etapa **Parâmetros** do PcD mostra **Usar os salários do CNIS no PcD** quando
  há salários manuais sem livro aplicado.
- O motor usa a fonte importada; nenhum salário é inventado e o resultado
  continua marcado como simulação.
- O caminho do livro revisado permanece disponível, com a justificativa e a
  aplicação explícitas.

### Validação

- 506 testes Rust, 120 de interface e 36 de navegador.
- Nenhum CNIS, nome, CPF, PDF ou chave entra no repositório ou no pacote.

## previdenciar.io 0.15.1

O caminho PcD do assistente agora termina no cálculo. O editor avisa quando os
salários manuais ainda não têm o livro de contribuições aplicado e leva ao livro;
sem essa revisão o motor não usa os salários, por desenho.

### O que mudou

- A etapa **Parâmetros** do PcD mostra o aviso e o botão **Abrir o livro de
  contribuições** quando há salários manuais sem livro aplicado.
- O usuário confere o livro e clica em **Aplicar revisão ao cálculo**; o aviso
  desaparece e a execução do PcD fica disponível.
- O fluxo resultado bloqueado → editor PcD → livro → revisão → execução foi
  coberto por teste de interface.

### Validação

- 506 testes Rust, 119 de interface e 36 de navegador.
- Nenhum CNIS, nome, CPF, PDF ou chave entra no repositório ou no pacote.

## previdenciar.io 0.15.0

Agora o aplicativo pode comparar a RMI do motor local com o cálculo da Fábrica
de Cálculos/TRF3. É uma conferência opcional, com chave própria e envio mínimo:
o resultado local continua sendo o resultado do aplicativo.

### Como funciona

- Em **Configurações → Fábrica de Cálculos**, informe a chave da API (campo de
  senha). Ela fica na base local cifrada e só é usada no botão de conferência.
- No resultado, clique em **Conferir na Fábrica de Cálculos**. Um diálogo mostra
  exatamente o que sai: data de nascimento, sexo, DIB, espécie, períodos e
  salários revisados.
- Nome, CPF, NIT, CNIS, PDF, endereço e anotações **não** são enviados. A chave
  vai apenas no cabeçalho `x-api-key`; não entra no corpo, no PDF ou nos logs.
- O resumo do provedor mostra RMI, regra/fundamento, salário de benefício,
  coeficiente, requisitos, conclusão, fonte, horário e hashes. Quando a correção
  local está ativa, o app compara os valores em centavos.
- O PDF detalhado imprime a seção de conferência quando ela foi feita.

### Cenário PcD

- O aviso agora explica que a LC 142 tem um módulo próprio no editor; não diz
  mais que a aposentadoria PcD "não tem motor".
- Depois de alterar as premissas do PcD, a etapa **Revisão** do editor permite
  confirmar a revisão do assistente; a execução deixa de falhar com
  "Complete as perguntas..." e oferece o atalho **Ir para revisão**.
- A conferência continua preparando vínculos e salários; o resultado bloqueado
  do RGPS leva ao botão **Abrir o cenário PcD no editor**, sem valor inventado.

### O que foi validado

- Documentação técnica da Fábrica lida por inteiro: 47 rotas na spec unificada
  v1.5.20, `calculos-rmi`, `fundamentos`, `marcos-temporais`, datas ISO e
  autenticação `x-api-key`.
- Caso sintético de 300 competências: espécies 41 e 42 retornaram HTTP 200 com
  RMI e regra. As espécies 31 e 32 retornaram HTTP 200 **sem RMI apurada** com
  o payload atual; o app mostra a resposta do provedor, mantém o motor local e
  não inventa valor.
- 506 testes Rust, 118 de interface e 36 de navegador.

### Limites

- A conferência de 31/32 não apurou RMI no teste; PcD (LC 142) não é enviada.
- O hash registra a resposta recebida, não é assinatura do provedor. A retenção
  do lado da Fábrica segue a política dela.
- A conferência não substitui revisão documental, perícia, nexo, DII ou
  conferência de CTC. Sem chave, o aplicativo funciona offline como antes.
- A distribuição não tem assinatura de código Windows; confira o SHA-256.

## previdenciar.io 0.14.0

O motor deixou de atender só a espécie 42. Aposentadoria por idade (41),
incapacidade permanente (32) e auxílio por incapacidade temporária (31) agora
têm fórmula, requisitos e base legal próprios. O que não tem regra implementada
continua bloqueado, com motivo — nunca aproximado.

### O que passou a funcionar

- **41 — Aposentadoria por idade**: 62/65 anos (60/55 rural), 180 contribuições,
  regra permanente e regra do art. 18 da EC 103 para quem já era filiado em
  13/11/2019. Coeficiente de 60% + 2% por ano acima de 15/20.
- **32 — Incapacidade permanente**: carência de 12 ou isenção por acidente,
  doença ocupacional ou doença da lista; 60% + 2% na regra comum e 100% na
  origem acidentária. A DER anterior à reforma usa o art. 44 e avisa sobre a DII
  e o Tema 1300 do STF.
- **31 — Incapacidade temporária**: 91% do salário de benefício, limitado à
  média dos últimos 12 salários (art. 29, § 10).
- **Requisitos na tela e no PDF**: idade, carência, tempo, incapacidade e
  origem aparecem com valor exigido, apurado, meses faltantes e a marca
  "declarado, não verificado pelo motor" quando é o caso.
- **Escolha da espécie** no editor e na entrevista guiada, com a origem da
  incapacidade no editor de política.

### O que continua bloqueado, de propósito

- **46 — Especial**: exige PPP, enquadramento e 15/20/25 anos de exposição.
- **57 — Professor**: exige comprovação exclusiva de magistério.
- Pensão por morte, salário-maternidade, auxílio-acidente e demais prestações
  não são aposentadorias e não têm fórmula neste motor.
- A incapacidade é declarada. O programa calcula a renda; não faz perícia nem
  certifica nexo, DII ou qualidade de segurado.

### Também nesta versão

- A matriz completa está em `docs/COVERAGE.md`.
- O resumo do PDF mostra a espécie e a regra escolhida; a memória mostra cada
  requisito.
- 495 testes Rust, 111 de interface e 36 de navegador.

### Limites

- O direito adquirido da espécie 42 continua sem recálculo da RMI antiga (80%
  maiores e fator/85-95); ele aparece como elegível, sem valor.
- O módulo PcD da LC 142/2013 continua separado e com premissas declaradas.
- A distribuição não tem assinatura de código; confira o SHA-256.
