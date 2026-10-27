# PRODUCT.md

## Produto

previdenciar.io é um aplicativo desktop local (Tauri + React + Rust) para organizar CNIS, montar hipóteses de cálculo previdenciário e gerar relatórios em PDF. Não é serviço oficial do INSS, não certifica direito e não substitui revisão profissional.

## Quem usa

Advogados e profissionais do Direito Previdenciário, muitos entre 50 e 75 anos, com pouca familiaridade com software. Trabalham sozinhos, leem devagar, usam óculos, erram cliques e desistem quando não sabem o próximo passo. O caso típico tem um CNIS longo (20 a 30 páginas) e um cliente com deficiência ou tempo rural.

## Trabalho a ser feito

Receber um CNIS real, conferir o que o documento diz, decidir parâmetros com honestidade e chegar a um relatório que possa ser lido e explicado ao cliente.

## Promessas

- Dado ausente não vira zero, e pendência não vira resultado.
- O CNIS não é alterado; a hipótese é uma cópia revisada.
- Nenhum valor, sexo, deficiência ou data é inferido.
- Cálculo, importação e PDF acontecem na máquina do operador.
- O resultado bloqueado continua bloqueado, com motivo legível.

## Restrições

- O motor RGPS cobre as espécies 42 (tempo de contribuição), 41 (idade), 32 (incapacidade permanente) e 31 (incapacidade temporária); a 46 (especial) e a 57 (professor) bloqueiam com motivo.
- A LC 142/2013 existe como simulação com premissas declaradas em módulo próprio.
- O aplicativo não tem backend web, conta ou telemetria.

## Anti-metas

- Parecer um painel corporativo ou um ERP.
- Esconder pendências para o fluxo "passar".
- Encher a tela de opções técnicas simultâneas.
- Prometer concessão, valor garantido ou conformidade jurídica.
