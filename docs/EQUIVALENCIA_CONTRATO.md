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

## Conflito resolvido em 02/11/2026 (commit seguinte)

`crates/rgps-core/tests/trf3_conferencia.rs` montava quatro casos com os mesmos valores
(`SB_OFICIAL` / `RMI_OFICIAL`) e rótulos `sintetico-{i}`, e afirmava
`conjunto.equivalencia_declarada`. Esse era exatamente o comportamento que R1 remove, e o teste
ficava vermelho.

Com autorização pontual, a asserção foi trocada para **recusar** equivalência por repetição: o
teste agora confere `casos_informados == 4`, `casos_distintos == 1` e
`!equivalencia_declarada`. Também renomeei o teste de
`sb_validado_independentemente_e_rmi_nao_nasce_do_sb` para
`sb_validado_independentemente_rmi_independente_e_repeticao_nao_declara_equivalencia`, porque a
intenção antiga não incluía repetição.

O trecho usa apenas valores que já estavam no arquivo e mede **deduplicação**, não cobertura de
fronteiras. Nenhum caso "oficial" novo foi inventado, e nenhum valor artificial foi apresentado
como prova de paridade. O teste `#[ignore]` de paridade, as âncoras de SB/RMI e a expectativa de
fallback `5068.27` foram preservados; o bloco do `#[ignore]` ficou byte a byte idêntico.

## Compatibilidade dos campos novos

`ConferenciaResultado` passou a serializar dois campos, `casos_distintos` e `casos_informados`.
A mudança é **aditiva**, não uma renomeação:

- nenhum campo existente foi renomeado, removido ou teve tipo alterado;
- nenhum consumidor do repositório lê `ConferenciaResultado` fora de `rgps-core` — as buscas em
  `crates/`, `apps/desktop`, `web/` e `scripts/` não encontraram uso;
- `conferencia` não é reexportado em `lib.rs`; o acesso é por `rgps_core::conferencia::*`.

O que isso **não** garante: consumidores externos ao repositório que desserializem
`ConferenciaResultado` com struct fechada passariam a falhar se recusarem campos desconhecidos.
Não afirmei contrato intocado; afirmei que a mudança é aditiva e que não há consumidor
conhecido neste repositório. A conferência não é selo de estabilidade de API.

## Estado

- Sem execução de `cargo` nesta frente: cache de integridade ativo.
- `crates/rgps-core/tests/ipcr_source.rs` (8 testes) e `conferencia_distincao.rs` (7 testes)
  seguem **não executados**.
- `paridade_centavo_trf3_falha_enquanto_a_serie_ipcr_nao_estiver_carregada` continua sendo o
  único `#[ignore]` do crate.
