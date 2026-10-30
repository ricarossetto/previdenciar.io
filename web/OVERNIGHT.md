# Runbook overnight — da demonstração à versão web com dados reais

**Objetivo:** transformar o mock navegável em uma versão web que abre no navegador, carrega um workspace real exportado em JSON, persiste localmente, exporta de volta, imprime a prévia do relatório e mantém a demonstração como fallback.

**Fora de escopo original:** importar CNIS/PDF, gerar PDF real, backend, autenticação ou sincronização. O **`rgps-core`** foi incluído depois como extensão WebAssembly; o adaptador `previdenciar-rgps`, o CNIS, o workspace cifrado e o PDF continuam no app desktop.

## Fases

| Fase | Entrega | Critério de aceite |
| --- | --- | --- |
| 0 | Linha de base | `web/` abre em `file://`; 13 áreas; detector e axe limpos |
| 1 | Contrato `workspace` v1 | JSON versionado com clientes, casos, períodos, contribuições, agenda, documentos; validação com mensagem clara |
| 2 | Ponte de dados | Importar JSON, exportar JSON, persistir no `localStorage`, restaurar demonstração, baixar exemplo |
| 3 | Views data-driven | Contagens, pendências, contribuições por caso e origem derivadas dos dados; estados vazios sem quebrar |
| 4 | Impressão | `window.print()` na prévia; rail/topbar/dock ocultos; capa e tabelas legíveis em A4 |
| 5 | Verificação | Detector 0; axe 0; sem overflow em 390 px; round-trip import/export; print sem erros; zero rede |
| 6 | Documentação | README, DESIGN, exemplo de workspace e este runbook atualizados |

## Contrato v1

```json
{
  "version": 1,
  "exportedAt": "2026-10-26T00:00:00.000Z",
  "app": "previdenciar.io",
  "clients": [{ "id": 1, "name": "...", "birth": "1968-02-14" }],
  "cases": [{ "id": 1, "client": 1, "name": "...", "kind": "RGPS", "rule": "...", "dib": "2026-08-01", "status": "Calculado", "rmi": 4812.37, "months": 431, "contribs": 431, "start": "1990-03", "end": "2026-06", "updated": "2026-10-24" }],
  "periods": { "1": [{ "employer": "...", "kind": "Empregado", "start": "1990-03", "end": "1997-12", "contribs": 94, "status": "conferido", "source": "CNIS p. 4" }] },
  "contributions": { "1": [{ "comp": "2026-05", "link": "...", "value": "4.120,00", "origin": "CNIS", "status": "conferido" }] },
  "appointments": [{ "date": "2026-10-26", "time": "09:00", "title": "...", "who": "..." }],
  "documents": [{ "id": 1, "name": "...", "category": "...", "updated": "2026-10-12", "body": "..." }]
}
```

Campos opcionais por caso: `cnisCount`, `adjustments`, `used`, `pending`, `cnisPages`, `importedAt`, `pendencies`, `disability`, `ctc`, `special`, `rural`. Quando ausentes, a interface mostra `—` ou um texto genérico, nunca um número inventado.

## Decisões autônomas

- O núcleo da versão web continua sem build e sem rede: o arquivo JSON entra por `<input type="file">` e a persistência é `localStorage`. A Fábrica de Cálculos e a aba Referências usam um proxy local opcional para APIs públicas; o motor wasm permanece offline.
- O motor não é simulado em dados importados: se um caso não trouxer RMI, a interface mostra `—` e o selo “Experimental/Bloqueado” conforme o estado.
- A demonstração permanece intacta em memória; “Restaurar demonstração” limpa o workspace salvo.
- A impressão usa a prévia do relatório, não o app inteiro; a capa e as tabelas são o documento.

## Registro

- [x] Fase 0 — linha de base concluída.
- [x] Fase 1 — contrato v1, validação e normalização em `workspace.js`; exemplo em `workspace.example.json`.
- [x] Fase 2 — importar, exportar, persistir, restaurar demo e baixar exemplo em Configurações.
- [x] Fase 3 — contagens, pendências, contribuições por caso, origem e estados vazios derivados do workspace.
- [x] Fase 4 — `@media print` A4; capa flex; toolbar, lista lateral e chrome ocultos; botão Imprimir.
- [x] Fase 5 — verificação completa abaixo.
- [x] Fase 6 — README, DESIGN, exemplo e este runbook atualizados.

