# Auditoria técnica — 03/11/2026

- Repositório: `previdenciar/desktop` (todos os caminhos abaixo são relativos a ele).
- Branch: `wip/consolidacao-20261102`.
- HEAD de código auditado: `f3c753eb6728696c01d44a18a0c8703a60911750`. `AUDITORIA.md` não está commitado e não possui hash git próprio; o hash do arquivo é registrado apenas como prova documental.
- Natureza: **somente leitura**. Nenhum código, configuração, manifesto, teste, migration, container ou serviço foi criado ou alterado. Nenhum Cargo, npm, build, Docker, instalação, migration, rede, push, `git add` ou commit foi executado.
- Runtime: **NÃO verificado**. O relatório distingue código estático, teste histórico e hipótese; não declara que a aplicação rodou.
- Estado do relatório: **primeira versão aprovada estaticamente** em 03/11/2026 pelo revisor `ses_e606c88a9ffeoRdLAc0xfCFtWa`, que verificou R1–R5 nas fontes e não executou runtime. Esta versão com ajustes documentais foi submetida a nova revisão; a aprovação anterior não é aprovação de implementação, runtime ou push.
- Escopo: os cinco prompts pedidos — 1) testes de integração; 2) confiança no cliente; 3) concorrência; 4) idempotência; 5) separação de responsabilidades. O foco é o produto web oficial (`apps/server`, `apps/desktop/src` e crates Rust consumidos). O desktop Tauri é legado congelado e não foi auditado como se fosse o web. O repositório separado de marketing (`../web`) não foi lido nem editado; não era necessário.
- Ambiente medido no início: C: com 85,77 GiB livres e 389,57 GiB usados; `target` com 43.435.003.754 bytes (40,45 GiB, valor reaproveitado da medição já existente); RAM total de 15,66 GiB e 1,55 GiB livres. O baseline M1.3 permanece 46,66 GiB aproximado, **sem reset**; o check real continua exit 1 e Cargo continua bloqueado. M1.3b está suspenso.
- Este é o único arquivo de projeto gerado. Notas de coordenação ficam na saga externa. Não houve commit nem push deste relatório.

## 1. Os cinco maiores riscos

| # | Risco | Gravidade | Evidência local | Status |
|---|---|---|---|---|
| R1 | **Corrida no primeiro workspace**: `bootstrap_workspace` faz `SELECT` e depois `INSERT` sem transação, e `workspaces.owner_id` não tem restrição única. Duas requisições concorrentes do mesmo usuário podem criar duas bases; a segunda fica órfã e pode haver `id` inconsistente entre a primeira resposta e a base usada depois. Nenhuma rota aceita `workspace id`, então não há fluxo de escrita na segunda base; não há perda de conteúdo demonstrada. | Alta — integridade de dados e criação duplicada; sem perda de conteúdo demonstrada | `apps/server/src/db.rs:19-41`; `apps/server/migrations/0001_initial.sql:12-22` (índice não único); teste `usuario_e_workspace_persistem_entre_chamadas` é sequencial (`apps/server/src/integration_db.rs:244-316`) | Revisor confirmou o trecho e o schema em revisão estática; banco real não executado nesta auditoria |
| R2 | **Admin por e-mail não comprovado**: o web oficial entra com **ID token** (scope `openid email profile`) e valida JWKS, issuer e audience; `authenticate` ainda aceita `email` quando `email_verified` está ausente e usa `username` como e-mail quando `email` falta. `is_admin` compara esse valor com `ADMIN_EMAILS`. A hipótese depende das claims do emissor, **não de access token**; a política admin (`sub` × e-mail verificado) é decisão de produto. | Alta, **condicional** ao emissor | `apps/server/src/auth.rs:59-78`; `apps/server/src/config.rs:164-167`; `apps/server/src/routes.rs:385-397`; `apps/desktop/src/auth-web.tsx:30-38` | Hipótese condicionada ao emissor; política de admin não decidida; nenhuma consulta ao Cognito real |
| R3 | **Entitlement `advanced` não é aplicado no servidor**: o valor é gravado, consultado e devolvido em `/api/me`, mas nenhuma rota verifica `advanced` antes de calcular, gerar PDF, importar CNIS ou usar a chave de IA da plataforma. A documentação diz que o módulo fica atrás de `previdenciario:advanced`, mas o front também não contém essa checagem. | Alta para produto/abuso; média para segurança de dados | `apps/server/src/routes.rs:335-350`; `apps/server/src/db.rs:88-109`; `docs/WEB_ARCHITECTURE.md:18`; busca em `apps/desktop/src` não encontra `advanced` | Comprovado por código; comportamento de produto não aprovado |
| R4 | **Chave de API da Fábrica em claro no workspace JSONB**: o documento inteiro é serializado em `workspaces.document`, incluindo `settings.fabrica_api_key`. O próprio teste de integração registra esse defeito e só verifica que a chave não aparece na auditoria ou para outro usuário. | Alta/ Média — segredo em repouso | `apps/server/src/db.rs:60-70`; `apps/server/src/routes.rs:705-736`; `apps/desktop/src/model.ts:22`; `apps/server/src/integration_db.rs:800-817` | Comprovado pelo código e pelo comentário do teste; estado do banco real não verificado |
| R5 | **Teste de integração não é gate seguro**: sem banco, os 12 caminhos Postgres retornam cedo e o teste passa; o CI não sobe Postgres. Além disso, `TEST_DATABASE_URL` não exige loopback (só `DATABASE_URL` exige) e o helper executa `CREATE EXTENSION` e migrações no banco apontado. | Alta para operação de teste; média para runtime | `apps/server/src/integration_db.rs:76-90,95-108,170-177`; `apps/server/README.md:55-57`; `.github/workflows/ci.yml:56`; `.github/workflows/web.yml:27`; `.github/workflows/release.yml:40` | Comprovado por código e workflow; nenhum teste de banco executado aqui |

Os cinco têm evidência local. R2 é o único que depende de configuração externa; R5 é risco de gate e operação, não de porta aberta em produção. Não há prova de exploração em runtime.

## 2. Método, comandos e limites

Comandos seguros usados: `git rev-parse`, `git status`, `git ls-files`, `rg`, `Get-Content`, `Get-ChildItem`, `Get-PSDrive`, `Get-CimInstance` e leitura de arquivos. Não foram executados `cargo`, `npm`, `node --test`, build, Docker, `docker run`, migração, psql, acesso a banco, acesso a `.env` (valores), acesso a CNIS/PDF pessoal, nem chamadas de rede. O `.env` não foi aberto; apenas nomes de variáveis do `.env.example` foram considerados quando necessário.

Arquivos de referência lidos, entre outros: `apps/server/src/{main,routes,db,auth,limits,storage,config,blocking}.rs`, `apps/server/src/integration_db.rs`, `apps/server/migrations/*.sql`, `apps/server/{Cargo.toml,README.md,Dockerfile}`, `apps/desktop/src/{auth-web,web-api,web-auth-store,oauth-callback,model,aiCase,App}.tsx|ts`, componentes de relatório/IA, `docs/{WEB_ARCHITECTURE,ARCHITECTURE,PRIVACY}.md`, `README.md`, workflows e `Cargo.toml`.

Contagens históricas como 655/2/248/38/1 do baseline `4e73c32` são **relatos de execução anterior**, não prova do HEAD atual e não foram reexecutadas. Os 14 testes do scanner M1.2 e as 52 autoverificações pertencem à fase anterior; não cobrem cadastro, banco, S3, Cognito ou navegador.

## 3. Etapa 1 — Testes de integração

### 3.1 Os 12 caminhos Postgres (10 em `integration_db` + 2 em `routes`)

Todos usam `#[cfg(test)]` e `BancoTemporario`, que cria schema `tmp_teste_previdenciar_<uuid>` e aplica as migrações. Sem banco, `banco_ou_pular!` imprime aviso e **retorna**; o teste é contado como sucesso. Os dois testes de `routes` usam `test_db()` e fazem `return` quando não há banco.

