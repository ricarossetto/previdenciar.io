# Assistente RGPS: observação e melhorias de interface

Data: 23/10/2026. Referência de comportamento: aplicativo proprietário instalado, observado localmente com cenário fictício. Este documento não contém código recuperado, documentos, capturas ou dados de clientes.

## Escopo da observação

REA MCP 6.7.0 e Ghidra 12.1.4 analisaram o executável x64 com SHA-256 `70e97d5780aa2222b3231d3bc2ee4ecbf573f79462bd452d2ced11bed72d3a8e`. A navegação Windows foi feita separadamente com Computer Use, pois o observador nativo do REA desta instalação atende macOS.

Foi percorrido Novo cálculo → Utilizar o assistente → sem benefício atual → sem importação → simulação → novo benefício → nascimento e inscrição fictícios → sexo, DIB e espécie → painel interno. O aplicativo abriu diretamente o domínio de tempo de contribuição. As telas de dados iniciais conservaram as datas e escolhas da entrevista, com identificação pessoal vazia.

Foram abertos os domínios de tipo de cálculo, dados iniciais, dados complementares, dependentes, períodos, salários, complemento, agrupamento e as quatro abas de opções da RMI. Foram inspecionados os diálogos de inclusão de período, qualidade de segurado, inclusão de salário e opções avançadas de tempo. Os diálogos de inclusão foram fechados sem gravar linhas. O cenário de auditoria permanece no aplicativo; registros existentes não foram excluídos.

## Comportamentos observados e aplicação

| Observação no aplicativo de referência | Aplicação no previdenciar.io |
| --- | --- |
| A entrevista entrega o caso a editores por domínio, com menu persistente. | Painel do cálculo disponível na conferência, revisão e resultado, com acesso direto aos mesmos editores e preservação do rascunho. |
| Nascimento, DIB, espécie e sexo acompanham o caso após a entrevista. | Resumo permanente do cenário e atalho para editar os parâmetros. |
| Períodos possuem fonte, indicadores, carência, necessidade de prova, tipo e fator. | Pendências e origem continuam visíveis; abrir um domínio não equivale a confirmar seus registros. |
| Qualidade de segurado possui dados próprios por vínculo e eventos específicos. | Não presumir que o simples preenchimento da entrevista valida esse domínio. |
| Salários distinguem atividade principal, secundárias, pagamento e competência. | Separação explícita entre dados importados, ajustes do operador e entrada preparada; sem inferir concomitância ou salários ausentes. |
| Complemento e agrupamento são operações distintas, com parâmetros e ativação próprios. | Não apresentar controles decorativos ou prometer operações sem implementação e validação. |
| Opções da RMI separam planejamento, EC 103, Lei 9.876 e direito adquirido. | Manter opções e limites explícitos; nenhuma fórmula foi inferida da aparência dessas telas. |

O original apresentou escolhas previamente marcadas, incluindo ausência de deficiência e salário mínimo em meses sem salário. Isso é uma observação da interface, não justificativa para inferir respostas ou preencher lacunas em nosso sistema.

Ao acionar Gerar Relatórios sem períodos cadastrados, o original apresentou um aviso exigindo períodos para o fator previdenciário e indicando onde corrigir a entrada. Não foi produzido relatório nesse cenário. Essa interação reforça a utilidade de direcionar o operador ao domínio responsável pela pendência.

## Correções implementadas

- Etapas anteriores não recebem mais confirmação apenas pela posição na navegação. Respostas desconhecidas e resultados bloqueados continuam pendentes.
- O painel exibe contagens do rascunho e pendências, com destaque do domínio atual e navegação por teclado.
- Revisão final e resultado bloqueado oferecem atalhos de correção para o domínio correspondente.
- Entrada preparada, última execução bloqueada e resultado experimental possuem textos distintos. Dados prontos não são descritos como já utilizados pelo motor.
- A navegação entre resultado, períodos, remunerações e revisão preserva o cálculo e não executa nem salva automaticamente.

## Evidência estática e limites

O Ghidra encontrou a string `INFOEXE` em `0x1400487e8` e sete referências. A decompilação da função que contém `0x140002781` mostra carregamento de recurso e tratamento de uma lista WLL. Isso evidencia um carregador; não recupera o assistente, suas fórmulas ou o motor previdenciário. A busca por nomes associados a assistente/RGPS não encontrou funções nomeadas, e o estado de carregamento do PDB foi reportado como desconhecido.

Evidências REA: `ev_b09afd37e72e8df87ed8f8c79c733a8bcc3a182e2b76b8e0cccd8f15fa95b4a3`, `ev_14b7f76e61db62d1b56116b2a316296e1bd1a9641399692528a36dd543898f12`, `ev_d62d6343c5b189729db74aadf45fdd5f9cbd91d271f6c7dcd73323136e8f9d6b` e `ev_e7f3b0e7c868995908f1f195c3363710fd3f119f3ff410eaf39334fe77dfadbd`.

Não houve cobertura exaustiva de todas as combinações, operações em massa, divisão de vínculo, gráficos, cálculo preenchido com contribuições nem exportação nesta rodada. Não se afirma equivalência matemática, jurídica ou visual. O motor Rust, o renderer PDF, os contratos IPC e o armazenamento não foram alterados.

## Verificação desta alteração

Build TypeScript/Vite aprovado. Suíte de interface: 104 testes aprovados. Playwright focado: quatro cenários aprovados, incluindo conferência em 390 e 1440 pixels e central de relatórios. As capturas sintéticas foram inspecionadas visualmente. Esses testes usam IPC sintético e não certificam transporte Tauri, cálculo Rust, diálogos nativos ou exportação PDF real. Não foi produzido novo executável nativo nesta rodada.
