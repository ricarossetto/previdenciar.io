# previdenciar.io — demonstração web

Versão web navegável do produto, feita para rodar no navegador sem build e sem o executável desktop. Cobre as 15 áreas do app com dados fictícios, a linguagem visual própria do previdenciar.io, o **motor `rgps-core` real compilado para WebAssembly** e uma área **Fábrica de Cálculos** que fala com a API oficial do TRF3 por um proxy local opcional. O adaptador, o CNIS e o PDF continuam no desktop.

## Abrir

1. **Somente o mock e o motor wasm:** abra `web/index.html` com duplo clique ou arraste para o navegador.
2. **Com a Fábrica de Cálculos:** copie `.env.example` para `.env`, preencha `FABRICA_API_KEY` e rode `node server/index.js`; abra `http://127.0.0.1:8787/`.
3. Se preferir um servidor estático: `python -m http.server 8080` na raiz e acesse `http://localhost:8080/web/`; a área da Fábrica precisará apontar para o proxy local em `http://127.0.0.1:8787`.

O servidor local usa apenas módulos nativos do Node: não há `npm install`, bundler, framework ou fonte remota no frontend. O motor wasm é embutido em base64 para funcionar direto de `file://`, e o `.wasm` também fica disponível ao lado da página. A chave da Fábrica fica no `.env` do servidor, nunca no navegador.

## O que cobre

As 15 áreas da navegação: Início, Clientes, Cálculos, Cálculos do RGPS, **Motor local (WASM)**, **Fábrica de Cálculos**, Assistente de cálculos, Editor de dados, Tabelas e índices, Módulos de apoio, Documentos, Relatórios, Agenda, Configurações e REA e validação.

O caso (Helena Fagundes) tem seis abas: visão geral com a régua de tempo, CNIS e vínculos, livro de contribuições, premissas, memória e relatórios.

## Interações

- `Ctrl/Cmd+K` ou “Buscar”: paleta com áreas, clientes e cálculos.
- Rail e dock inferior: navegação; em telas estreitas o rail vira gaveta.
- Tema claro/escuro e “Reduzir animações” no topo; ambos persistem no `localStorage`.
- Régua de tempo: foco ou hover em um período mostra origem, tempo e contribuições.
- Novo cálculo: escolha entre assistente guiado e editor manual.
- Assistente: sete etapas navegáveis com escolha e conferência.
- Relatórios: seleção de cálculo, documentos do lote, prévia de capa e **Imprimir** (somente a capa, em A4).
- Tabelas: troca de série; Documentos: troca de modelo; RGPS: troca de cenário.
- Filtros de clientes e cálculos funcionam no cliente, sem recarregar.
- Configurações: importar, exportar, baixar exemplo e restaurar demonstração.
- **Motor local (WASM)**: executa o `rgps-core` real no navegador com o fixture sintético de 300 competências ou com os dados do caso atual; mostra resumo, avisos e JSON bruto.
- **Fábrica de Cálculos**: calcula tempo e RMI, lê CNIS/RDCTC/Carta/PAP/PrevJud, consulta índices e expõe o console JSON para as demais rotas.

## Fábrica de Cálculos

A área usa a API pública da Fábrica de Cálculos do TRF3. O navegador **não recebe a chave**: ele chama o proxy local em `/api/fabrica/*`, e o proxy injeta `x-api-key` a partir de `FABRICA_API_KEY` no `.env`.

```powershell
Copy-Item .env.example .env
# edite .env e preencha FABRICA_API_KEY
node server/index.js
# abra http://127.0.0.1:8787/
```

Abas implementadas:

