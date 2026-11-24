# Proxy local da Fábrica de Cálculos

Servidor HTTP mínimo, sem dependências, que:

- serve o mock web em `web/`;
- injeta `x-api-key` nas chamadas à Fábrica de Cálculos do TRF3;
- libera CORS para o mock aberto em `file://` ou em outro servidor local;
- repassa o envelope PDPJ sem reescrever o resultado;
- mantém a chave fora do navegador e fora do git.

## Rodar

```powershell
Copy-Item ..\.env.example ..\.env
# edite ..\.env e preencha FABRICA_API_KEY
node index.js
```

Abra `http://127.0.0.1:8787/`.

Variáveis:

| Variável | Padrão | Uso |
| --- | --- | --- |
| `FABRICA_API_KEY` | vazio | Obrigatória para as rotas da API. |
| `FABRICA_BASE_URL` | `https://fabrica-de-calculos.app.trf3.jus.br` | Destino do proxy. |
| `PORT` | `8787` | Porta local. |
| `HOST` | `127.0.0.1` | Interface de escuta. |

## Rotas

| Local | Destino |
| --- | --- |
| `GET /api/fabrica/health` | Estado do proxy (não expõe a chave). |
| `POST /api/fabrica/tempo` | `/rgps/api/v1/calculos-tempo` |
| `POST /api/fabrica/tempo/por-grupos` | `/rgps/api/v1/calculos-tempo/por-grupos` |
| `POST /api/fabrica/rmi` | `/rgps/api/v1/calculos-rmi` |
| `POST /api/fabrica/rmi/por-fundamento` | `/rgps/api/v1/calculos-rmi/por-fundamento` |
| `POST /api/fabrica/rmi/desvinculada` | `/rgps/api/v1/calculos-rmi/desvinculada` |
| `POST /api/fabrica/atrasados` | `/rgps/api/v1/calculos-atrasados` |
| `POST /api/fabrica/tempo/prevjud` | `/rgps/api/v1/calculos-tempo/prevjud/calcular` |
| `POST /api/fabrica/{tempo,atrasados,rmi}/calc/*` | Rotas `arquivo-calc` correspondentes (multipart `.calc`) |
| `POST /api/fabrica/rmi/async`, `/stream` | `/rgps/api/v1/calculos-rmi/*` |
| `GET /api/fabrica/rmi/jobs/:id[/resultado]` | `/rgps/api/v1/calculos-rmi/jobs/:id[/resultado]` |
| `POST /api/fabrica/documentos/:tipo` | `/rgps/api/v1/documentos/:tipo/ler` (`cnis`, `rdctc`, `carta`, `pap`, `prevjud`) |
| `GET /api/fabrica/fundamentos` | `/rgps/api/v1/fundamentos` |
| `GET /api/fabrica/marcos-temporais` | `/rgps/api/v1/marcos-temporais` |
| `GET /api/fabrica/calculadoras` | `/apps/api/v1/calculadoras` |
| `GET /api/fabrica/documentos-legais[/:slug]` | `/apps/api/v1/documentos-legais[/:slug]` |
| `GET /api/fabrica/indices/:nome` | `/indices/api/v1/:nome` |
| `GET /api/fabrica/indices/tabela/:competencia` | `/indices/api/v1/tabela/:competencia` |
| `POST /api/fabrica/exchange` | `/apps/api/v1/exchange` |
| `POST /api/fabrica/mcp` | `/mcp` |
| `GET /api/referencias/bcb/:serie` | Banco Central — SGS, cache de 6 h |
| `GET /api/referencias/ibge/:ipca|inpc` | IBGE — SIDRA, cache de 6 h |
| `POST /api/referencias/cjf` | Importa CSV do CJF/SICOM para memória |
| `GET /api/referencias/cjf` | Devolve a última tabela CJF importada |

`demonstrativo` e `permalink` passam como query para as rotas que os aceitam.

## Limites e segurança

- Corpo JSON: 12 MiB. Upload: 32 MiB. Resposta upstream: 48 MiB.
- Cálculos: até 180 s no proxy (a API pode responder 429 com `Retry-After`).
- Referências: BCB/IBGE em cache de 6 h; o BCB aceita até 120 pontos por consulta e o SGS limita intervalos longos a 10 anos. A tabela CJF é CSV de até 8 MiB e 5.000 linhas, mantida apenas na memória do processo.
- As rotas de referência só acessam hosts fixos (`api.bcb.gov.br` e `servicodados.ibge.gov.br`); não há URL arbitrária.
- Sem autenticação local: escute em `127.0.0.1` e não exponha o proxy publicamente sem uma camada de autenticação e HTTPS.
- Não há log de corpo, documento, resultado ou chave. O log é apenas método, caminho, status e duração.
- Conecta/INSS e DataJud/Atrium não estão integrados neste proxy: o primeiro exige credenciamento e base legal; o segundo é escopo do Atrium MCP.
- O `.env` é ignorado pelo git (`/.env`, `.env.*`, com exceção de `.env.example`).
- A chave fornecida para testes é pública, mas trate qualquer chave como segredo operacional.

## Teste rápido

```powershell
Invoke-RestMethod http://127.0.0.1:8787/api/fabrica/health
```

Se `configured` for `false`, o proxy está no ar, mas falta `FABRICA_API_KEY`.