| # | Teste | Arquivo:linha | Tipo | Banco real | Cobertura principal |
|---:|---|---|---|---|---|
| 1 | `usuario_e_workspace_persistem_entre_chamadas` | `apps/server/src/integration_db.rs:244` | DB direto (chama `db::ensure_user`, `db::bootstrap_workspace`, `db::save_workspace`; não usa o router) | sim | usuário, e-mail, bootstrap sequencial, revisão |
| 2 | `save_workspace_recusa_revisao_antiga_pelo_http` | `apps/server/src/integration_db.rs:318` | rota real `PUT /api/workspace` + DB | sim | CAS 409, ausência de gravação órfã |
| 3 | `workspace_de_um_usuario_nao_vaza_para_outro` | `apps/server/src/integration_db.rs:381` | rota real + DB | sim | isolamento por `owner_id` |
| 4 | `auditoria_de_login_nao_guarda_dados_pessoais` | `apps/server/src/integration_db.rs:440` | rota real `/api/me` + DB | sim | auditoria de login sem PII |
| 5 | `auditoria_admin_respeita_o_teto_de_200_eventos` | `apps/server/src/integration_db.rs:476` | rota admin + DB | sim | limite 200 |
| 6 | `admin_users_respeita_o_teto_de_500_usuarios` | `apps/server/src/integration_db.rs:514` | rota admin + DB | sim | limite 500 |
| 7 | `reports_saved_ignora_expirados_e_respeita_o_teto_de_100` | `apps/server/src/integration_db.rs:548` | rota real + DB | sim | expiração e limite 100 |
| 8 | `admin_entitlement_atualiza_o_usuario_alvo_no_banco` | `apps/server/src/integration_db.rs:624` | rota admin + DB | sim | upsert e 404 |
| 9 | `aceite_de_termos_e_idempotente_e_aparece_no_perfil` | `apps/server/src/integration_db.rs:692` | rota + DB | sim | PK de termos |
| 10 | `auditoria_nao_expoe_chave_de_api_do_workspace` | `apps/server/src/integration_db.rs:738` | rota + DB | sim | não vazamento em auditoria/outro usuário; registra chave em claro no workspace |
| 11 | `cadastro_novo_respeita_limite_por_ip` | `apps/server/src/routes.rs:1582` | rota real `/api/me` + DB | sim | limite de cadastro por IP |
| 12 | `admin_com_permissao_lista_usuarios` | `apps/server/src/routes.rs:1616` | rota admin + DB | sim | admin lista usuários |

Dos 12, **11 passam por router/HTTP com Postgres real** (9 em `integration_db` + 2 em `routes`) e **1 é teste direto de DB** (`usuario_e_workspace...`, sem router). Além desses, existe **um teste independente com pool lazy** que não usa banco real: `erro_de_banco_nao_vaza_detalhe_tecnico_para_o_cliente` (`apps/server/src/integration_db.rs:833`). Ele é o “pool lazy extra” e não conta como um dos 12 caminhos Postgres.

### 3.2 Fluxos realmente existentes

| Fluxo | O que existe | Onde | Cobertura de teste | Lacuna |
|---|---|---|---|---|
| Cadastro | Cognito hosted UI; `/api/auth/token` troca code; primeiro `/api/me` ou `/api/bootstrap` cria usuário | `apps/desktop/src/auth-web.tsx:24-56`; `apps/server/src/routes.rs:136-171,303-324`; `db.rs:6-17` | `usuario_e_workspace...`, `cadastro_novo_respeita_limite_por_ip` | Sem Cognito real nem mock; sem teste do code exchange |
| Login | Front com PKCE/state; refresh; `/api/me` | `auth-web.tsx:19-56`; `oauth-callback.ts`; `routes.rs:178-216` | `auth_invalido_retorna_401_json`, `rate_limit_de_login...` | Sem emissor Cognito real; só 401 e 429 |
| Entitlement | `entitlements.advanced`; `/api/me` devolve; admin altera | `0002_platform.sql:1-5`; `db.rs:88-109`; `routes.rs:433-467` | `admin_entitlement...` | Nenhuma rota consome `advanced` como autorização; sem expiração |
| Termos | `terms_acceptances`; `/api/terms/accept`; front bloqueia antes do app | `0002_platform.sql:7-12`; `db.rs:112-119`; `routes.rs:359-383`; `auth-web.tsx:58-80` | `aceite_de_termos...` | Servidor não exige aceite nas demais rotas; gate é do front |
| Workspace CAS | `PUT /api/workspace`; revisão; 409 | `db.rs:43-78`; `routes.rs:705-736` | `save_workspace_recusa...`, `workspace_de_um_usuario...` | Teste de conflito é sequencial; não há teste concorrente real |
| Clientes/casos | Arrays `clients`, `calculations`, `source_documents`, `readings`, `facts` no JSONB do workspace; sem tabela normalizada | `apps/desktop/src/model.ts:22`; `db.rs:60-70`; `routes.rs:705-736` | Indiretamente `usuario_e_workspace...` e CAS | Sem validação de esquema por entidade no servidor; todo o documento é gravado |
| CNIS | Upload multipart, extração Rust, OCR local, sem persistência | `routes.rs:804-937`; `crates/previdenciar-cnis` | Testes da crate; teste de limite/413 | Sem teste da rota feliz com PDF sintético e sem teste de contrato com DB |
| Salários/cálculo | `POST /api/calculate` recalcula no servidor com `previdenciar-rgps` | `routes.rs:743-776` | Testes de domínio nas crates | Sem teste de contrato HTTP que compare cálculo e PDF |
| Relatórios PDF | `preview`, `save`, `saved`, `{id}`, `delete`; S3 privado | `routes.rs:499-698`; `storage.rs` | `reports_saved...`; `upload_s3_sem_configuracao...` (503) | Sem S3 real/MinIO; `save`/`url`/`delete` não são exercitados com storage |
| IA | `ai/status`, `ai/chat` SSE; consentimento na crate; chave da plataforma | `routes.rs:218-293`; `crates/previdenciar-ai/src/agent.rs:199-300` | `copiloto_sse_entrega_evento_erro` (provedor falso) | Sem provedor real; `aiCancel` web é no-op; sem teste de consentimento falso |
| Admin | users, entitlement, audit | `routes.rs:385-497` | 3 testes de integração | Sem teste de role/expiração; admin por e-mail |
| Checkout/assinatura/webhook | **Não existe** | busca por `stripe`, `checkout`, `subscription`, `billing`, `webhook`, `payment_intent` em `apps`, `crates`, `server` e docs não encontra implementação | — | Não inventar Stripe/Supabase/RLS; não há fluxo |
| Equipes/papéis | Tabela `workspace_members` existe; nenhuma rota a usa | `0001_initial.sql:24-30`; busca em `apps/server/src` não encontra uso | — | Documentação fala em equipes opcionais, mas não há API; não confundir DDL com runtime |
| S3 de CNIS/exports | Documentação cita `cnis/` e `exports/`; código só grava `reports/` | `docs/PRIVACY.md:41-46`; `routes.rs:544` | — | Divergência documento/código; CNIS fica em memória/temp; `exports` não é S3 |

### 3.3 CI, comando único e execução real

