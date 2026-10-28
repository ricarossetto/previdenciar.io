# previdenciar.io web — ATRIUM ui-v2

Sistema visual da demonstração web. A referência visual é o **ATRIUM atual** (`atrium-pro`, `css/views/ui-v2/*`); o movimento é adaptado do painel do **GioMed IA** (`gi-clinical`, `apps/web/src/styles/motion.css` e `components/amicro.tsx`). O conteúdo, os fluxos e a semântica continuam os do previdenciar.io.

## Tese

O trabalho continua sendo um livro-razão de contribuições, mas a superfície usa a linguagem do ATRIUM: fundo grafite, superfícies em camadas, tipografia editorial e ouro como acento. A densidade e a origem do dado continuam visíveis; a mudança é de linguagem, não de produto.

## Referências

- **Visual:** `atrium-pro/css/views/ui-v2/tokens.css`, `primitives.css`, `shell.css` e `dashboard.css`.
- **Movimento:** `gi-clinical/apps/web/src/styles/motion.css` e `components/amicro.tsx` (Rise, Stagger, Press, Hover, Heartbeat/LiquidDots como vocabulário).
- **Não copiar:** marca, logo, navegação ou textos do ATRIUM/GioMed. A identidade previdenciar.io permanece.
- As fontes Inter e Playfair Display são locais (`web/fonts/`), com licença aberta, para o mock continuar offline.

## Paleta

| Papel | Escuro (padrão) | Claro |
| --- | --- | --- |
| fundo | `#111315` | `#F3F4F2` |
| fundo sutil / sidebar | `#15181B` | `#ECEEEB` |
| superfície | `#181B1E` | `#FAFAF8` |
| superfície secundária | `#202428` | `#EEF0EF` |
| superfície elevada / fólio | `#282D32` | `#FFFFFF` |
| tinta | `#ECEEED` | `#252A2E` |
| tinta secundária | `#B4B9BC` | `#555D63` |
| texto mudo | `#989FA4` | `#565E63` |
| primário (navegação/estado) | `#8DA1AF` | `#4F5F6B` |
| ouro / ação e destaque | `#C0A261` | `#806226` |
| ouro suave | `#30291D` | `#EEE6D5` |
| sucesso | `#8AA08E` | `#566C5A` |
| atenção | `#C39A58` | `#8C5C22` |
| perigo | `#CD7F77` | `#9D4D45` |
| filete | `#343A3F` | `#D2D6D5` |
| filete forte | `#485057` | `#B8C0C2` |

Ação primária no escuro usa o `--v2-action-primary-*` (ouro translúcido); no claro usa o azul-acinzentado do ATRIUM. Ouro é destaque editorial e ação; o estado nunca depende só da cor.

## Tipografia

- Interface: **Inter** 400/500/600/700.
- Editorial: **Playfair Display** 500/600/700 em títulos, fólios e capa.
- Dados: `Cascadia Mono`/`Consolas`, com `font-variant-numeric: tabular-nums`.
- Base 14 px; corpo 13–14; títulos de fólio 15; h1 28–36; medida de 65–75ch.
- Sem fonte remota; os quatro pesos de Inter e três de Playfair ficam em `web/fonts/`.

## Grade e composição

- Sidebar de **240 px** + área de trabalho; topbar de **64 px**; conteúdo até 1440 px com gutter fluido.
- Sidebar: fundo grafite, filete de ouro sob a marca, grupos em caixa-alta, item ativo com fundo `primary-soft`, filete de 3 px em ouro e ícone dourado.
- Cabeçalho de página: Playfair + frase em Inter; filete inferior e um traço de ouro de 68 px. Sem eyebrow inventado.
- Fólio: superfície elevada, borda de 1 px, raio 14 px e sombra `--v2-shadow-card`; cabeçalho com filete.
- Tabelas: cabeçalho 10.5 px em caixa-alta, corpo 13 px; hover de superfície secundária.
- Formulários: 40 px de altura, raio 10 px, fundo `surface-inset`, foco de 2 px.
- Botões: 40 px, raio 10 px, Inter 13/600; primário em ouro translúcido; ícone 16–18 px.
- Abas: controle segmentado (fundo secundário, raio 10 px, ativo em superfície), não sublinhado.

## Movimento (GI)

- Entrada de página: `v2-view-enter` 280 ms, opacidade + 6 px; filhos diretos com atraso de 40 ms.
- Stagger de cards: 40 ms por item, `mo-stagger-in` 200 ms, no máximo 4–6 itens.
- Hover: nav desloca 2 px, ícone escala 1.08; cards clicáveis sobem 1 px e ganham `shadow-elevated`.
- Press: botão desce 1 px e escala 0.99; ícone escala 0.94.
- Atenção: ponto de `state--warn` pulsa com `mo-pulse` 1.7 s.
- Overlays: paleta/modal com `v2-popover-enter`; toast e avisos com `v2-feedback-enter`.
- `prefers-reduced-motion` e o botão “Reduzir animações” desligam tudo; conteúdo visível por padrão.
- Nada de layout animado, `transition: all`, parallax gratuito ou animação bloqueando interação.

