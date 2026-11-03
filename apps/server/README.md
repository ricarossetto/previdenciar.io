# Servidor web

API Rust/Axum que reutiliza os crates do desktop. O frontend React é o mesmo de `apps/desktop`, compilado em modo web.

## Desenvolvimento local

1. Suba o PostgreSQL:

```powershell
docker run -d --rm --name previdenciar-pg `
  -e POSTGRES_PASSWORD=dev `
  -e POSTGRES_DB=previdenciar `
  -p 127.0.0.1:55432:5432 postgres:16-alpine
```

2. Crie o `.env` a partir do exemplo e use o banco local.
3. Compile o frontend web:

```powershell
npm --prefix apps/desktop run build:web
```

4. Inicie a API:

```powershell
cargo run -p previdenciar-web-server
```

O frontend fica em `http://127.0.0.1:8787/` e a API em `/api/*`.

## Endpoints iniciais

| Método | Rota | Função |
| --- | --- | --- |
| GET | `/api/health` | Saúde do processo |
| GET | `/api/bootstrap` | Workspace do usuário autenticado |
| PUT | `/api/workspace` | Salva o workspace com controle de revisão |
| POST | `/api/calculate` | Executa o motor RGPS |
| POST | `/api/prepare-from-cnis` | Prepara a hipótese a partir do CNIS |
| POST | `/api/tools/{tool}` | Ferramentas de apoio |
| POST | `/api/finance/{tool}` | Cálculos financeiros |

## Autenticação

`AUTH_DISABLED=1` só existe para desenvolvimento e só é aceita com `APP_ENV=development` ou `APP_ENV=test`; em produção (APP_ENV ausente, `production` ou valor desconhecido) a flag é ignorada com aviso no log e o header `x-dev-user` fica fora do CORS. Em produção, a API valida o JWT do Cognito pelo JWKS, com issuer e audience configurados. O frontend usa o modo web quando compilado com `vite build --mode web`.

## Testes

A partir da 1.4, o comando único é:

```powershell
node scripts/test-integration.mjs --check
node scripts/test-integration.mjs --run --ledger <json> --baseline <json> `
  --saga <dir> --target <dir> --repo <dir> [--temp <dir>] [--pg <dir>] `
  [--cache cargo=<dir>] --timeout-seconds 3600
```

- `--check` só valida `TEST_DATABASE_URL` e o plano; não executa Cargo, não abre banco e não imprime a URL.
- `--run` usa `scripts/run-with-disk-budget.mjs`, ledger persistente (sem reset/init), Cargo **offline**, `-j 1`, apenas `previdenciar-web-server`, e exige revisão prévia. Não use Docker/PG/Cargo antes dessa revisão.
- `TEST_DATABASE_URL` é obrigatória e explícita. Host literal `127.0.0.1` ou `::1`, porta explícita, usuário explícito e banco `previdenciar_test_<id>`. Não há fallback para `DATABASE_URL`, `.env` ou `localhost`.
- Host remoto, query/`host` override, multi-host, banco padrão ou porta ausente são recusados antes de conexão/SQL. Sem banco isolado válido o teste falha; não há `[pulado]`, `#[ignore]` ou retorno antecipado verde.
- Cada teste cria schema `tmp_teste_previdenciar_<uuid>`, aplica somente as migrações existentes `0001`/`0002` e apaga o schema; `public` não é tocado. Nenhuma migração nova é aplicada.
- `x-dev-user` com `APP_ENV=test` simula o emissor externo para as rotas; isso **não** cobre JWKS/Cognito real.
- O teste de isolamento de relatório usa um cliente S3 de teste local sem rede; não usa MinIO, download ou objeto real.

Os testes de integração ficam em `src/integration_db.rs`; os dois caminhos de `routes.rs` usam o mesmo helper. O teste concorrente de bootstrap é uma regressão **vermelha conhecida** até a correção do workspace; não trate a suíte como verde antes disso.

```sql
SELECT nspname FROM pg_namespace WHERE nspname LIKE 'tmp_teste_previdenciar_%';
DROP SCHEMA "tmp_teste_previdenciar_<uuid>" CASCADE;
```

## Banco

As migrações ficam em `migrations/`. O primeiro schema já separa `users`, `workspaces`, `workspace_members` e `audit_events`; o documento do workspace permanece em `jsonb` nesta fase para preservar o contrato do desktop. A normalização por caso virá junto com a migração das telas.
