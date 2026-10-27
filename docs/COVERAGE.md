# Cobertura do motor RGPS

Atualizado em 27/10/2026 (versão 0.16.0). Este documento diz o que o motor
calcula, com qual fundamento e o que ele recusa calcular. A regra do projeto é
simples: **nenhuma espécie é aproximada por outra** para produzir um número.

## Espécies atendidas

| Código | Espécie | Requisitos conferidos | Renda mensal inicial | Fundamento principal |
|---|---|---|---|---|
| 42 | Aposentadoria por tempo de contribuição | tempo apurado por vínculos; regras de transição da EC 103 e direito adquirido | coeficiente 60% + 2% por ano acima de 20/15 (regra geral) ou regra de transição | EC 103/2019, arts. 15 a 17 e 26; Lei 8.213/1991, arts. 25, II, e 29 |
| 41 | Aposentadoria por idade | idade (62 mulher / 65 homem; 60/55 rural) e carência de 180 contribuições; regra do art. 18 para quem já era filiado em 13/11/2019 | 60% + 2% por ano acima de 15 (mulher) / 20 (homem) | CF art. 201, § 7º, I e II; EC 103/2019, arts. 18 e 26, §§ 2º e 5º; Lei 8.213/1991, arts. 25, II, 48 e 50 |
| 32 | Aposentadoria por incapacidade permanente | carência de 12 contribuições (isenta em acidente, doença ocupacional ou doença da lista) e incapacidade **declarada** | 60% + 2% (regra comum), 100% se acidente/doença ocupacional; 100% na DER anterior à reforma | Lei 8.213/1991, arts. 25, I, 26, II, e 44; EC 103/2019, art. 26, §§ 2º, III, 3º, II, e 5º |
| 31 | Auxílio por incapacidade temporária | carência de 12 contribuições (mesmas isenções) e incapacidade **declarada** | 91% do salário de benefício, limitado à média dos últimos 12 salários | Lei 8.213/1991, arts. 25, I, 26, II, 29, § 10, e 61 |
| 46 | Aposentadoria especial | tempo especial **declarado** (15/20/25 anos) com PPP/enquadramento declarados; carência 180; idade 55/58/60 ou pontos 66/76/86 | 100% no direito adquirido; 60% + 2% na regra permanente e na transição por pontos | Lei 8.213/1991, arts. 25, II, 57 e 58; EC 103/2019, arts. 19, 21 e 26 |
| 57 | Aposentadoria de professor | magistério exclusivo **declarado** (25 mulher/30 homem); carência 180; idade 62/65, pontos 81/91 ou pedágio 100% | 100% no pedágio; 60% + 2% na permanente e nos pontos; direito adquirido 80% × fator não calculado | Lei 8.213/1991, arts. 25, II, 29 e 56; EC 103/2019, arts. 15, 18, 20 e 26 |
| LC 142 | Aposentadoria da pessoa com deficiência | módulo próprio, com grau, data de início da deficiência, projeções e premissas declaradas | 100% da média (grau constante) | LC 142/2013; cenário descrito em [LC142_SCENARIOS.md](LC142_SCENARIOS.md) |

## O que o motor não decide

- **Incapacidade (31/32)**: o motor calcula a renda. Não examina laudo, nexo,
  data de início da incapacidade (DII), qualidade de segurado ou cessação. A
  espécie escolhida é a declaração do operador; o relatório marca o requisito
  como "declarado, não verificado pelo motor".
- **Deficiência (LC 142)**: o grau e a data são premissas declaradas. O motor
  não certifica laudo nem continuidade da deficiência.
- **Tempo especial (46)**: o PPP, o enquadramento e a efetiva exposição são
  **declarados**; o motor calcula a renda e marca o requisito como "declarado, não
  verificado". Sem grau/tempo especial, a regra não fica elegível; nenhuma
  atividade comum é transformada em especial.
- **Professor (57)**: o tempo de magistério e a exclusividade são **declarados**;
  o motor calcula a renda e marca o requisito como "declarado, não verificado".
  O direito adquirido de 80% × fator não é calculado; quando essa é a única regra
  aplicável, o valor não é inventado e o relatório explica a pendência.