- **Tempo de contribuição:** segurado, espécie, opções de contagem, períodos, salários, cálculo e leitura dos benefícios cumpridos/não cumpridos. Inclui “Usar caso atual” para montar o payload a partir do workspace fictício.
- **Renda mensal (RMI):** mesmo payload, mais fundamento (`/fundamentos`) e marco temporal (`/marcos-temporais`); usa `/calculos-rmi/por-fundamento`.
- **Atrasados:** editor JSON com o esqueleto do contrato; usa `/calculos-atrasados`.
- **Documentos:** upload de CNIS, RDCTC, Carta de Concessão, PAP ou PrevJud; o resultado pode preencher o formulário de tempo.
- **Índices e limites:** limites, tabela de indexadores, reajuste, moedas, tábua de mortalidade, adiantamento e limite de baixa renda/reclusão.
- **Referências:** BCB SGS (salário mínimo, INPC, IPCA, IGP-M), IBGE SIDRA (IPCA/INPC) e importação de CSV do CJF/SICOM. O proxy guarda BCB/IBGE em cache por 6 h e a tabela CJF apenas em memória.
- **Console da API:** acesso direto a `/tempo`, `/tempo/por-grupos`, `/rmi`, `/rmi/por-fundamento`, `/rmi/desvinculada`, `/atrasados`, `/mcp`, fundamentos, marcos e índices.

Rotas locais do proxy em `server/index.js`:

| Rota local | Destino na Fábrica |
| --- | --- |
| `POST /api/fabrica/tempo` | `/rgps/api/v1/calculos-tempo` |
| `POST /api/fabrica/tempo/por-grupos` | `/rgps/api/v1/calculos-tempo/por-grupos` |
| `POST /api/fabrica/rmi` | `/rgps/api/v1/calculos-rmi` |
| `POST /api/fabrica/rmi/por-fundamento` | `/rgps/api/v1/calculos-rmi/por-fundamento` |
| `POST /api/fabrica/rmi/desvinculada` | `/rgps/api/v1/calculos-rmi/desvinculada` |
| `POST /api/fabrica/atrasados` | `/rgps/api/v1/calculos-atrasados` |
| `POST /api/fabrica/documentos/{tipo}` | `/rgps/api/v1/documentos/{tipo}/ler` |
| `GET /api/fabrica/fundamentos` | `/rgps/api/v1/fundamentos` |
| `GET /api/fabrica/marcos-temporais` | `/rgps/api/v1/marcos-temporais` |
| `GET /api/fabrica/indices/{nome}` | `/indices/api/v1/{nome}` |
| `POST /api/fabrica/mcp` | `/mcp` |
| `GET /api/referencias/bcb/{serie}` | BCB SGS (cache 6 h) |
| `GET /api/referencias/ibge/{ipca,inpc}` | IBGE SIDRA (cache 6 h) |
| `POST /api/referencias/cjf` | Importa CSV do CJF/SICOM para a memória do proxy |
| `GET /api/referencias/cjf` | Devolve a última tabela CJF importada |

O proxy encaminha o envelope PDPJ sem reinterpretar o resultado. Erros 400 mostram as mensagens de validação; 429 preserva `Retry-After`; uploads e respostas têm limite local. O proxy não grava payloads, documentos ou resultados. Para produção, coloque-o atrás de autenticação e HTTPS; não exponha a chave em um frontend público.

**Conecta/INSS** não está integrado: a API de benefícios exige credenciamento, chave, whitelist de IP e base legal para dados pessoais. **DataJud/Atrium** fica para metadados e publicações processuais; cálculo e referências continuam na Fábrica. O CJF/SICOM entra como CSV importado, não por scraping.

## Motor local (WASM)

O `rgps-core` 0.1.0 foi compilado para `wasm32-unknown-unknown` (453.273 bytes) e embutido em `engine.data.js` para rodar em `file://`. A página **Motor local (WASM)** oferece duas entradas:

- **Exemplo sintético**: o fixture de 300 competências do repositório; saída atual R$ 1.600,00 de RMI, 300 competências.
- **Dados do caso**: monta vínculos e competências a partir do workspace atual no módulo RGPS; no caso de demonstração são 12 competências e o motor devolve R$ 1.728,50 com os avisos de coeficiente e correção. PcD, pensão e outros módulos ficam no desktop.

A ABI é JSON puro: `pio_alloc`, `pio_calculate`, `pio_free` e `pio_version`, com envelope `{ ok, engine, version, data | error }`. Não há WASI, filesystem, rede, Tauri ou CNIS no wasm. O adaptador `previdenciar-rgps`, o workspace cifrado e o renderer de PDF continuam no desktop.

Regerar:

```powershell
scripts\build-web-engine.ps1
# ou:
cargo build -p previdenciar-wasm --target wasm32-unknown-unknown --release
node scripts\embed-web-engine.js
```