- `cargo test --workspace --locked` aparece em `.github/workflows/ci.yml:56`, `.github/workflows/web.yml:27` e `.github/workflows/release.yml:40`.
- Nenhum desses jobs declara `services: postgres`, `TEST_DATABASE_URL` ou um passo que suba banco. A busca por `postgres`, `services` e `TEST_DATABASE_URL` nos workflows não encontra nada.
- `apps/server/README.md:55` confirma: sem banco, os testes imprimem `[pulado]` e retornam. `banco_ou_pular!` (`integration_db.rs:76-90`) e `test_db()` (`routes.rs:1337-1345`) fazem isso.
- Consequência: o CI pode ficar verde sem exercitar nenhum dos 12 caminhos. **Não existe hoje comando único fail-closed de integração.** O `cargo test -p previdenciar-web-server --locked -j 2` é apenas o comando **documentado historicamente** em `apps/server/README.md:49-53`, não uma recomendação para agora. Qualquer execução futura deve usar Cargo com `-j 1` e controle de orçamento aprovado; o script/CI fail-closed que suba Postgres descartável, exporte `TEST_DATABASE_URL` e derrube o banco continua **faltante** (`README.md:57-71` descreve o processo manual, mas não há script).
- `TEST_DATABASE_URL` tem precedência e **não** passa pela checagem de loopback; só `DATABASE_URL` passa (`integration_db.rs:95-108`). O helper ainda executa `CREATE EXTENSION IF NOT EXISTS pgcrypto` (`:170`) e `sqlx::migrate!` (`:174`). Se a variável apontar para um banco que não seja descartável, há escrita de extensão e migração, além do schema temporário. Isso viola o requisito de banco isolado.
- Testes executados nesta auditoria: **nenhum**. Não havia banco isolado autorizado, e Cargo está bloqueado; não se declara “verde” para os 12 caminhos.
- Separação de tipos de prova: os testes de domínio puro ficam nas crates (`previdenciar-rgps`, `previdenciar-cnis`, `previdenciar-reports`, `previdenciar-workspace`) e não provam HTTP/DB; os testes de navegador/E2E usam fixtures IPC sintéticas e não foram executados, portanto não são runtime web; os 12 caminhos acima são os únicos que tentam router + Postgres real.

### 3.4 Achados da etapa 1

| ID | Achado | Risco | Arquivo:linha | Correção proposta | Status |
|---|---|---|---|---|---|
| A1.1 | Testes Postgres pulam sem banco e contam como sucesso | Alto (falso verde) | `integration_db.rs:76-90`; `routes.rs:1337-1345` | Falhar antes de tocar dados; não criar novo `#[ignore]` nem pulo; job separado com banco obrigatório | Comprovado por código |
| A1.2 | `TEST_DATABASE_URL` aceita host remoto; `CREATE EXTENSION`/migrações rodam nesse banco | Alto (operação) | `integration_db.rs:95-108,170-177` | Exigir loopback ou autorização explícita; nunca usar banco de produção | Comprovado por código; não executado |
| A1.3 | CI não sobe Postgres; `cargo test` não cobre os 12 caminhos | Médio/alto | workflows citados | Serviço Postgres isolado + `TEST_DATABASE_URL`; job separado | Comprovado por workflow |
| A1.4 | Login/cadastro Cognito não têm teste de contrato; só 401 e rate limit | Médio | `routes.rs:1347-1479`; `auth-web.tsx` | Mock de JWKS/claims e teste de code exchange com provedor falso | Comprovado por ausência |
| A1.5 | `save_report`/`saved_report_url`/`delete` não testam S3 real; só 503 sem storage e listagem sintética | Médio | `routes.rs:1522-1555`; `integration_db.rs:548-618` | MinIO/S3 fake local no job de integração; teste de URL e delete | Comprovado por ausência |
| A1.6 | Entitlement e termos não têm teste de autorização porque a autorização não existe | Alto (liga R3) | `routes.rs:335-350`; `docs/WEB_ARCHITECTURE.md:18` | Definir guarda e testá-la | Comprovado por código |
| A1.7 | CNIS/salários têm testes de parser, mas não teste de contrato da rota até o cálculo | Médio | `routes.rs:804-937`; `crates/previdenciar-cnis` | Fixture sintética + rota + cálculo + PDF em teste de integração | Limitação de cobertura |
| A1.8 | Os testes existentes de workspace são sequenciais; não há concorrência real | Alto (liga R1) | `integration_db.rs:244-434` | Teste com duas conexões/duas tarefas e uma constraint | Comprovado por leitura |

### 3.5 Frases propostas para a etapa 1 (PROPOSTAS — não implementadas)

Todas são propostas para aprovação humana. Nenhum teste foi criado nesta auditoria.

1. **[PROPOSTA] Quando** duas requisições `GET /api/bootstrap` do mesmo `sub` chegarem antes de existir workspace, **então** o banco conterá exatamente um workspace e as duas respostas trarão o mesmo `id`.
2. **[PROPOSTA] Quando** duas sessões enviarem `PUT /api/workspace` com a mesma revisão, **então** exatamente uma receberá 200 e a outra 409; o documento final será o da vencedora e não haverá gravação parcial.
3. **[PROPOSTA] Quando** um usuário tentar ler ou apagar relatório de outro `owner_id`, **então** a resposta será 404 e nenhum metadado ou URL será devolvido.
4. **[PROPOSTA] Quando** a execução de integração ocorrer sem banco isolado válido, **então** ela falhará antes de tocar dados, sem aprovação aparente ou pulo; os 12 caminhos só contarão quando executados.
5. **[PROPOSTA] Quando** `TEST_DATABASE_URL` apontar para host que não seja loopback, **então** o helper recusará iniciar sem uma autorização explícita e não executará `CREATE EXTENSION` nem migração.
6. **[PROPOSTA] Quando** o CI executar o job de servidor, **então** um Postgres descartável estará disponível e os 12 caminhos serão efetivamente executados; o job não poderá ficar verde apenas com skips.
7. **[PROPOSTA] Quando** `POST /api/reports/save` for chamado sem storage, **então** responderá 503 antes de renderizar e não criará linha nem objeto.
8. **[PROPOSTA] Quando** o CNIS sintético for enviado por `POST /api/cnis/import`, **então** a resposta preservará salários e vínculos e o cálculo seguinte usará os mesmos valores, sem inventar dado ausente.

## 4. Etapa 2 — Confiança no cliente

### 4.1 Cadeia de autenticação e autorização

- **JWT/Cognito**: o servidor valida RS256, `aud` = `COGNITO_CLIENT_ID`, `iss` = emissor Cognito e JWKS com cache de 1 h (`apps/server/src/auth.rs:41-99`). O front usa PKCE, `state` de 32 bytes e `sessionStorage` para a tentativa; compara o `state` antes de trocar o code (`apps/desktop/src/auth-web.tsx:19-41`; `oauth-callback.ts:56-119`). Isso está bem encaminhado.
- **Dev header**: `AUTH_DISABLED` só é aceito com `APP_ENV` development/test (`config.rs:68-80`); em produção a flag é ignorada. O CORS só libera `x-dev-user` nesse caso (`main.rs:80-100`). O front também tem `VITE_AUTH_DISABLED`, mas isso só pula a tela; a API continua validando (`auth-web.tsx:114-118`).
- **Owner**: as rotas de workspace/relatórios derivam o dono do token (`ensure_user_guarded`), não do corpo (`routes.rs:1087-1112`; consultas com `owner_id` em `routes.rs:550-557,601-604,641-649,671-679`). Não há `owner_id` fornecido pelo cliente.
- **CAS**: o workspace usa revisão otimista com `UPDATE ... WHERE revision = $4` (`db.rs:43-78`). O teste existente prova o 409 sequencial; a atomicidade do `UPDATE` é a proteção de banco.
- **Admin**: `is_admin` compara o e-mail recebido do token com `ADMIN_EMAILS` em minúsculas (`config.rs:164-167`); `require_admin` aplica antes de users/entitlement/audit (`routes.rs:385-397`). Não há papel em banco, expiração ou trilha de concessão. O front oficial usa **ID token** com scope `openid email profile` (`auth-web.tsx:30-38`) e o servidor valida JWKS/issuer/audience; a hipótese de R2 é sobre as claims desse ID token, **não** sobre um access token.
- **Entitlement**: existe a coluna `advanced` e o admin pode alterá-la (`0002_platform.sql:1-5`; `routes.rs:433-467`), mas nenhuma rota a usa como autorização. `/api/me` só a devolve (`routes.rs:335-350`). A documentação promete `previdenciario:advanced` (`docs/WEB_ARCHITECTURE.md:18`); a busca em `apps/desktop/src` não encontra `advanced`. É uma promessa não implementada, não um gate de cliente.
- **Termos**: o front chama `/api/me`, vê `terms.accepted` e mostra a tela de aceite (`auth-web.tsx:58-80`). O servidor grava o aceite com PK (`db.rs:112-119`), mas não exige o aceite nas rotas de cálculo, workspace, relatório, CNIS ou IA. Um cliente direto pode usar a API sem aceitar. Isso é compliance/UX, não vazamento entre usuários.
- **Expiração**: `entitlements` tem apenas `updated_at`; não há `expires_at` nem plano. Um `advanced=false` só muda por ação administrativa. Não existe checkout/assinatura; não há como expirar automaticamente. Isso é lacuna de produto, não bug de segurança.