- **Pensão por morte (21), salário-maternidade (80), auxílio-acidente (36/94)**
  e demais prestações não são aposentadorias e não têm fórmula neste motor.
  O adaptador bloqueia com `especie_unsupported`.
- **Rural**: a regra rural só é aplicada quando **todos** os vínculos informados
  são rural/especial. Mistura de rural e urbano exige análise própria; o motor
  não presume a condição de segurado especial.
- **Incapacidade acidentária**: a origem (acidente, doença ocupacional, doença
  do trabalho, doença da lista) é uma opção declarada no editor
  (`Origem da incapacidade`). Sem essa declaração, aplica-se a regra comum de
  60% + 2% e o relatório avisa.

## Regras de transição da espécie 42

O comparador executa as regras da EC 103/2019 e escolhe a de maior RMI entre as
elegíveis:

- direito adquirido (tempo mínimo cumprido até 13/11/2019);
- pedágio de 50% (art. 17), com fator previdenciário;
- pedágio de 100% (art. 20), com 100% da média;
- pontos (art. 15), com 60% + 2%;
- idade progressiva (art. 16), com 60% + 2%;
- idade permanente (art. 18), com 60% + 2%.

O direito adquirido ainda não recalcula a RMI da regra antiga (80% maiores e
fator previdenciário/85-95): ele é exibido como elegível, mas sem valor. Essa é
uma limitação declarada, não um valor aproximado.

## Requisitos e transparência

O resultado traz `especie` e `beneficio.regras[]`, com cada requisito, o valor
exigido, o apurado, os meses faltantes e se o motor verificou o requisito. A
interface e o PDF mostram essa tabela. Um requisito não cumprido gera aviso
"simulação condicional, não concessão"; nenhum valor é apresentado como direito
adquirido.

## Conferência online opcional (Fábrica de Cálculos/TRF3)

O aplicativo pode comparar a RMI local com o cálculo do provedor oficial, sem
substituir o resultado local:

- **Endpoint fixo**: `POST https://fabrica-de-calculos.app.trf3.jus.br/rgps/api/v1/calculos-rmi`.
- **Autenticação**: cabeçalho `x-api-key`, com chave opcional informada em
  Configurações e guardada na base local cifrada. Escopo necessário: `rgps`.
- **Enviado**: data de nascimento, sexo, DIB, espécie, períodos e salários
  revisados. As descrições dos períodos são genéricas ("Período 1").
- **Não enviado**: nome, CPF, NIT, CNIS, PDF, endereço, anotações e a chave no
  corpo.
- **Retorno**: RMI do provedor, regra/fundamento, salário de benefício,
  coeficiente, requisitos e conclusão. O app guarda o resumo, a fonte, o
  horário e os hashes SHA-256 do pedido e da resposta; a resposta bruta não é
  gravada.
- **Validação em 26/10/2026** com casos sintéticos: 41 e 42 retornaram HTTP 200
  com RMI e regra; 31 e 32 retornaram HTTP 200 sem RMI apurada com o payload
  atual. Nesses dois casos o app mostra a resposta do provedor, mantém o motor
  local e não inventa valor.
- **PcD (LC 142)**: a conferência online não é oferecida; o módulo PcD continua
  separado, com premissas declaradas.
- **Limite**: o hash registra a resposta recebida, não é assinatura do provedor.
  O provedor pode alterar regras e reter a requisição conforme a política dele.

## Fontes conferidas

- Constituição Federal, art. 201, § 7º, I e II.
- EC 103/2019, arts. 16, 18, 26 (caput, §§ 2º, 3º e 5º).
- Lei 8.213/1991, arts. 25, 26, 29, 44, 50 e 61.
- Tema 1300 do STF (incapacidade permanente e EC 103), citado no aviso da
  espécie 32.

Os textos foram conferidos em 26/10/2026 nas páginas oficiais do Planalto e em
compilações legislativas; a memória do cálculo cita o dispositivo, não uma
interpretação livre.
