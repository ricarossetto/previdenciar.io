# Registro de 02/11/2026 — insumo IPC-R encontrado (paridade ainda não executada)

Complementa [TRF3_VALIDATION.md](TRF3_VALIDATION.md), que permanece válido como registro de
31/10/2026. Nada do registro anterior foi reescrito.

## O que mudou

A série que faltava para a faixa IPC-R (07/1994–06/1995) do perfil `trf3-v1` foi localizada
na fonte pública do IBGE e versionada no repositório. Isso remove o bloqueio de **insumo**.
Não remove o bloqueio de **paridade**: o confronto com a API do TRF3 continua sem execução.

## Fonte

- Provedor: IBGE — API de Serviços v3.
- Agregado 86, **Índice de Preços ao Consumidor em Real (IPC-R)**. O nome oficial não tem
  o qualificador "restrito".
- Variável 103, "IPCR - Percentual no mês", unidade `%`.
- Classificação 72, categoria 2640, "Índice geral".
- Nível N1, localidade 1 (Brasil).
- Períodos: 199407 a 199506 (12 meses).
- Requisição: `GET https://servicodados.ibge.gov.br/api/v3/agregados/86/periodos/199407-199506/variaveis/103?localidades=N1%5Ball%5D&classificacao=72%5B2640%5D`
- Coleta: 2026-11-02T07:35:23Z, HTTP 200, `application/json; charset=utf-8`, corpo comprimido
  com gzip (296 bytes em trânsito, 495 bytes de conteúdo).
- O provedor **não** envia `ETag` nem `Last-Modified` e **não** declara versão do agregado.
  A chave de versão usada no dataset é nossa, derivada de agregado, variável, nível, categoria
  e data de coleta. Nenhuma versão foi inventada.

## Artefatos

| Arquivo | Tamanho | Papel |
| --- | ---: | --- |
| `crates/rgps-core/data/ipcr-ibge-86-20261102.json` | 495 B | corpo HTTP exato, sem reescrita |
| `crates/rgps-core/data/ipcr-ibge-86-20261102.meta.json` | 1.801 B | URL, método, data UTC, bytes, SHA-256, série bruta |
| `crates/rgps-core/data/ipcr.json` | 1.142 B | dataset no formato do loader (`serie`/`versao`/`fonte`/`valores`) |

SHA-256 do corpo bruto (e do conteúdo gravado):
`5CEFB1CABA2B0046A83848161E27A0D8C2F12B3AE3E3BE19B3846B2DBC025B98`.

## Unidade confirmada

A variável 103 é **percentual**, não fração. O loader trata `variacao` como percentual
(`perfil_correcao::fator_com_perfil` aplica `1 + taxa/100`), e o dataset foi gravado nessa
escala: `1994-07 = "6.08"` significa 6,08% no mês e produz fator 1,0608. A conversão é
`Decimal` a partir da string original, sem `float`.

## Valores (iguais aos do corpo bruto)

| Competência | Variação (%) |
| --- | ---: |
| 1994-07 | 6,08 |
| 1994-08 | 5,46 |
| 1994-09 | 1,51 |
| 1994-10 | 1,86 |
| 1994-11 | 3,27 |
| 1994-12 | 2,19 |
| 1995-01 | 1,67 |
| 1995-02 | 0,99 |
| 1995-03 | 1,41 |
| 1995-04 | 1,92 |
| 1995-05 | 2,57 |
| 1995-06 | 1,82 |

## O que está provado e o que não está

Provado (conferência estática, sem executar Rust):

- o dataset carrega no formato real do loader: `serie`, `versao`, `fonte`, `valores`;
- 12 competências contíguas, estritamente crescentes, de 1994-07 a 1995-06;
- toda variação é `Decimal` positivo, sem vírgula e sem `float`;
- os valores do dataset são idênticos aos do corpo bruto do IBGE, mês a mês;
- o hash do arquivo bruto confere com o registrado;
- o dataset não contém RMI, salário de benefício nem origem TRF3.

Não provado:

- **paridade com o TRF3.** O caso de 31/10/2026 esperava SB R$ 5.083,89 e RMI R$ 4.067,11.
  Com a IPC-R carregada, o esperado é que o SB se aproxime, mas isso só se sabe executando.
  Uma fixture igual não prova equivalência multi-caso.
- o comportamento do motor em fronteiras de faixa, série salarial não uniforme e meses
  extremos. `conferencia::MINIMO_CASOS_PARIDADE` continua em 4.
- qualquer valor de RMI. Nenhum resultado financeiro foi produzido nesta frente.

Os valores esperados de 31/10/2026 **não** foram ajustados para caber na série. Se o confronto
falhar, a apuração é a etapa seguinte.

## Teste preparado (não executado)

`crates/rgps-core/tests/ipcr_source.rs` cobre: carga pelo loader externo, 12 meses e ordem,
unidade percentual, recusa de mês fora da cobertura, igualdade com o corpo bruto, metadados,
ausência de RMI/origem TRF3 e resolução da faixa IPC-R sem fallback.

O teste inteiro de paridade em `crates/rgps-core/tests/trf3_conferencia.rs`
(`paridade_centavo_trf3_falha_enquanto_a_serie_ipcr_nao_estiver_carregada`) continua com
`#[ignore]`. Ele só deve perder a marcação depois da prova, e não nesta frente.

## Pendências

1. Executar `cargo test -p rgps-core --test ipcr_source` (Rust pendente nesta frente).
2. Rodar o teste de paridade hoje ignorado e apurar a diferença residual.
3. Ampliar para casos múltiplos antes de qualquer alegação de equivalência.