## Extensão — motor WASM

O núcleo puro `rgps-core` (só `rust_decimal`, `serde`, `serde_json`, `thiserror`) foi compilado para `wasm32-unknown-unknown` no crate `crates/previdenciar-wasm`, com ABI JSON própria (`pio_alloc`, `pio_calculate`, `pio_free`, `pio_version`). O adaptador `previdenciar-rgps` não entra: ele depende de CNIS/workspace e não é wasm-friendly.

- Build: `scripts/build-web-engine.ps1` → `engine.wasm` de **453.273 bytes**, base64 em `engine.data.js` (604.393 bytes) para rodar em `file://`.
- Página **Motor local (WASM)** na navegação, com duas entradas: fixture sintético de 300 competências e dados do caso atual.
- Resultado do fixture: 300 competências, coeficiente 80, **RMI R$ 1.600,00**.
- Resultado do caso de demonstração: 12 competências, coeficiente 60, **RMI R$ 1.728,50**, com aviso de coeficiente e correção — a saída é do motor, não do valor cenográfico do caso.
- Chrome em `file://`: executa, sem erro de console e sem requisição externa. Node também executa o wasm e lê a memória.
- Testes nativos: `cargo test -p previdenciar-wasm` (fixture 300 + entrada inválida) e testes do `rgps-core` continuam no workspace.

O wasm é o núcleo de cálculo, não o app: não importa CNIS, não lê a base cifrada, não gera PDF e não aplica as políticas do adaptador. O resultado continua experimental.

## Resultados

| Verificação | Resultado |
| --- | --- |
| `impeccable detect web` | exit 0, sem achados |
| `axe-core` WCAG 2 A/AA, 2.1, 2.2 AA | 0 violações em Início, Caso, Contribuições, Assistente, Relatórios, Configurações e Caso mobile |
| 19 views a 390 px | 0 overflow horizontal, 0 erros de console |
| Importar `workspace.example.json` | 1 cliente, 1 caso, 3 contribuições, 1 compromisso, 1 documento |
| Persistência | “Dados locais” após reload |
| Contribuições do caso importado | 3 linhas, selo “1 pendência” |
| Exportar | JSON v1, 1 cliente, 1 caso, 1 mapa de contribuições |
| Restaurar demonstração | 8 clientes, 9 casos, 14 contribuições, 6 compromissos, 6 documentos; permanece após reload |
| JSON inválido | rejeitado com “O workspace precisa de pelo menos um cliente.”, estado anterior intacto |
| Impressão | rail `none`, lista lateral `none`, capa `flex`; captura em `../work/web-review/print-preview.png` |
| Build wasm | 453.273 bytes; `cargo test -p previdenciar-wasm` 2/2 |
| Motor no Chrome | exemplo 300 → R$ 1.600,00; caso 12 → R$ 1.728,50; 0 erros; 0 rede |
| Motor no Node | `WebAssembly.instantiate` + leitura de memória ok |
| Rede | 0 requisições externas |

## Incidente e recuperação

Uma manipulação de `data.js`, `data2.js` e `data3.js` via PowerShell sem `-Encoding utf8` decodificou os arquivos como ANSI e os regravou como UTF-8, produzindo mojibake. A recuperação foi feita por dupla decodificação (UTF-8 → Latin-1 → UTF-8), com substituição dos caracteres perdidos (`—`) e remoção do BOM. A verificação com Node confirmou `contribuição` presente e ausência de `Ã` e `U+FFFD`; `node --check` passou em todos os arquivos. Nenhuma perda permaneceu nos dados de demonstração.

## Arquivos entregues

`index.html`, `styles.css`, `core.js`, `data.js`, `data2.js`, `data3.js`, `workspace.js`, `engine.js`, `engine.data.js`, `engine.wasm`, `engine.example.js`, `engine.example.json`, `views-main.js`, `views-case.js`, `views-more.js`, `views-rest.js`, `views-engine.js`, `app.js`, `workspace.example.json`, `README.md`, `DESIGN.md`, `ENGINE.md`, `OVERNIGHT.md`; no Rust: `crates/previdenciar-wasm/`; no build: `scripts/build-web-engine.ps1`, `scripts/embed-web-engine.js`.