### 4.2 Matriz de confiança servidor × cliente

| Controle | Verificação no servidor | Dependência do cliente | Situação |
|---|---|---|---|
| Identidade | JWT/JWKS/issuer/audience | Login Cognito; storage do token | Servidor valida; front guarda token em `localStorage` |
| Dono do workspace | `owner_id` derivado do token | Nenhuma escolha do cliente | Correto |
| Revisão/CAS | `WHERE revision` atômico | Enviar a revisão correta | Correto; conflito 409 |
| Admin | `ADMIN_EMAILS` + e-mail do token | Cognito e claim de e-mail | Depende de e-mail verificado; R2 |
| Entitlement | Não aplicado | Nenhum gate no front | **Não implementado** |
| Termos | Grava aceite | Front bloqueia | **Não exigido nas rotas** |
| Limites de upload | 64 MiB corpo, 32 MiB CNIS, 64 MiB caso | Escolha do arquivo | Servidor aplica |
| Limites de IA/S3 | Rate limit por e-mail; sem quota global | Nenhum | Parcial; ver A2.9 |
| Caminho S3 | `reports/{owner}/{uuid}.pdf` | Nenhum path do cliente | Correto |
| Redownload | `owner_id` + `expires_at` | Nenhum | Correto |
| Cálculo | Recalculado no servidor; PDF usa o mesmo renderer | Entrada do cliente | Servidor não confia em resultado salvo |
| IA consentimento | `consent` obrigatório na crate | Checkbox do front | Servidor aplica o booleano; não prova consentimento humano |
| IA minimização | `case_context` limitado a 30k chars; sem validação de PII | `caseSummary`/anexo revisado | Minimização é do front; cliente direto pode enviar mais |
| Senha/chave | Não há senha no app; Cognito | — | Chave da Fábrica no workspace é o problema R4 |

### 4.3 Segredos, identificadores públicos e armazenamento no front

- **Identificadores públicos Cognito**: `apps/desktop/src/auth-web.tsx:9-10` tem fallback hardcoded para o client ID e o domínio do pool. São identificadores públicos do cliente, não credenciais; a existência não exige rotação. Não há `client_secret` no front. O histórico tem 40 achados e as 3 exceções de política; isso não significa 40 segredos e não foi reaberto nesta auditoria.
- **`VITE_*`**: o front usa `VITE_COGNITO_CLIENT_ID`, `VITE_COGNITO_DOMAIN`, `VITE_COGNITO_GOOGLE` e `VITE_AUTH_DISABLED`. Nenhum é chave secreta. Não há `VITE_*` de API key no código.
- **Tokens**: ID token e refresh token ficam em `localStorage` (`web-auth-store.ts:31-48`). O refresh token é de vida longa; XSS poderia exfiltrá-lo. A CSP `script-src 'self'` reduz a superfície, mas não elimina o risco. Alternativa mais segura seria cookie `HttpOnly`/BFF, com decisão de arquitetura; não é correção de uma linha.
- **Refresh antigo**: `setSession` só grava refresh quando ele vem; se um novo login não trouxer refresh, o valor antigo permanece (`web-auth-store.ts:40-43`). Em troca de conta no mesmo navegador, isso pode confundir sessões. Proposta: limpar no início do login ou remover quando ausente.
- **Chave da Fábrica**: o front tem campo `fabrica_api_key` no workspace (`model.ts:22`) e o envia no `save` (`App.tsx:21,54`). O servidor grava o documento inteiro em JSONB (`db.rs:60-70`). O teste `auditoria_nao_expoe_chave...` confirma que a chave não aparece em auditoria/users/outro workspace, mas registra que ela está em claro no documento (`integration_db.rs:800-817`). É o R4.
- **Proxy local TRF3**: `server/index.js` é um proxy Node local que lê `FABRICA_API_KEY` do ambiente; não é o servidor web oficial e não foi executado. O caminho web `webApi.conferirFabricaRmi` é `unsupported` (`web-api.ts:116`). Não há chave da Fábrica no bundle web.
- **`.env`**: não foi aberto. O `.env.example` só foi usado para nomes. Nenhum valor de credencial foi lido ou reproduzido.

### 4.4 PostgreSQL, RLS e exposição

- **Supabase/RLS**: não há Supabase, nem policies RLS, nem client DB no código. A única migração cria tabelas e índices; nenhum `ENABLE ROW LEVEL SECURITY` aparece. A busca por `supabase` e `RLS` não encontra uso.
- **Acesso**: todo acesso a Postgres é do backend (`sqlx`); o banco não é exposto por rota. As consultas de workspace/relatórios filtram `owner_id`; as consultas admin são deliberadamente amplas e ficam atrás de `require_admin`.
- **Ausência de RLS**: como não há acesso direto do navegador ao banco, a ausência de RLS **não é automaticamente uma porta aberta**. O risco seria se o banco fosse exposto ou se uma rota esquecesse o filtro; não foi encontrado vazamento de workspace entre usuários nos testes existentes. Ainda assim, RLS não é defesa em profundidade aqui; a proteção é o filtro de backend.
- **Equipes**: `workspace_members` existe (`0001_initial.sql:24-30`), mas não há consulta/rota. Não há RLS nem checagem de papel; a funcionalidade de equipe não existe no runtime atual. Não confundir DDL com comportamento.
- **Estado real**: nenhuma conexão com banco de produção, staging ou local foi aberta. Não se afirma estado de RLS, grants, roles ou dados no banco real. Nada foi limpo.

### 4.5 Achados da etapa 2

