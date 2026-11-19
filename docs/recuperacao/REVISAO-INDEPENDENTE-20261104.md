# Revisão independente — estado em 04/11/2026

Revisão somente leitura do branch `wip/consolidacao-20261102-correcoes` em `20134c4`, sem ler workspace, CNIS ou PDFs pessoais e sem executar build/testes.

## Achados altos que impedem publicação

1. **Base mista no fator pós-reforma**: o tempo total usa meses derivados de marcos em dias corridos, enquanto a fração de dias vem do total 30/360. É necessário derivar meses e dias da mesma base.
2. **30/360 não aditivo por vínculo**: a soma de períodos 30/360 separados pode diferir da união contínua (exemplo 58 vs 60 dias); a regra precisa ser aplicada à união efetiva, com testes de virada de mês/ano/bissexto.
3. **Carência e benefício/abaixo do mínimo**: o motor conta meses de benefício e competências abaixo do mínimo na carência; isso contraria as limitações declaradas do próprio aplicativo e precisa de regra jurídica explícita.
4. **Coluna Filantropia e `AmountKind`**: o caminho cooperativo grava remuneração fixa e não honra a coluna de contribuição quando existir; falta teste negativo.
5. **Reparo de texto persistido**: a normalização de mojibake no boot pode alterar a fonte CNIS e depois salvá-la; a fonte deve permanecer intacta ou o reparo deve ser versionado e reversível.
6. **Documento de publicação antigo**: apontava para branch/HEAD que não contêm as correções; corrigido neste mesmo diretório.

## Pontos médios relevantes

- Descontos com 30 dias fixos e `.max(0)` silencioso.
- Possível dupla subtração entre competências abaixo do mínimo e descartadas em chamada direta do núcleo.
- Relatório marca toda regra elegível como “Regra Escolhida” e inclui regras inelegíveis no ROI.
- Prompt do Copiloto ainda contém mojibake em `previdenciar-ai/src/agent.rs`.
- Testes escassos de `dias_30_360`, fator fracionário, projeção e CNIS negativo.

## Conclusão

O candidato não está pronto para publicação. O cálculo está próximo da referência, mas os achados acima precisam ser fechados e testados antes de homologar e congelar a versão.
