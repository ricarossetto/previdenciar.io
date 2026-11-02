# Contrato de equivalência e rejeição de opções incoerentes

Data: 02/11/2026. Complementa [TRF3_VALIDATION.md](TRF3_VALIDATION.md) e
[TRF3_VALIDATION_20261102.md](TRF3_VALIDATION_20261102.md). Não substitui nenhum dos dois
e não altera valores esperados.

## R1 — o que conta como caso para declarar equivalência

Correção anterior: `conferir_casos` exigia quatro **entradas** na lista. Quatro cópias dos
mesmos valores com rótulos diferentes declaravam equivalência. O rótulo era tratado como
prova de diversidade, o que não é.

Regra agora:

- A unidade de contagem é a **tupla de valores normalizada** (SB local, RMI local, SB oficial,
  RMI oficial), convertida pelo valor monetário padrão. `"800"` e `"800.00"` são a mesma tupla.
- O rótulo não entra na conta. Repetir valores com outro rótulo não cria caso, e o veredito
  do caso repetido registra isso.
- `MINIMO_CASOS_PARIDADE` continua em **4**. A exigência não foi reduzida nem removida.
- `ConferenciaResultado` passou a expor `casos_distintos` (tuplas distintas) e
  `casos_informados` (entradas na lista). A declaração usa `casos_distintos`.

Prova que a conferência **não** dá:

`ConferenciaCaso` guarda apenas os quatro valores já produzidos. Ele não guarda competências,
faixas de índice, série salarial, teto nem qualquer identificação de entrada. A conferência
não consegue, portanto, atestar cobertura de fronteiras de faixa de índice, série salarial não
uniforme, meses extremos ou teto. O texto do fundamento afirma isso explicitamente.

Consequência: a declaração vale **somente para o conjunto apresentado**. Quatro tuplas
distintas conferindo ao centavo não autorizam dizer que o motor foi validado em geral, e não
substituem casos de fronteira derivados de entradas reais.

Para cobrir fronteiras seria necessário acrescentar à estrutura a entrada de origem e conferir
a diversidade por ela. Isso é mudança de contrato público e não foi feita nesta frente.

## R2 — perfil de correção incoerente

Correção anterior: com `perfil_correcao` preenchido e `corrigir_salarios = false`,
`rgps_core::calcular` devolvia `Ok` e simplesmente não usava o perfil. O adaptador
(`previdenciar-rgps`) bloqueava com `profile_requires_correction`, mas a biblioteca pública
não. Quem chama o núcleo direto recebia um resultado nominal achando que o perfil tinha sido
aplicado.

Regra agora, em `validar()` — chamada por `calcular()` antes de qualquer apuração:

| Entrada | Antes | Agora |
| --- | --- | --- |
| perfil + `corrigir_salarios=false` | `Ok`, perfil ignorado | erro nomeando perfil e a opção |
| perfil desconhecido | falhava só no meio do cálculo | falha na validação, listando os disponíveis |
| `fallback_inpc_documentado=true` sem perfil | `Ok`, fallback inerte | erro pedindo o perfil |
| perfil em branco ou só espaços | tratado como ausente | continua tratado como ausente |

Números, fórmulas, coeficientes e o caminho nominal sem perfil não foram alterados. O teste
`perfil_em_branco_e_ausencia_e_nao_bloqueia` fixa que o caminho nominal segue devolvendo
R$ 1.600,00 no caso de 300 competências de R$ 2.000,00.

## Conflito conhecido, não resolvido aqui

`crates/rgps-core/tests/trf3_conferencia.rs`, linhas 148–159, monta quatro casos com os mesmos
valores (`SB_OFICIAL` / `RMI_OFICIAL`) e rótulos `sintetico-{i}`, e afirma
`conjunto.equivalencia_declarada`. Esse é exatamente o comportamento que R1 remove. Com a
correção, aquele teste falha.

O arquivo está fora do escopo autorizado desta frente (o teste de paridade ignorado mora nele e
só perde o `#[ignore]` depois da prova). A decisão fica para quem revisar: atualizar a
asserção para tuplas distintas reais, ou aceitar a falha como sinal de que a alegação antiga
era indevida.

## Estado

- Sem execução de `cargo` nesta frente: cache de integridade ativo.
- `crates/rgps-core/tests/ipcr_source.rs` (8 testes) e `conferencia_distincao.rs` (7 testes)
  seguem **não executados**.
- `paridade_centavo_trf3_falha_enquanto_a_serie_ipcr_nao_estiver_carregada` continua sendo o
  único `#[ignore]` do crate.