| ID | Achado | Risco | Arquivo:linha | Correção proposta | Status |
|---|---|---|---|---|---|
| A2.1 | Admin por e-mail; `email_verified` ausente não reprova; `username` pode virar e-mail | Alto (condicional) | `auth.rs:59-78`; `config.rs:164-167`; `routes.rs:385-397` | Exigir `email_verified === true` quando houver e-mail; nunca usar `username` para autorização; mapear admin por `sub` ou e-mail verificado | Hipótese condicionada |
| A2.2 | `advanced` não é aplicado em nenhuma rota | Alto (produto/abuso) | `routes.rs:335-350`; `docs/WEB_ARCHITECTURE.md:18` | Definir lista de rotas avançadas e guarda server-side; testar 403 | Comprovado por código |
| A2.3 | Termos não são exigidos nas rotas; só o front bloqueia | Médio (compliance) | `routes.rs:359-383`; `auth-web.tsx:58-80` | Exigir aceite da versão vigente nas rotas de negócio, com resposta 403 e mensagem própria | Comprovado por código |
| A2.4 | Chave da Fábrica em claro em `workspaces.document` | Alto/médio (segredo em repouso) | `db.rs:60-70`; `model.ts:22`; `integration_db.rs:800-817` | Remover do documento e guardar em cofre/KMS/coluna cifrada; migrar com revisão humana | Comprovado pelo teste |
| A2.5 | ID/refresh token em `localStorage` | Médio (XSS) | `web-auth-store.ts:31-48` | Avaliar cookie `HttpOnly`/BFF; manter CSP; rotacionar refresh | Comprovado por código |
| A2.6 | Refresh antigo permanece em novo login sem refresh | Médio/baixo (confusão de sessão) | `web-auth-store.ts:40-43` | Limpar tokens no início do login e remover refresh quando ausente | Comprovado por código |
| A2.7 | `/api/me` grava auditoria a cada chamada e não tem rate limit para usuário conhecido | Médio (crescimento de DB) | `routes.rs:326-354,1087-1112` | Limitar/ deduplicar auditoria de login por janela ou não gravar em cada `me` | Comprovado por código |
| A2.8 | `save_workspace` e `terms/accept` também não têm rate limit; workspace aceita até 64 MiB | Médio (abuso de storage/DB) | `routes.rs:29,359-383,705-736` | Rate limit por usuário e quota de documento; decidir limite de produto | Comprovado por código |
| A2.9 | Relatórios: 20/hora por e-mail, sem quota global nem limite de tamanho do PDF gerado | Médio (custo S3) | `routes.rs:78-82,518-590` | Quota por conta e/ou global; limite de bytes de saída; alerta de custo | Comprovado por código |
| A2.10 | Falhas de auditoria são ignoradas (`let _ = db::audit`) em ações sensíveis | Médio (trilha incompleta) | `routes.rs:338-347,372-381,456-465,573-582,687-696,725-734,765-774` | Auditar na mesma transação da ação ou registrar falha de forma explícita | Comprovado por código |
| A2.11 | `ensure_user` faz upsert só por `cognito_sub`; colisão de e-mail único vira 500 | Médio/baixo (identidade) | `db.rs:6-17`; `0001_initial.sql:3-10`; `routes.rs:1109-1111` | Tratar colisão de e-mail com erro de domínio e decisão de vinculação; não sobrescrever | Comprovado por código |
| A2.12 | RLS ausente, mas banco não é exposto; `workspace_members` não é usado | Limitação | `0001_initial.sql:12-30`; busca sem uso | Se houver acesso direto futuro, RLS/roles; hoje, manter filtros de backend e testes | Limitação; runtime não verificado |
| A2.13 | Fallback público de client ID/domínio Cognito no front | Informativo | `auth-web.tsx:9-10` | Manter como identificador público; não tratar como segredo nem exigir rotação por existência | Comprovado; não é achado de segredo |
| A2.14 | S3 de CNIS/`exports` citado em docs, mas só `reports/` é gravado | Baixo (documentação) | `docs/PRIVACY.md:41-46`; `routes.rs:544` | Corrigir a documentação ou implementar o ciclo de vida; decidir retenção com revisão humana | Divergência comprovada |

| A2.15 | Strings de mensagem em `crates/previdenciar-ai/src/agent.rs` nas linhas 24, 48, 215, 283, 674 e 808 apresentam mojibake/encoding corrompido (observação baixa; não corrigido e não reproduzido). | Baixo (qualidade de mensagem) | `crates/previdenciar-ai/src/agent.rs:24,48,215,283,674,808` | Limpeza futura de strings, sem alterar contrato, lógica ou segurança; não corrigir nesta fase | Observação estática; leitura mínima sem exibir as strings |

### 4.6 Frases propostas para a etapa 2 (PROPOSTAS — não implementadas)

1. **[PROPOSTA] Quando** o ID token não tiver `email_verified=true`, **então** o servidor não usará `email` para autorização administrativa e responderá 403 em `/api/admin/*`. A escolha entre admin por `sub` ou por e-mail verificado é pergunta de produto; esta frase só descreve o comportamento a aprovar, não escolhe a política mecanicamente.
2. **[PROPOSTA] Quando** o token tiver `email` ausente, **então** o servidor usará apenas `sub` como identidade e não `username` como e-mail.
3. **[PROPOSTA] Quando** `advanced=false`, **então** as rotas avançadas definidas pelo produto responderão 403, enquanto `/api/me` continuará 200.
4. **[PROPOSTA] Quando** os termos da versão vigente não estiverem aceitos, **então** as rotas de negócio responderão 403 com mensagem própria, e `/api/me` continuará informando o estado.
5. **[PROPOSTA] Quando** o workspace contiver `fabrica_api_key`, **então** o documento persistido não guardará o valor em claro; a leitura do dono continuará funcionando por cofre/campo cifrado.
6. **[PROPOSTA] Quando** o usuário trocar de conta no mesmo navegador, **então** o refresh token da conta anterior não será reutilizado e a sessão nova não será substituída pela antiga.

## 5. Etapa 3 — Concorrência

### 5.1 Mapa de escritas e decisões

| Operação | Sequência | Proteção existente | Resultado esperado | Situação |
|---|---|---|---|---|
| `ensure_user` | `INSERT ... ON CONFLICT (cognito_sub) DO UPDATE` | Unique `cognito_sub`; unique `lower(email)` | Um usuário por `sub`; colisão de e-mail vira erro | Protegido para `sub`; colisão de e-mail não tratada |
| `bootstrap_workspace` | `SELECT` (0..1) + `INSERT` | Nenhuma unique em `owner_id`; sem transação | Um workspace por usuário | **Corrida R1** |
| `save_workspace` | `SELECT revision` + `UPDATE ... WHERE revision` | `WHERE id AND owner_id AND revision`; retorno `None` → 409 | Um vencedor; sem lost update | Protegido por código; teste sequencial |
| `accept_terms` | `INSERT ... ON CONFLICT DO NOTHING` | PK `(user_id, version)` | Uma linha; repetição inofensiva | Protegido |
| `admin_entitlement` | `INSERT ... ON CONFLICT (user_id) DO UPDATE` | PK `user_id` | Última escrita administrativa vence | Idempotente por efeito; sem CAS/expiração |
| `save_report` | `Uuid::new_v4` + `S3 put` + `INSERT` | PK `id`; nenhuma chave de operação | Duas chamadas = dois objetos/linhas | **Não idempotente** |
| `delete_saved_report` | `S3 delete` + `DELETE` | `owner_id`; S3 delete idempotente | Repetição tende a 404 após a primeira | Não atômico entre S3 e DB |
| `calculate` | cálculo puro + auditoria | Nenhuma escrita de negócio | Resultado igual; auditoria por chamada | Puro; auditoria sem rate |
| `import_cnis` | parser + temp file | Tempfile; sem persistência | Sem escrita de negócio | Puro por requisição |
| `ai_chat` | SSE + provedor externo | Rate limit por e-mail; `request_id` só validação | Duas chamadas = duas cobranças | Não idempotente; sem cancel server-side |
| `me` | `ensure_user` + `session_info` + auditoria | Nenhum rate para usuário conhecido | N chamadas = N eventos de auditoria | Efeito colateral sem limite |

### 5.2 Achados e sequências concretas

**R1 — primeiro workspace.** Duas requisições `GET /api/bootstrap` do mesmo `sub`, sem workspace:
1. A e B executam `SELECT ... WHERE owner_id = $1 ORDER BY created_at ASC LIMIT 1` e ambas recebem vazio (`db.rs:20-29`).
2. A e B executam `INSERT INTO workspaces (owner_id, document) ...` (`db.rs:32-39`); não há `UNIQUE(owner_id)` (`0001_initial.sql:12-22`).
3. Ambas recebem 200, com IDs diferentes. `save_workspace` depois pega o mais antigo (`db.rs:48-53`); a segunda base fica órfã e inalcançável, e pode haver `id` inconsistente entre a primeira resposta e a base usada depois. **Nenhuma rota aceita `workspace id`**, então não há fluxo API para gravar na segunda base; não há perda de conteúdo demonstrada.
4. Correção provável: unique em `owner_id` para o workspace pessoal (ou `is_personal` + índice parcial, se equipes forem reais), `INSERT ... ON CONFLICT` e `SELECT` na mesma transação/CTE. Migração exige decisão de produto e revisão humana.