Detalhes em `ENGINE.md`.

## Dados reais

O mock aceita um **workspace JSON v1** em Configurações → Dados do workspace:

- **Importar JSON**: valida o contrato, substitui clientes, casos, períodos, contribuições, agenda e documentos, e persiste no `localStorage`.
- **Baixar dados**: exporta o workspace atual em `previdenciar-workspace.json`.
- **Baixar exemplo**: baixa `workspace.example.json`, um arquivo mínimo e válido para editar à mão.
- **Restaurar demonstração**: volta aos dados fictícios e remove o workspace salvo.

O arquivo **não é cifrado** e não sai do navegador; trate-o como dado de cliente. O **`rgps-core` roda no navegador via WebAssembly** na página Motor local; o adaptador `previdenciar-rgps`, o CNIS, o workspace cifrado e o PDF continuam no desktop. Campos ausentes aparecem como `—` ou estado vazio, nunca como número inventado. O app desktop ainda não exporta este formato; o próximo passo é um exportador no Tauri que gere o mesmo contrato.

O contrato completo está em `OVERNIGHT.md` e o exemplo em `workspace.example.json`. Campos de caso opcionais incluem `cnisCount`, `adjustments`, `used`, `pending`, `cnisPages`, `importedAt`, `pendencies`, `disability`, `ctc`, `special` e `rural`.

## Arquivos

| Arquivo | Papel |
| --- | --- |
| `index.html` | Carcaça, topbar, rail, dock, meta e favicon |
| `styles.css` | Base de layout, componentes e regras de impressão |
| `visual.css` | Camada visual: tokens, shell, tipografia, fólios, estados e movimento |
| `fonts/` | Inter 400/500/600/700 e Playfair Display 500/600/700 locais |
| `core.js` | Ícones, estado, helpers, formatação e semântica de status |
| `data.js`, `data2.js`, `data3.js` | Dados fictícios: clientes, casos, vínculos, contribuições, séries, modelos, cenários |
| `views-main.js` | Início, Clientes, Cálculos e RGPS |
| `views-case.js` | Caso, régua de tempo, abas, pendências e resultado |
| `views-more.js` | Assistente, Editor, Tabelas, Módulos e Documentos |
| `views-rest.js` | Relatórios, Agenda, Configurações e REA |
| `views-engine.js` | Página Motor local; entrada do caso, execução e saída |
| `api.js` | Cliente do proxy da Fábrica, base local e tratamento do envelope PDPJ |
| `views-fabrica.js` | Abas Tempo, RMI, Atrasados, Documentos, Índices e Console |
| `fabrica.css` | Componentes da área da Fábrica (métricas, períodos, resultados, JSON) |
| `../server/index.js` | Proxy local sem dependências: estático, CORS, `x-api-key` e rotas da Fábrica |
| `../.env.example` | Modelo de configuração do proxy (copie para `.env`) |
| `workspace.js` | Contrato v1, validação, importação, exportação, localStorage e restauração |
| `workspace.example.json` | Workspace mínimo válido para editar e importar |
| `engine.js` | Loader da ABI wasm (base64 e memória) |
| `engine.data.js` | Wasm embutido em base64 para `file://` (gerado) |
| `engine.wasm` | Build wasm32-unknown-unknown do `rgps-core` (gerado) |
| `engine.example.js`, `engine.example.json` | Fixture sintético de 300 competências (gerado) |
| `app.js` | Render, rotas por hash, eventos, paleta, modais, toast e persistência |
| `DESIGN.md` | Sistema visual da superfície web |
| `ENGINE.md` | ABI, contrato de entrada, build e limites do wasm |
| `OVERNIGHT.md` | Runbook e registro da execução |

## Limites desta demonstração

