# CNIS local: leitura do documento autorizado

Este registro usa apenas contagens agregadas. Nome, CPF, NIT, empregadores,
competências e valores do documento privado **não** foram copiados para o
repositório.

## Entrada

- Documento: CNIS privado de 27 páginas, 595 x 842 pt, texto extraível.
- SHA-256: `41dd9dcbd09cedcbc22e48355845d1e0b8f9034b2cf26b023cc2aca6dfdd3b3b`.
- Parser: `pdf-extract 0.7.12 / lopdf 0.34.0`, local, em memória.
- A extração não é validação de direito, carência, RMI, PRPPS ou regime.

## Antes da correção

| Medida | Resultado |
|---|---:|
| Lançamentos de remuneração/contribuição | 542 |
| Meses de competência | 438 |
| Vínculos reconhecidos | 11 |
| Lançamentos sem vínculo | 112 |
| Cabeçalhos de relação ambíguos | 10 |
| Pares fora da tabela | 22 |
| Linhas com colunas não adjacentes | 4 |
| Valores consolidados sem vínculo | 3 |

O problema dominante era o descarte de relações cujo tipo de filiação não
estava na lista fechada (`CONTRIBUINTE EM DOBRO`, `RECOLHIMENTO`,
`AGRUPAMENTO DE CONTRATANTES/COOPERATIVAS` etc.). O parser deixava de definir
o vínculo corrente e os lançamentos seguintes ficavam órfãos.

## Depois da correção

| Medida | Resultado |
|---|---:|
| Lançamentos de remuneração/contribuição | 564 |
| Meses de competência | 460 |
| Vínculos reconhecidos | 21 |
| Lançamentos sem vínculo | 4 |
| Cabeçalhos de relação ambíguos | 0 |
| Pares fora da tabela | 0 |
| Linhas com colunas não adjacentes | 4 |
| Valores consolidados sem vínculo | 3 |

Os 4 lançamentos sem vínculo incluem 3 valores consolidados por ano civil,
que a origem não atribui a um empregador. O quarto e as 4 linhas não
adjacentes permanecem pendentes de conferência documental; não foram
atribuídos por heurística.

## O que mudou no código

- `crates/previus-cnis/src/lib.rs` passou a reconhecer `CONTRIBUINTE EM DOBRO`,
  `RECOLHIMENTO`, `AGRUPAMENTO DE CONTRATANTES/COOPERATIVAS`, `AUTÔNOMO` e
  `SEGURADO ESPECIAL` como tipos de filiação observáveis.
- Relações com tipo desconhecido não são mais descartadas: o vínculo é
  preservado com `affiliation: None` e o lançamento continua ligado à sequência.
- Dois testes sintéticos novos cobrem filiação não padronizada e preservação do
  vínculo. A suíte do crate passou para 38 testes.
- O resultado agora inclui o aviso `unknown_affiliation` quando uma relação é
  preservada sem tipo reconhecido; o dado não é descartado nem adivinhado.

## Comparação com a Fábrica

A chamada `rgps_tempo_ler_cnis` da Fábrica devolveu 23 períodos e 79 valores
consolidados para o mesmo PDF. Isso **não** é uma comparação de igualdade:
a Fábrica pode agrupar ou descartar linhas com semântica própria, e o parser
local preserva mais lançamentos individuais. O número não foi usado para
alterar o parser sem evidência.

## Limites

- O documento privado não foi adicionado ao repositório.
- O parser não valida direito, carência legal, PRPPS, concomitância, CTC ou RMI.
- 4 linhas não adjacentes ainda não são importadas; ficam como aviso.
- A importação do mesmo PDF nas plataformas IEPREV autenticadas foi executada
  posteriormente em sessão isolada; as observações estão em
  `2026-10-28-sessao-autenticada.md`. O Pro leu 23 períodos; o Calc leu 23
  períodos e acrescentou 3 linhas de benefício vazias.
- Próximo passo: abrir uma sessão de navegador isolada com a conta autorizada,
  importar o mesmo CNIS e comparar contagens, vínculos, avisos e parâmetros sem
  publicar dados pessoais.