**R2 — admin/e-mail.** O caminho é:
1. `auth.rs:68` só rejeita e-mail não verificado se `email_verified` **estiver presente** e for falso.
2. `auth.rs:71-74` usa `email` ou `username` ou `sub` como e-mail.
3. `config.rs:164-167` compara esse valor com `ADMIN_EMAILS`; `routes.rs:393-395` libera admin.
4. Se o ID token do emissor trouxer `username` igual a um e-mail de admin e `email_verified` ausente, a autorização administrativa pode ser obtida. A configuração do emissor é um pressuposto de política e não foi consultada; a decisão entre admin por `sub` ou por e-mail verificado é de produto. Correção proposta, a aprovar: exigir `email_verified=true` para qualquer decisão por e-mail; usar `sub` para admin; testar claims ausentes/falsas.

**R3 — entitlement.** O fluxo é:
1. `db::session_info` cria `entitlements` com `advanced=true` por padrão (`db.rs:88-109`).
2. `/api/me` devolve o valor (`routes.rs:335-350`).
3. Nenhuma rota lê `advanced` antes de `calculate`, `preview_report`, `save_report`, `import_cnis`, `ai_chat` ou `run_tool`.
4. O front não referencia `advanced`; a documentação fala em `previdenciario:advanced`. Hoje o valor é informativo. Correção: decidir quais rotas são avançadas e aplicar guarda server-side; testar 403.

**A3.3 — e-mail único.** `users` tem unique em `lower(email)` (`0001_initial.sql:10`), mas `ensure_user` só tem `ON CONFLICT (cognito_sub)` (`db.rs:6-17`). Se o mesmo e-mail chegar com outro `sub` ou se o e-mail mudar para um já usado, o insert/update viola a unique e a rota responde 500 genérico (`routes.rs:1109-1111`). Não há vinculação de contas. Correção: capturar a violação e devolver erro de domínio; decidir política de vinculação; não sobrescrever identidade.

**A3.6 — relatório duplicado.** `save_report` gera `Uuid::new_v4()` por chamada (`routes.rs:543`), grava o objeto S3 (`:545-548`) e insere a linha (`:558-572`). Duas chamadas iguais geram dois objetos e duas linhas; se o S3 responde e o `INSERT` falha, o objeto fica órfão. O `report_artifacts` tem PK `id`, mas nenhuma chave de operação (`0002_platform.sql:14-24`). Correção: chave de idempotência por operação lógica e estado `pending/ready/failed`, com revisão humana; não basta um `outbox` ingênuo.

**A3.7 — delete.** `delete_saved_report` apaga o objeto S3 (`routes.rs:680`) e depois a linha (`:681-686`). Se o DB falhar, a linha fica sem objeto; se o S3 falhar, a linha fica. O retry do S3 delete é idempotente, mas o contrato não é atômico. Correção: aceitar a limpeza eventual e documentar; ou inverter com estado, decidir com revisão.

**A3.9 — rate limit por processo.** `RateLimiter` é `HashMap` em memória (`limits.rs:41-44`) criado por processo (`main.rs:75`). Com duas instâncias, cada uma tem seu próprio contador; o limite de login/IA/PDF não é global. Isso é uma limitação conhecida; não propor Redis/fila sem decisão de infraestrutura. Se o deploy for uma instância, o comportamento é o esperado.

**A3.10 — rotas sem rate.** `LOGIN`, `CADASTRO`, `CNIS`, `OCR`, `CALC`, `TOOL`, `AI` e `PDF` têm janelas (`routes.rs:33-82`). `bootstrap`, `me`, `terms/accept` e `workspace` não têm. Um usuário autenticado pode repetir `me`/`terms`/`workspace`; `me` e `terms` inserem auditoria; `workspace` grava JSONB de até 64 MiB. Isso é abuso de DB/storage, não acesso a outro tenant.

**A3.11 — dinheiro.** O produto aqui é cálculo previdenciário puro; não há pedido, estoque, cupom, saldo ou transação financeira. Não se aplica o padrão de idempotência de pagamento. Não inventar Stripe/checkout. O único “dinheiro” é o valor calculado, que é determinístico e recalculado no servidor.

### 5.3 Contratos de erro observados

| Situação | Status | Efeito |
|---|---:|---|
| `save_workspace` com revisão antiga | 409 | Nenhuma gravação; cliente deve recarregar (`routes.rs:717-720`) |
| `save_report` sem S3 | 503 | Antes de renderizar; nenhum registro (`routes.rs:531-534`) |
| `save_report` com S3 ok e DB falhando | 500 | Objeto S3 pode ficar órfão; sem idempotência |
| `save_workspace`/`terms`/`admin` com auditoria falhando | 200 | Ação concluída sem trilha; erro ignorado |
| `me` com banco indisponível | 500 | JSON genérico; sem detalhe técnico (teste lazy) |
| Rate limit | 429 | `Retry-After`; por processo |
| Corpo/arquivo acima do limite | 413 | JSON; `MAX_BODY_BYTES`/`MAX_CNIS_PDF_BYTES` |
| `bootstrap` concorrente | 200/200 | Pode criar dois workspaces; sem 409 |

### 5.4 Frases propostas para a etapa 3 (PROPOSTAS — não implementadas)

1. **[PROPOSTA] Quando** duas chamadas concorrentes de bootstrap chegarem para o mesmo `sub`, **então** haverá exatamente um `workspaces.owner_id` e as duas respostas terão o mesmo `id`.
2. **[PROPOSTA] Quando** duas chamadas `PUT /api/workspace` usarem a mesma revisão, **então** uma terá 200 e a outra 409, sem perda de escrita.
3. **[PROPOSTA] Quando** duas chamadas `POST /api/reports/save` usarem a mesma chave de idempotência, **então** haverá um único objeto S3 e uma única linha, com a mesma resposta.
4. **[PROPOSTA] Quando** o S3 falhar depois do registro `pending`, **então** o retry com a mesma chave não duplicará e não deixará objeto órfão.
5. **[PROPOSTA] Quando** a auditoria falhar em uma ação administrativa, **então** a ação não será considerada concluída com sucesso ou a falha será registrada de forma explícita.
6. **[PROPOSTA] Quando** o servidor rodar em duas instâncias, **então** o limite de login/IA/PDF terá uma política definida (global ou por instância declarada), sem prometer limite global que o código não entrega.

## 6. Etapa 4 — Idempotência

### 6.1 Inventário de chamadas externas

| Canal | Existe? | Onde | Chave/assinatura | Situação |
|---|---|---|---|---|
| Webhook de entrada | **Não** | busca por `webhook`, `hmac`, `signature` não encontra rota | — | Não se aplica hoje |
| E-mail/SMTP/SES | **Não** | busca por `smtp`, `sendgrid`, `ses`, `mailgun` não encontra envio | — | Não se aplica |
| Fila/worker/gateway | **Não** | nenhuma fila; `BlockingGate` é semáforo local | — | Não adicionar fila |
| Cognito token/refresh | Sim | `routes.rs:148-160,195-205`; `auth.rs:87-99` | Code de autorização é de uso único; refresh token | Sem chave de idempotência; retry de code falha |
| Provedor de IA | Sim | `routes.rs:261-284`; `agent.rs:264-300` | `request_id` só validado e logado | Sem dedup; retry pode cobrar duas vezes |
| S3 relatórios | Sim | `storage.rs:23-59`; `routes.rs:545-548` | Nenhuma; chave aleatória | Put/delete; sem idempotência de operação |
| S3 conhecimento | Sim | `main.rs:50-61`; `storage.rs:61-80` | Nenhuma | GET no startup; sem assinatura |
| TRF3/Fábrica no web | **Não** | `web-api.ts:116` é `unsupported` | — | O proxy Node local e o Tauri não são o web |
| JWKS | Sim | `auth.rs:87-99` | Cache 1 h | GET idempotente; sem retry sofisticado |

Não há webhook, portanto não há “assinatura do corpo bruto antes do processamento” para auditar. Se um webhook for adicionado no futuro, a proposta é: ler o corpo bruto, validar assinatura/HMAC antes de parsear, gravar `received` com ID único e processar com estado; não implementar agora.