- Executa o **`rgps-core`** via WebAssembly, mas não o adaptador `previdenciar-rgps`, o CNIS, o workspace cifrado ou o renderer de PDF; esses continuam no desktop.
- A **Fábrica de Cálculos** depende do proxy local, da rede e da disponibilidade da API do TRF3; sem o proxy, o restante do mock continua funcionando offline.
- A área **Documentos** envia o PDF ao proxy em memória para leitura; o proxy não grava o arquivo. Use somente dados sintéticos nesta demonstração.
- O resultado do wasm é experimental e pode bloquear entrada incompleta; ele não certifica direito. O resultado da API é o retorno oficial do serviço, também sujeito a revisão profissional.
- Não exporta PDF; os botões de prévia, lote e exportação são simulações rotuladas. A impressão usa a capa do relatório.
- O JSON importado fica em `localStorage` sem cifra; não use dados reais em máquina compartilhada.
- Não é ferramenta oficial do INSS, não certifica direito e não substitui revisão profissional.
- A identidade visual é provisória no produto; esta superfície propõe uma extensão web, não a identidade final.

## Próximos passos naturais

1. Criar no Tauri um exportador que gere este `workspace.json` v1 a partir da base local, para a web abrir dados reais sem backend.
2. Ligar o assistente e o editor ao wasm para recalcular o caso na hora, com confirmação de revisão e sem perder a origem.
3. Completar o formulário guiado de atrasados com os enums de consectários e benefícios; hoje ele usa o editor JSON.
4. Salvar resultados da Fábrica como snapshot no workspace e oferecer download do demonstrativo/permalink.
5. Usar o renderer Rust para gerar os PDFs e uma fila de lote com pasta de destino; ligar “Imprimir” à prévia real.
6. Cobrir os estados vazios de primeira execução, o modo somente leitura e a migração de versões do contrato.
7. Revisar a proposta visual com o time de design antes de promover a identidade.

## Processo

A superfície usa a camada visual própria do previdenciar.io, com fontes Inter/Playfair locais. O contrato de direção em `../.impeccable/surfaces/web.md` foi atualizado; a implementação está em `visual.css`, carregado depois de `styles.css` e `fabrica.css`. O detector determinístico e o `axe-core` foram usados como piso: tokens, foco, movimento reduzido, sem gradiente em texto, sem cartão dentro de cartão, sem ícone-emoji e sem métrica inventada.

Verificação da ponte de dados: importação do exemplo (1 cliente, 1 caso, 3 contribuições), persistência após reload, exportação de volta em JSON v1, restauração da demonstração (8 clientes, 9 casos), rejeição de workspace inválido com mensagem, e impressão em mídia `print` com rail, lista lateral e chrome ocultos.

Verificação do motor: build `wasm32-unknown-unknown` de 453.273 bytes; Node e Chrome executaram o fixture de 300 competências (RMI R$ 1.600,00) e o caso de demonstração (12 competências, RMI R$ 1.728,50), sem erros de console e sem requisições externas. `axe-core` (WCAG 2 A/AA, 2.1 e 2.2 AA) sem violações nas telas amostradas; 20 views a 390 px sem overflow horizontal.

Verificação visual: tema escuro por padrão e claro disponível; Inter/Playfair carregadas de `web/fonts/`; entrada de página, stagger de cards, hover de navegação e press de botões; `axe-core` sem violações em Home, Caso, Relatórios, Configurações, Motor, Fábrica e Referências; 390 px sem overflow; impressão em A4 com chrome oculto e capa branca. As capturas ficam em `../work/` (local, não versionado).

Verificação da Fábrica: `GET /api/fabrica/health` respondeu `configured: true`; `POST /api/fabrica/tempo` devolveu `status: ok` com 2.820 dias, 94 meses de carência e benefícios analisados para um payload sintético; `POST /api/fabrica/rmi/por-fundamento` devolveu `status: ok`; a UI mostrou “Conectado”, calculou tempo e RMI e consultou limites. A área foi verificada em `file://` e em `http://127.0.0.1:8787/`, a 1440 px e 390 px, sem erros de console, sem overflow horizontal e com `axe-core` sem violações. O proxy encaminha a chave apenas no servidor; o `.env` é ignorado pelo git.

Verificação das referências: BCB SGS 1619 devolveu 12 pontos com salário mínimo `R$ 1.621,00`; a segunda chamada veio do cache (`X-Ref-Cache: HIT`); IBGE SIDRA devolveu IPCA/INPC no intervalo; CSV do CJF foi importado com 2 linhas e relido do cache em memória; `axe-core` sem violações na aba Referências, a 1440 px e 390 px. `Conecta/INSS` e `DataJud/Atrium` ficaram fora por decisão explícita.
