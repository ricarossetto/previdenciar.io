# Divergências do caso real — DIB 30/06/2023 (sanitizado)

## Referência Previus (relatórios de 24/07/2026)

| Item | Valor |
|---|---:|
| Parcelas no PBC | 327 |
| Descartes automáticos | 17 |
| Competências consideradas | 310 |
| Soma dos 310 salários | R$ 968.829,96 |
| Média corrigida | R$ 3.125,26 |
| Fator previdenciário | 0,7299 |
| RMI | R$ 2.281,13 |
| Tempo de contribuição na DIB | 38a 1m 10d (344 contribuições) |
| Tempo em 13/11/2019 | 34a 5m 23d |
| Tempo em 13/11/2019 após os 17 descartes | 33a 0m 23d |
| Competências sem salário | 12 (09/2018 a 08/2019, auxílio-doença) |

A referência aplica a regra do pedágio de 50% **depois** do descarte automático, com recálculo do tempo. A RMI é a média corrigida multiplicada pelo fator previdenciário.

## Motor atual com a preparação completa e o perfil `trf3-v1`

| Item | Referência | Motor | Diferença |
|---|---:|---:|---:|
| Entrada de competências | 333 | 345 (333 do CNIS + 12 do auxílio no mínimo) | +12 documentadas |
| Abaixo do mínimo fora do PBC/tempo | 18 | 18 | 0 |
| Parcelas no PBC | 327 | 327 | 0 |
| Descartes automáticos | 17 | 17 | 0 |
| Competências consideradas | 310 | 310 | 0 |
| Soma dos 310 salários | R$ 968.829,96 | R$ 952.227,00 | −1,71% |
| Média corrigida | R$ 3.125,26 | R$ 3.071,70 | −1,71% |
| Fator previdenciário | 0,7299 | 0,7398 | +1,36% |
| RMI (pedágio de 50%) | R$ 2.281,13 | R$ 2.272,69 | −R$ 8,44 (−0,37%) |
| Tempo total após os 17 descartes | 36a 8m 10d | 36a 9m 4d | +24 dias |
| Tempo até a reforma após os 17 descartes | 33a 0m 23d | 33a 6m 29d | +6m 6d |
| Coeficiente da regra | 100% | 100% | 0 |
| Competências sem salário | 12 | 12 (vínculo e salários documentados no caso) | 0 |

- O motor agora inclui o período de aluno aprendiz de 19/02/1982 a 28/12/1984 (2a 10m 10d), as 12 competências de auxílio-doença de 09/2018 a 08/2019 no salário mínimo e o período de 1982–1984 como vínculo manual documentado.
- A regra escolhida é a **transição por pedágio de 50%**, elegível. A RMI exibida no resumo, na interface e no relatório é a da regra escolhida; a RMI bruta da regra base (2.150,19) fica preservada na memória para auditoria.
- A RMI de 2.272,69 é calculada, não fixada. A diferença de R$ 8,44 para a referência vem da série de correção e do tempo, não de uma regra ausente.

## O que foi implementado e validado

1. **Descarte automático da EC 103, art. 26, §6º.** O adaptador deixou de bloquear `descartarMenoresMinimo`. O motor exclui as menores remunerações corrigidas até o mínimo declarado e recalcula tempo, carência, quantidade de competências, coeficiente e regras de transição. O caminho INPC e o caminho com perfil de correção têm a mesma semântica.
2. **Meses abaixo do mínimo fora do tempo.** O adaptador já os excluía do PBC; agora também os passa ao motor, que reduz tempo e carência. Na DIB, 18 competências ficaram fora do PBC e do tempo.
3. **Perfil `trf3-v1` com a série IPC-R.** O motor registra a série IPC-R versionada (`data/ipcr.json`) no catálogo antes de aplicar o perfil. O perfil e o descarte mínimo funcionam juntos.
4. **RMI da regra escolhida promovida.** O resumo passou a exibir a RMI da regra vencedora, com coeficiente 100% para as regras de fator/100% da média, e o nome da regra. A RMI bruta/base continua na memória.
5. **Caso real completo salvo no workspace local.** Revisão 4, dois casos, 23 vínculos e 345 competências no caso real. A nota do caso registra a preparação e a diferença residual. A interface mostra a RMI 2.272,69 e a regra “Transição por pedágio de 50%”.

## Testes e binário

- `cargo test -p rgps-core`: 73 testes unitários, 11 âncoras, 7 de distinção, 8 do insumo IPC-R, 3 de limites do modelo e 6 de conferência TRF3 (1 ignorado conhecido) — todos passam.
- `cargo test -p previdenciar-rgps`: 7 testes de fábrica, 72 do adaptador, 6 de contribuições, 10 do fluxo guiado, 21 do PCD, 1 de cobertura e 2 de edição de vínculo — todos passam.
- Binário do servidor: `previdenciar-web-server.exe`, SHA-256 `F72CF5CD1D641806CF388938520632D3906D1CC4A1CDD7778BAB72978BCA7C34`.
- Instância local descartável: `http://127.0.0.1:8788/`, processo 3896, contêiner `previdenciar_test_a67ec8e86ef9`.

## Diferenças residuais, sem forçar a fórmula

1. **Série de correção.** Em 08/1994 o motor corrige 74,40 para 728,35 (índice 9,789682); a referência corrige para 738,81 (índice 9,930221). A diferença de 1,42% se espalha pelas competências antigas e explica a média 1,71% menor. O perfil `trf3-v1` é a série oficial versionada mais próxima, mas não é idêntico à série “Reajuste dos Salários de Contribuição” do Previus.
2. **Tempo até a reforma.** O motor apura 33a 6m 29d após os descartes, contra 33a 0m 23d da referência (+6m 6d). A diferença está no tratamento dos três períodos concomitantes e na conversão 365/30; é a causa do fator 0,7398 contra 0,7299.
3. **Datas do benefício no parser.** O `BenefitRecord` ainda não guarda início/fim do auxílio-doença. Neste caso as 12 competências foram incluídas como vínculo e salários documentados no próprio caso. Para os próximos casos, o parser precisa preservar as datas para que a preparação seja automática.
4. **Formulário de política.** O caso salvo contém `descartarMenoresMinimo=310`, `perfilCorrecao=trf3-v1` e `expectativaSobrevida=26.1`. O formulário da interface não expõe todos esses campos; a política salva é preservada e usada no cálculo, mas a edição de novos casos ainda depende do JSON da política.

Os erros de média (−1,71%) e de fator (+1,36%) se compensam em parte; a RMI final fica 0,37% abaixo da referência. Nenhuma fórmula foi alterada para produzir o valor esperado. Nenhum dado pessoal foi incluído neste documento, em commits ou em serviços externos.