### 6.2 Operações e idempotência

| Operação | Repetição | Efeito | Classificação |
|---|---|---|---|
| `ensure_user` | Upsert por `sub` | Mesmo `id`; e-mail atualizado | Idempotente no efeito; colisão de e-mail é erro |
| `bootstrap_workspace` | Sequencial: mesma linha; concorrente: duas linhas | Pode duplicar | **Não idempotente sob concorrência** |
| `save_workspace` | Mesma revisão: 409; nova revisão: grava | Sem lost update, mas repetição não devolve sucesso | Seguro, não idempotente no sentido HTTP |
| `accept_terms` | PK; `DO NOTHING` | Uma linha; auditoria duplicada | Efeito idempotente; auditoria não |
| `admin_entitlement` | Upsert | Última escrita vence; auditoria duplicada | Efeito idempotente; sem CAS |
| `save_report` | Nova chamada | Novo UUID, novo S3, nova linha | **Não idempotente** |
| `delete_saved_report` | Segunda chamada | 404; S3 delete idempotente | Efeito quase idempotente; não atômico |
| `calculate` | Repetição | Mesmo resultado; auditoria nova | Puro; auditoria não |
| `import_cnis` | Repetição | Mesmo parse; temp descartado | Puro por requisição |
| `ai_chat` | Repetição | Nova chamada ao provedor; custo novo | **Não idempotente**; `request_id` não deduplica |
| `me` | Repetição | Auditoria nova | Efeito colateral não idempotente |

### 6.3 Achados da etapa 4

| ID | Achado | Risco | Arquivo:linha | Correção proposta | Status |
|---|---|---|---|---|---|
| A4.1 | Não há webhook/mail/gateway; não há assinatura a validar | Informativo | busca sem resultado | Não adicionar canal sem necessidade; se adicionar, assinatura antes do parse | Comprovado por ausência |
| A4.2 | `save_report` sem chave de operação | Médio (duplicata/custo/órfão) | `routes.rs:543-572`; `0002_platform.sql:14-24` | Chave de idempotência por operação + estado; migração revisada | Comprovado por código |
| A4.3 | `ai_chat` tem `request_id`, mas ele não deduplica | Médio (custo/duplicata) | `agent.rs:207-217`; `routes.rs:256-284` | Usar a chave para dedup/limite, se o produto quiser; não usar dados pessoais | Comprovado por código |
| A4.4 | `save_workspace` repetido com revisão antiga devolve 409, não sucesso idempotente | Baixo (UX) | `db.rs:43-78` | Documentar contrato; cliente recarrega; não “consertar” para 200 sem CAS | Comprovado por teste sequencial |
| A4.5 | `accept_terms`/`admin_entitlement` repetem auditoria | Baixo | `routes.rs:372-381,456-465` | Deduplicar auditoria por operação/janela ou aceitar como log de eventos | Comprovado por código |
| A4.6 | `delete_saved_report` não é atômico entre S3 e DB | Médio/baixo | `routes.rs:671-686` | Estado/retry idempotente; documentar 404 após sucesso | Comprovado por código |
| A4.7 | Nenhuma fila existente; não criar uma para “resolver” idempotência | Limitação | — | Preferir unique/estado no DB; não adicionar Redis/fila sem decisão | Decisão de arquitetura |
| A4.8 | Não há preço/plano/checkout para idempotência financeira | Não se aplica | busca sem resultado | Não inventar Stripe/assinatura | Comprovado por ausência |

### 6.4 Frases propostas para a etapa 4 (PROPOSTAS — não implementadas)

1. **[PROPOSTA] Quando** o cliente enviar a mesma `Idempotency-Key` duas vezes para `/api/reports/save`, **então** haverá um único registro e um único objeto S3, e a segunda resposta devolverá o mesmo `id`.
2. **[PROPOSTA] Quando** o `INSERT` do relatório falhar depois do S3, **então** o retry com a mesma chave retomará o estado sem criar outro objeto.
3. **[PROPOSTA] Quando** `POST /api/reports/save` for chamado sem storage, **então** responderá 503 antes de renderizar e não criará nada.
4. **[PROPOSTA] Quando** `PUT /api/workspace` repetir uma revisão antiga, **então** responderá 409 e o documento não mudará; a resposta não será confundida com sucesso idempotente.
5. **[PROPOSTA] Quando** `/api/terms/accept` for repetido, **então** a linha de aceite será uma e a auditoria terá uma política definida (uma por operação ou uma por chamada, declarada).
6. **[PROPOSTA] Quando** uma chamada de IA for repetida com o mesmo `request_id`, **então** o produto terá uma política explícita: reexecutar e cobrar de novo, ou deduplicar; hoje ele reexecuta.

## 7. Etapa 5 — Separação de responsabilidades

### 7.1 Candidato principal: `apps/server/src/routes.rs`

O arquivo tem cerca de 1.713 linhas e concentra responsabilidades que mudam por motivos diferentes:

| Bloco | Linhas aproximadas | Responsabilidade |
|---|---:|---|
| Constantes de rate limit | 29-82 | Política de abuso e mensagens |
| `router` | 84-127 | Composição de rotas e SPA |
| Auth Cognito | 136-216 | Troca de code/refresh e erros |
| IA/SSE | 218-293 | Autenticação, rate, bloqueio, streaming |
| Bootstrap/me/terms | 303-383 | Identidade, sessão, aceite |
| Admin | 385-497 | Autorização, consultas amplas, entitlement, auditoria |
| Relatórios PDF/S3 | 499-698 | Render, storage, CAS, presign, delete, auditoria |
| Workspace/calculate | 700-776 | CAS, cálculo, auditoria |
| CNIS/PDF de caso | 804-937 | Multipart, tempfile, OCR, sanitização de nome |
| Preview/prepare/tools/finance | 955-1081 | Render e domínio |
| Rate/ensure/erros | 1083-1250 | Infra de request e envelope |
| Testes | 1252-1713 | 11 testes de rota, 2 com DB |

Dependências cruzadas: o arquivo importa `auth`, `db`, `AppState`, `limits`, `storage` indireto, `previdenciar_*`, `aws` indireto, `sqlx`, `axum`, `reqwest`, `sha2`, `tempfile`, `uuid`, `tokio`. Um erro em `ApiError` ou no envelope pode afetar todas as rotas; um teste de admin pode importar helpers de DB; a leitura exige percorrer 1.700 linhas para separar authz de render.

Candidatos secundários:
- `apps/desktop/src/components/MinutaPage.tsx` (461 linhas) e `AiCopilot.tsx` (403): UI + consentimento + streaming + configuração; menor risco de segurança que o servidor, mas bom candidato depois.
- `apps/server/src/integration_db.rs` (870 linhas) é grande, mas coeso (testes de DB); não é o primeiro.
- `apps/server/src/db.rs` (144) é pequeno, mas concentra usuário/workspace/entitlement/terms/audit; pode ser dividido por domínio depois.

### 7.2 Proposta de decomposição em passos pequenos (sem executar)

1. **Caracterização primeiro**: rodar os testes existentes e adicionar (com OK) 5 testes de contrato que fixem status/JSON de workspace CAS, isolamento, admin 403, reports 503/409, rate 429. Nenhum bug deve ser corrigido no mesmo commit da refatoração.
2. **Extrair `routes/error.rs`**: mover `ApiError`, `IntoResponse`, `json_error_envelope`, `multipart_error`, `oversized_body_message`; manter as mesmas mensagens e status. Nenhuma rota muda.
3. **Extrair `routes/admin.rs`**: users, entitlement, audit, `require_admin`; importar `AppState`; testar 403/200.
4. **Extrair `routes/reports.rs`**: preview/save/saved/url/delete, `render_report_bytes`, `ReportRequest`; manter paths e corpos.
5. **Extrair `routes/workspace.rs`**: bootstrap, me, terms, save/calculate; manter owner/CAS.
6. **Extrair `routes/ingest.rs`**: CNIS, case PDF, prepare; manter limites e tempfiles.
7. **Extrair `routes/ai.rs`**: status/chat; manter SSE e consent.
8. **Mover os testes** para módulos de teste por domínio, sem mudar a contagem nem o comportamento.

