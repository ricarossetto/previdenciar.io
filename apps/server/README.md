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

```powershell
cargo test -p previdenciar-web-server --locked -j 2
```

Use sempre `-j 2`: o paralelismo padrão do cargo estoura memória virtual ao linkar os binários de teste deste pacote.

Os testes de integração ficam em `src/integration_db.rs`. Cada um cria um schema temporário próprio (`tmp_teste_previdenciar_<uuid>`), aplica as migrações e apaga o schema no fim; o schema `public` não é tocado. Sem banco configurado eles imprimem um aviso `[pulado] ...` e retornam, em vez de falhar.

Para executar a persistência de verdade, suba um Postgres descartável e aponte `TEST_DATABASE_URL`:

```powershell
docker run -d --rm --name tmp-teste-previdenciar `
  -e POSTGRES_PASSWORD=dev `
  -e POSTGRES_DB=previdenciar `
  -p 127.0.0.1:55433:5432 postgres:16-alpine

$env:TEST_DATABASE_URL = "postgres://postgres:dev@127.0.0.1:55433/previdenciar"
cargo test -p previdenciar-web-server --locked -j 2 -- --nocapture

docker stop tmp-teste-previdenciar
```

O usuário do banco precisa de permissão para criar e apagar schema. Quando `TEST_DATABASE_URL` não existe, `DATABASE_URL` só é usada se apontar para loopback. Para inspecionar sobras de uma execução interrompida:

```sql
SELECT nspname FROM pg_namespace WHERE nspname LIKE 'tmp_teste_previdenciar_%';
DROP SCHEMA "tmp_teste_previdenciar_<uuid>" CASCADE;
```

## Banco

As migrações ficam em `migrations/`. O primeiro schema já separa `users`, `workspaces`, `workspace_members` e `audit_events`; o documento do workspace permanece em `jsonb` nesta fase para preservar o contrato do desktop. A normalização por caso virá junto com a migração das telas.
