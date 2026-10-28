# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

delegated: static HTML/CSS/JS, no build step, so the mock opens from disk in a browser. An optional local Node proxy (no dependencies) serves the same files and forwards explicit Fábrica de Cálculos requests with the API key kept server-side. The desktop app (React + Tauri + Rust) is the product reference, not a dependency.

## Users

Advogados e consultores previdenciários que organizam CNIS, testam hipóteses de aposentadoria e produzem relatórios para clientes. Trabalham sozinhos ou em escritórios pequenos, no Windows, alternando entre documentos oficiais, cálculo e conferência manual. A versão web é uma demonstração navegável do produto para esse mesmo profissional.

## Product Purpose

Tornar o caminho entre o CNIS e uma hipótese previdenciária conferível: importar e organizar vínculos e contribuições, declarar premissas, calcular e levar a memória junto em relatórios. Sucesso é o profissional conseguir explicar cada número a partir da origem, e não apenas ver um resultado.

## Positioning

Origem antes do resultado. Cada valor mostra de onde veio, cada dado ausente permanece pendente, e o cálculo não inventa o que o documento não informa. O produto é local por padrão e declara seus limites em vez de prometer cobertura total.

## Operating Context

O trabalho começa com um CNIS em PDF e termina em um relatório revisado. No meio: conferência de vínculos, livro de contribuições, escolha de cenário (RGPS, PcD), premissas explícitas, memória de cálculo e uma central de relatórios. O mock cobre esse fluxo com dados fictícios e sem executar o motor Rust. A área Fábrica permite comparar o motor local com o serviço oficial e usar os leitores de documentos por um proxy local.

## Capabilities and Constraints

- Versão web navegável das 15 áreas do app: Início, Clientes, Cálculos, RGPS, Motor local (WASM), **Fábrica de Cálculos**, Assistente, Editor de dados, Tabelas e índices, Módulos de apoio, Documentos, Relatórios, Agenda, Configurações e REA/validação.
- Dados do mock 100% fictícios, autorais e identificados como demonstração. O núcleo `rgps-core` roda no navegador via WebAssembly; a área Fábrica é a exceção explícita: envia somente o payload que o usuário preencher ao proxy local e à API pública do TRF3.
- A aba **Referências** consulta BCB SGS e IBGE SIDRA por um proxy com cache e importa CSV do CJF/SICOM; não faz scraping de calculadoras. Conecta/INSS fica fora até haver credenciamento e base legal.
- O adaptador `previus-rgps`, o CNIS, o workspace cifrado e o renderer de PDF continuam no desktop.
- Sem backend remoto, sem Tauri, sem PDF real no mock: exportações continuam simulações de interface. O proxy local da Fábrica é opcional e não persiste payloads, documentos ou resultados.
- Não é ferramenta oficial do INSS, não certifica direito e não substitui revisão profissional. O retorno da Fábrica é o serviço público do TRF3, sujeito aos mesmos limites de revisão.
- Interface em pt-BR, com tema claro/escuro, navegação por teclado e movimento reduzido.

## Brand Commitments

O nome previdenciar.io e a voz "às claras" são fixos. A identidade visual final continua provisória e aguarda OpenDesign; esta superfície adota o ATRIUM ui-v2 como referência visual (grafite, ouro, Playfair/Inter) e o movimento do painel GioMed IA, sem copiar marca ou conteúdo dos dois produtos.

## Evidence on Hand

README, docs/ e site/ do repositório; o app desktop em apps/desktop para vocabulário, fluxos e estados. Não existem depoimentos, clientes reais, benchmarks ou dados de CNIS no repositório. Toda a matéria do mock é sintética e assim deve permanecer rotulada.

## Product Principles

1. Origem antes do resultado: todo número pode ser rastreado até o documento ou a premissa.
2. Dado ausente não vira zero: pendência é um estado de primeira classe.
3. Local por padrão: privacidade é arquitetura, não promessa de marketing.
4. Limite declarado: o produto diz o que não faz, com a mesma clareza com que mostra o que faz.
5. Revisão profissional: o resultado organiza o trabalho, não o substitui.

## Accessibility & Inclusion

Meta WCAG 2.2 AA: contraste de texto, foco visível, alvos de toque, navegação completa por teclado, respeito a `prefers-reduced-motion` e leitura coerente em 390 px e 1440 px.