Não renomear rotas, props, colunas ou campos JSON; não trocar framework; não introduzir crate nova; não corrigir bug junto com a movimentação.

### 7.3 Achados da etapa 5

| ID | Achado | Risco | Arquivo:linha | Correção proposta | Status |
|---|---|---|---|---|---|
| A5.1 | `routes.rs` mistura authz, DB, S3, PDF, IA, erros e testes | Médio/alto (manutenção e risco de regressão) | `apps/server/src/routes.rs:29-1713` | Decompor por caso de uso, com caracterização antes | Comprovado por leitura |
| A5.2 | `ApiError` e o envelope de erro vivem no mesmo arquivo das rotas | Médio | `routes.rs:1129-1250` | Extrair `error.rs` primeiro; mensagens/status congelados | Comprovado por leitura |
| A5.3 | Testes de rota e testes de DB no mesmo módulo | Baixo/médio | `routes.rs:1252-1713` | Mover para módulos de teste por domínio, mantendo fixtures | Comprovado por leitura |
| A5.4 | Front `MinutaPage`/`AiCopilot` concentram consentimento, streaming e UI | Médio (UX/segurança) | `MinutaPage.tsx`; `AiCopilot.tsx` | Separar hook de IA, painel de consentimento e apresentação; depois do servidor | Comprovado por leitura |

### 7.4 Frases propostas para a etapa 5 (PROPOSTAS — não implementadas)

1. **[PROPOSTA] Quando** `routes.rs` for dividido, **então** todas as rotas, métodos, status e corpos JSON permanecerão idênticos; os testes de caracterização passarão antes e depois.
2. **[PROPOSTA] Quando** um módulo for extraído, **então** nenhuma regra de negócio, autorização ou transação será alterada no mesmo passo.
3. **[PROPOSTA] Quando** os testes forem movidos, **então** a contagem e o resultado serão os mesmos, e nenhum teste de DB passará a pular silenciosamente.
4. **[PROPOSTA] Quando** a separação do front for feita, **então** o consentimento continuará explícito e o resumo de caso continuará omitindo identificação direta, rendimentos e textos livres.

## 8. Fluxos cobertos e comando único

**Coberto por leitura estática:** cadastro/login (Cognito + `/api/auth/token`), `/api/me`, bootstrap, workspace CAS, termos, entitlement/admin, CNIS, cálculo, relatórios/S3, IA/SSE, rate limits, erros, migrations, CI e front de autenticação/consumo.

**Não coberto ou não verificado:** runtime real de Postgres, Cognito, S3, IA e navegador; estado de RLS/grants; lifecycle S3; deploy AWS; comportamento do pool Cognito; dados reais; Tauri nativo; site de marketing separado; testes de browser/E2E.

**Comando documentado, não recomendado agora:** `cargo test -p previdenciar-web-server --locked -j 2` (`apps/server/README.md:49-53`) e `cargo test --workspace --locked` nos workflows são registros históricos. Se houver execução futura, usar Cargo com `-j 1` e controle de orçamento aprovado, sem tratar o `-j 2` como recomendação atual.

**Comando único faltante:** não existe hoje um comando único fail-closed de integração. Falta um script/CI que suba Postgres descartável, aplique as migrations, exporte `TEST_DATABASE_URL` e rode os 12 caminhos; um fake S3/MinIO para `save/url/delete`; um mock de Cognito/JWKS para login. Sem isso, “verde” não significa integração coberta. `npm --prefix apps/desktop run build:web`, `npm test` e `node scripts/review-public-source.mjs` existem, mas não substituem esse gate.

## 9. Prioridade de correção

**Ordem de continuidade, conforme o pedido do usuário:**

1. **Integração travando contratos confirmados** — harness fail-closed e os primeiros contratos sugeridos.
2. **Confiança no cliente**.
3. **Concorrência + idempotência**.
4. **Refatoração por último**.

**Caracterização observada × teste de regressão proposto.** São coisas diferentes:

- **Caracterização observada** registra o comportamento atual, inclusive os `return`/pulos históricos e os caminhos que hoje passam sem banco. Ela pode passar hoje e **não aprova** o comportamento nem o transforma em contrato desejado.
- **Teste de regressão de comportamento correto proposto** afirma o comportamento que ainda será aprovado; alguns falharão hoje — por exemplo, bootstrap com um único workspace, admin com e-mail verificado/`sub` e bloqueio por `advanced`. Não tratar a caracterização como aprovação de contrato.
- **Primeiros contratos sugeridos:** 1) bootstrap concorrente → um workspace; 2) CAS concorrente → 200/409 sem lost update; 3) isolamento de owner → 404 para outro usuário.
- **Isolamento do harness:** 4) sem banco isolado válido → falha antes de tocar dados; host não-loopback → bloqueado. Isso é requisito de execução, não contrato de produto.
- **ADMIN e `advanced` aguardam política e frases próprias** (seção 4.6); não entram nos três primeiros contratos e não foram implementados.
- Nenhum teste foi criado. A ordem R1→R2 não é imposta por este relatório; a exceção de contenção de R2 é **hipótese e proposta**, não execução. A separação de `routes.rs` fica por último, depois dos contratos e das correções que o usuário aprovar.

## 10. Propostas que dependem de OK e de revisão humana

| Proposta | Depende de | Observação |
|---|---|---|
| 3 primeiros contratos + isolamento do harness | OK do usuário | Nenhum teste novo foi criado; admin/`advanced` ficam para depois da política |
| Unique/transação de workspace (R1) | Decisão de produto (pessoal × equipe) + migration revisada | Não executar migration agora |
| Admin por `sub`/e-mail verificado (R2) | Decisão de identidade e Cognito | Pode exigir mudança de `ADMIN_EMAILS`; pergunta de produto, não escolha mecânica |
| Guarda de entitlement (R3) | Lista de rotas avançadas e política comercial | Sem preços/planos inventados |
| Cofre/cifra da chave da Fábrica (R4) | Decisão de cofre/KMS e migração de dados | Não expor/rotacionar automaticamente |
| Idempotência de relatório/IA (R5/A4) | Chave de operação, estado e migration | Sem fila/Redis nova |
| Trava de `TEST_DATABASE_URL` e CI com Postgres | OK operacional | Não criar container/serviço agora |
| Decomposição de `routes.rs` | Testes de caracterização aprovados | Sem renomear rotas/campos |

Migrações futuras são tarefas separadas, com revisão humana, backup e teste em banco isolado; nenhuma foi criada ou aplicada aqui.

## 11. Limitações e fontes

- Fontes são locais e relativas ao repositório; não há blog, estatística externa ou alegação de terceiro usada como prova.
- O relatório não reproduz tokens, chaves, identificadores reais, caminhos de perfil, dados de CNIS/PDF, valores de `.env` ou conteúdo de banco. Nomes de campos e caminhos relativos são usados apenas para localizar o código.
- O HEAD é local; não prova o histórico remoto, o deploy, o banco de produção, o bucket S3 ou o pool Cognito. Nenhum acesso de rede foi feito.
- O estado do banco, RLS, grants, lifecycle S3, Cognito e runtime web é **não verificado**. “Comprovado por código” não é o mesmo que “executado”.
- A primeira versão foi aprovada estaticamente em 03/11/2026 (`ses_e606c88a9ffeoRdLAc0xfCFtWa`), com R1–R5 verificados nas fontes e sem runtime. Esta versão com ajustes documentais foi submetida a nova revisão; **não há aprovação nova ainda**. A aprovação do relatório não é aprovação de implementação, runtime ou push.
- O relatório não foi commitado nem publicado e não possui hash git próprio; o HEAD de código permanece `f3c753e`. A barreira pública, o diff e a revisão independente devem preceder qualquer commit.