## Componentes e estados

- `state`: pílula com ponto semântico; ok, atenção, bloqueio e neutro.
- `stamp`: selo de 1 px para “Calculado”, “Experimental” e “Revisada”; sem rotação.
- `pendency`: quadrado de 6 px + texto; perigo para bloqueio, ouro para atenção, sucesso para conferido.
- `notice`: superfície com borda de ouro ou perigo, ícone e texto; sem borda lateral acima de 1 px.
- `tab`: segmento com fundo ativo; `aria-selected` obrigatório.
- `switch`: 40 × 22 px, thumb de 16 px; estado por posição e cor.
- `palette` e `modal`: scrim com desfoque, superfície elevada, foco no primeiro controle, Esc fecha.
- `toast`: superfície elevada, borda, ícone; `role="status"` e `aria-live`.
- Estados cobertos: hover, foco, ativo, desabilitado, erro, atenção, bloqueado, vazio, demonstração.

## Dados e estados

- O workspace v1 é a fonte da verdade da superfície; a demonstração é um workspace em memória.
- Origem visível: “Demonstração” em ouro e “Dados locais” em info; pé do rail e rodapé acompanham.
- Configurações → Dados do workspace: importar, exportar, baixar exemplo, restaurar demonstração.
- Caso sem contribuições: estado vazio explícito. Caso sem RMI: `—`, nunca zero.
- Datas e números usam `dateBR` e `brl`; campos ausentes mostram `—`.
- O `rgps-core` roda no navegador via WebAssembly; a Fábrica usa o proxy local e a API pública. O resultado do wasm é experimental; o da Fábrica é o retorno oficial do serviço, também sujeito a revisão.
- A aba Referências consulta BCB/IBGE e importa CSV do CJF; não faz scraping de calculadoras.

## Impressão

- `@media print` em A4, margem de 14 mm: rail, topbar, dock, rodapé, toolbar, lista lateral e chrome saem; a capa ocupa a página.
- A capa vira uma coluna flex, fundo branco, sem borda; `page-break-after: always`.
- O botão “Imprimir” em Relatórios chama `window.print()`; nenhum PDF é gerado pelo navegador.

## Responsivo

- 1120 px: settings e agenda em uma coluna.
- 960 px: rail vira gaveta com scrim; dock inferior; grids de caso e home em uma coluna; pass em duas colunas.
- 680 px: cabeçalho e ações empilhados; formulários e pares chave-valor em uma coluna; busca ocupa a linha.
- `minmax(0, 1fr)` em tracks com tabela/régua/texto longo; a página nunca rola na horizontal.

## Acessibilidade

- WCAG 2.2 AA: corpo ≥ 4.5:1, texto grande ≥ 3:1.
- Foco visível de 2 px, `:focus-visible`; `h1` recebe foco programático.
- Navegação por teclado em rail, dock, abas, paleta, modais e régua; `aria-current`, `aria-selected`, `aria-pressed`, `aria-live`.
- Alvos de toque ≥ 40 px nos controles principais; dock ≥ 46 px.
- `prefers-reduced-motion` + alternância manual; conteúdo nunca depende de animação para aparecer.
- Idioma `pt-BR`; `Ctrl/Cmd+K`; Esc fecha; link “Pular para o conteúdo”.

## Conteúdo

- pt-BR, voz “às claras”: frases curtas, termos do domínio (vínculo, competência, carência, DIB, RMI, memória, pendência).
- Dados fictícios, rotulados como demonstração; sem CNIS real, nome de cliente real ou promessa oficial do INSS.
- Números sempre com origem; pendência nunca vira zero.
- Nada de métrica inventada, depoimento, logo de cliente ou benchmark.

## Não fazer

- Não copiar marca, logo ou textos do ATRIUM/GioMed; a identidade exibida é previdenciar.io.
- Não trocar o fólio por cartões de ícone + título + texto.
- Não usar gradiente em texto, glass decorativo, emoji como ícone, cartão dentro de cartão ou sombra dura.
- Não usar monoespaçado como fantasia; só para dado e medida.
- Não prometer backend remoto, PDF real ou cálculo oficial nesta superfície. O proxy da Fábrica é local e explícito.
- Não adicionar dependência ou fonte remota ao mock; as fontes são locais e o núcleo não faz requisição.

## Procedência

- Direção visual: ATRIUM ui-v2, `atrium-pro` (commit `b49d200`).
- Movimento: GioMed IA, `gi-clinical` (commit `b75bce8`).
- Implementação: `web/atrium.css`, carregado depois de `styles.css` e `fabrica.css`.
- Verificação: 15 áreas + 6 abas de caso + Motor local + Fábrica + Referências em 390 px e 1440 px; sem overflow horizontal; sem erros de console; `axe-core` 0 nas telas amostradas; tema claro/escuro e movimento reduzido; impressão em A4.
- Wasm: `rgps-core` 0.1.0 em wasm32 (453.273 bytes), fixture de 300 competências e caso de 12 competências executados no Chrome a partir de `file://`, sem rede.
