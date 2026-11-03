//! Testes de integração contra Postgres real.
//!
//! Diferente dos `#[tokio::test]` de `routes.rs`, estes testes exercitam a
//! persistência de verdade: cada teste cria um schema temporário próprio
//! (`tmp_teste_previdenciar_<uuid>`), aplica as migrações de `migrations/` e,
//! no fim, apaga o schema com `CASCADE`. Nada é gravado no schema `public`.
//!
//! Quando não há banco configurado, o teste não falha: ele imprime um aviso
//! em `stderr` e retorna. Para rodar de verdade, defina `TEST_DATABASE_URL`
//! apontando para um Postgres local, por exemplo:
//!
//! ```text
//! TEST_DATABASE_URL=postgres://postgres:dev@127.0.0.1:55432/postgres \
//!   cargo test -p previdenciar-web-server --locked -j 2 -- --nocapture
//! ```
//!
//! Como alternativa, `DATABASE_URL` é aceita quando o host é de loopback, o
//! que cobre o `.env` de desenvolvimento do próprio projeto.
//!
//! Se um teste falhar no meio, o schema pode sobrar. A limpeza manual é:
//!
//! ```sql
//! SELECT nspname FROM pg_namespace WHERE nspname LIKE 'tmp_teste_previdenciar_%';
//! DROP SCHEMA "tmp_teste_previdenciar_<uuid>" CASCADE;
//! ```
//!
//! O usuário do banco precisa poder criar e apagar schema. A extensão
//! `pgcrypto` (usada por `gen_random_uuid()`) é criada no schema padrão; se o
//! usuário não puder criá-la e ela ainda não existir, as migrações falham de
//! propósito — isso é erro de ambiente, não motivo para pular o teste.

use crate::config::Config;
use crate::limits::RateLimiter;
use crate::{auth, blocking::BlockingGate, db, routes, AppState};
use axum::http::{header, HeaderMap};
use serde_json::{json, Value};
use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};
use sqlx::Row;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

/// Resultado da tentativa de abrir o banco de teste.
pub(crate) enum Banco {
    Pronto(BancoTemporario),
    Ausente(String),
}

/// Schema temporário isolado, com o pool apontando para ele.
///
/// O módulo é `cfg(test)`, então este auxiliary também serve aos testes de
/// `routes.rs`: nenhum teste deste pacote pode criar tabela no schema `public`.
pub(crate) struct BancoTemporario {
    pool: PgPool,
    admin: PgPool,
    schema: String,
}

impl BancoTemporario {
    /// Pool ligado ao schema temporário; some quando o `BancoTemporario` cai.
    pub(crate) fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub(crate) async fn descartar(self) {
        self.pool.close().await;
        let _ = sqlx::raw_sql(&format!("DROP SCHEMA IF EXISTS {} CASCADE", self.schema))
            .execute(&self.admin)
            .await;
        self.admin.close().await;
    }
}

/// Pula o teste com aviso quando o banco não estiver disponível.
macro_rules! banco_ou_pular {
    () => {
        match crate::integration_db::abrir_banco().await {
            crate::integration_db::Banco::Pronto(banco) => banco,
            crate::integration_db::Banco::Ausente(motivo) => {
                eprintln!(
                    "[pulado] banco de teste indisponível: {motivo}. \
                     Defina TEST_DATABASE_URL (ex.: \
                     postgres://postgres:dev@127.0.0.1:55432/postgres) para executar este teste."
                );
                return;
            }
        }
    };
}

/// URL do banco de teste. `TEST_DATABASE_URL` tem precedência; `DATABASE_URL`
/// só é aceita quando aponta para loopback, para nunca criar schema em banco
/// remoto por acidente.
fn url_de_teste() -> Option<String> {
    let _ = dotenvy::from_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".env"));
    if let Ok(url) = std::env::var("TEST_DATABASE_URL") {
        let url = url.trim();
        if !url.is_empty() {
            return Some(url.to_owned());
        }
    }
    let url = std::env::var("DATABASE_URL").ok()?;
    let options: PgConnectOptions = url.parse().ok()?;
    let host = options.get_host().to_lowercase();
    let loopback = matches!(host.as_str(), "127.0.0.1" | "localhost" | "::1" | "[::1]");
    loopback.then_some(url)
}

/// Abre um schema temporário próprio. Público dentro do crate porque
/// `routes::tests` usa o mesmo helper — nenhum teste pode migrar em `public`.
pub(crate) async fn abrir_banco() -> Banco {
    let Some(url) = url_de_teste() else {
        return Banco::Ausente(
            "nenhuma variável TEST_DATABASE_URL ou DATABASE_URL local definida".to_owned(),
        );
    };
    let Ok(options) = url.parse::<PgConnectOptions>() else {
        return Banco::Ausente("TEST_DATABASE_URL não é uma URL do Postgres válida".to_owned());
    };
    let admin = match PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(3))
        .connect_with(options.clone())
        .await
    {
        Ok(pool) => pool,
        Err(error) => return Banco::Ausente(format!("não foi possível conectar ({error})")),
    };

    // Nome só com caractere seguro: entra em DDL sem bind.
    let schema = format!("tmp_teste_previdenciar_{}", Uuid::new_v4().simple());
    if let Err(error) = sqlx::raw_sql(&format!("CREATE SCHEMA {}", schema))
        .execute(&admin)
        .await
    {
        admin.close().await;
        return Banco::Ausente(format!(
            "o usuário do banco não pode criar schema temporário ({error})"
        ));
    }

    // `public` continua no search_path para achar `gen_random_uuid()`. A
    // extensão é criada no schema padrão logo abaixo, fora do schema de teste.
    let scoped = options.options([("search_path", format!("{schema},public"))]);
    let pool = match PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(scoped)
        .await
    {
        Ok(pool) => pool,
        Err(error) => {
            let _ = sqlx::raw_sql(&format!("DROP SCHEMA {} CASCADE", schema))
                .execute(&admin)
                .await;
            admin.close().await;
            return Banco::Ausente(format!(
                "não foi possível conectar ao schema temporário ({error})"
            ));
        }
    };

    // Banco existe e aceita conexão: falha de migração é erro de teste, não
    // motivo para pular silenciosamente.
    //
    // A extensão é criada no schema padrão antes das migrações. Assim o
    // `CREATE EXTENSION IF NOT EXISTS pgcrypto` da migração não tenta criá-la
    // dentro do schema temporário nem exige superusuário de novo.
    let _ = sqlx::raw_sql("CREATE EXTENSION IF NOT EXISTS pgcrypto")
        .execute(&admin)
        .await;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrações aplicadas no schema temporário de teste");

    Banco::Pronto(BancoTemporario {
        pool,
        admin,
        schema,
    })
}

fn estado(config: Config, pool: PgPool) -> AppState {
    AppState {
        config: Arc::new(config),
        db: pool,
        http: reqwest::Client::new(),
        jwks: Arc::new(tokio::sync::RwLock::new(None)),
        storage: None,
        limits: Arc::new(RateLimiter::new()),
        blocking: Arc::new(BlockingGate::new()),
    }
}

async fn subir_servidor(estado: AppState) -> String {
    let app = crate::security_headers(routes::router(estado));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("porta livre para o servidor de teste");
    let addr = listener.local_addr().expect("endereço do listener");
    tokio::spawn(async move {
        let _ = axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await;
    });
    format!("http://{addr}")
}

fn usuario(email: &str) -> auth::AuthUser {
    auth::AuthUser {
        subject: format!("dev:{email}"),
        email: email.to_owned(),
    }
}

fn mensagem_erro(corpo: &Value) -> String {
    corpo
        .get("error")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

async fn bootstrap(servidor: &str, email: &str) -> Value {
    let resposta = reqwest::Client::new()
        .get(format!("{servidor}/api/bootstrap"))
        .header("x-dev-user", email)
        .send()
        .await
        .expect("resposta do servidor");
    assert_eq!(resposta.status(), 200, "bootstrap de {email}");
    resposta.json().await.expect("workspace em json")
}

// ---------------------------------------------------------------------------
// Persistência do usuário e do workspace
// ---------------------------------------------------------------------------

#[tokio::test]
async fn usuario_e_workspace_persistem_entre_chamadas() {
    let banco = banco_ou_pular!();

    let primeiro = db::ensure_user(&banco.pool, &usuario("persistencia@teste.local"))
        .await
        .expect("primeiro cadastro");
    let segundo = db::ensure_user(&banco.pool, &usuario("persistencia@teste.local"))
        .await
        .expect("segundo cadastro é idempotente");
    assert_eq!(primeiro, segundo, "ensure_user não pode duplicar usuário");

    // Mesmo `cognito_sub` com e-mail novo atualiza o e-mail sem trocar o id.
    let com_email_novo = auth::AuthUser {
        subject: "dev:persistencia@teste.local".to_owned(),
        email: "renomeado@teste.local".to_owned(),
    };
    let atualizado = db::ensure_user(&banco.pool, &com_email_novo)
        .await
        .expect("atualização do e-mail");
    assert_eq!(
        atualizado, primeiro,
        "o id deve sobreviver à troca de e-mail"
    );
    let email_gravado: String = sqlx::query_scalar("SELECT email FROM users WHERE id = $1")
        .bind(primeiro)
        .fetch_one(&banco.pool)
        .await
        .expect("e-mail gravado");
    assert_eq!(email_gravado, "renomeado@teste.local");

    let (workspace_id, documento, revisao) = db::bootstrap_workspace(&banco.pool, primeiro)
        .await
        .expect("bootstrap cria o workspace");
    assert_eq!(revisao, 0);
    assert_eq!(
        documento.get("schema_version").and_then(Value::as_u64),
        Some(previdenciar_workspace::SCHEMA_VERSION as u64)
    );

    // Segunda chamada reutiliza o mesmo workspace, sem duplicar.
    let (mesmo_id, _, mesma_revisao) = db::bootstrap_workspace(&banco.pool, primeiro)
        .await
        .expect("bootstrap reutiliza o workspace");
    assert_eq!(mesmo_id, workspace_id);
    assert_eq!(mesma_revisao, 0);

    let mut salvo = previdenciar_workspace::Workspace::default();
    salvo.settings.office_name = "Escritório de teste".to_owned();
    let devolvido = db::save_workspace(&banco.pool, primeiro, salvo)
        .await
        .expect("salva a revisão 0");
    assert_eq!(devolvido.revision, 1, "a revisão avança no banco");

    let (_, relido, revisao_final) = db::bootstrap_workspace(&banco.pool, primeiro)
        .await
        .expect("bootstrap após salvar");
    assert_eq!(revisao_final, 1);
    assert_eq!(
        relido
            .get("settings")
            .and_then(|settings| settings.get("office_name"))
            .and_then(Value::as_str),
        Some("Escritório de teste")
    );
    assert_eq!(
        relido.get("revision").and_then(Value::as_u64),
        Some(1),
        "o documento devolvido carrega a revisão vigente"
    );

    banco.descartar().await;
}

#[tokio::test]
async fn save_workspace_recusa_revisao_antiga_pelo_http() {
    let banco = banco_ou_pular!();
    let servidor = subir_servidor(estado(Config::for_tests(), banco.pool.clone())).await;
    let cliente = reqwest::Client::new();
    let email = "revisao@teste.local";

    let inicial = bootstrap(&servidor, email).await;
    let mut workspace = inicial["workspace"].clone();
    workspace["settings"]["office_name"] = json!("Primeira gravação");

    let resposta = cliente
        .put(format!("{servidor}/api/workspace"))
        .header("x-dev-user", email)
        .json(&json!({ "workspace": workspace }))
        .send()
        .await
        .expect("resposta do servidor");
    assert_eq!(resposta.status(), 200);
    let salvo: Value = resposta.json().await.expect("workspace salvo");
    assert_eq!(salvo["revision"], json!(1));

    // Reenviar a revisão 0 precisa ser recusado: outra sessão já gravou.
    let resposta = cliente
        .put(format!("{servidor}/api/workspace"))
        .header("x-dev-user", email)
        .json(&json!({ "workspace": workspace }))
        .send()
        .await
        .expect("resposta do servidor");
    assert_eq!(resposta.status(), 409);
    let corpo: Value = resposta.json().await.expect("erro em json");
    assert_eq!(
        mensagem_erro(&corpo),
        "A base foi alterada em outra sessão. Recarregue antes de salvar."
    );

    let depois = bootstrap(&servidor, email).await;
    assert_eq!(depois["workspace"]["revision"], json!(1));
    assert_eq!(
        depois["workspace"]["settings"]["office_name"],
        json!("Primeira gravação")
    );

    // Salvar sem workspace existente é NotFound, não gravação órfã.
    let orfao = db::ensure_user(&banco.pool, &usuario("sem-workspace@teste.local"))
        .await
        .expect("usuário sem workspace");
    match db::save_workspace(
        &banco.pool,
        orfao,
        previdenciar_workspace::Workspace::default(),
    )
    .await
    {
        Err(db::SaveError::NotFound) => {}
        Err(outro) => panic!("esperava SaveError::NotFound, veio {outro:?}"),
        Ok(_) => panic!("save_workspace não pode gravar sem workspace prévio"),
    }

    banco.descartar().await;
}

#[tokio::test]
async fn workspace_de_um_usuario_nao_vaza_para_outro() {
    let banco = banco_ou_pular!();
    let servidor = subir_servidor(estado(Config::for_tests(), banco.pool.clone())).await;
    let cliente = reqwest::Client::new();

    let alice = "alice@teste.local";
    let bruno = "bruno@teste.local";

    let mut espaco_alice = bootstrap(&servidor, alice).await["workspace"].clone();
    espaco_alice["settings"]["office_name"] = json!("Escritório da Alice");
    let resposta = cliente
        .put(format!("{servidor}/api/workspace"))
        .header("x-dev-user", alice)
        .json(&json!({ "workspace": espaco_alice }))
        .send()
        .await
        .expect("resposta do servidor");
    assert_eq!(resposta.status(), 200);

    // Bruno recebe um workspace novo, vazio, sem o conteúdo da Alice.
    let espaco_bruno = bootstrap(&servidor, bruno).await["workspace"].clone();
    assert_eq!(espaco_bruno["settings"]["office_name"], json!(""));
    assert_ne!(
        espaco_bruno["settings"]["office_name"],
        espaco_alice["settings"]["office_name"]
    );

    // A gravação do Bruno não altera o documento da Alice.
    let mut novo_bruno = espaco_bruno;
    novo_bruno["settings"]["office_name"] = json!("Escritório do Bruno");
    let resposta = cliente
        .put(format!("{servidor}/api/workspace"))
        .header("x-dev-user", bruno)
        .json(&json!({ "workspace": novo_bruno }))
        .send()
        .await
        .expect("resposta do servidor");
    assert_eq!(resposta.status(), 200);

    let relido_alice = bootstrap(&servidor, alice).await;
    assert_eq!(
        relido_alice["workspace"]["settings"]["office_name"],
        json!("Escritório da Alice")
    );

    let total_workspaces: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM workspaces")
        .fetch_one(&banco.pool)
        .await
        .expect("contagem de workspaces");
    assert_eq!(total_workspaces, 2, "cada usuário tem o próprio workspace");

    banco.descartar().await;
}

// ---------------------------------------------------------------------------
// Auditoria e limites de listagem
// ---------------------------------------------------------------------------

#[tokio::test]
async fn auditoria_de_login_nao_guarda_dados_pessoais() {
    let banco = banco_ou_pular!();
    let servidor = subir_servidor(estado(Config::for_tests(), banco.pool.clone())).await;
    let email = "auditoria@teste.local";

    let resposta = reqwest::Client::new()
        .get(format!("{servidor}/api/me"))
        .header("x-dev-user", email)
        .send()
        .await
        .expect("resposta do servidor");
    assert_eq!(resposta.status(), 200);

    let linhas = sqlx::query(
        "SELECT entity_type, action, payload::text AS payload, actor_id FROM audit_events ORDER BY id",
    )
    .fetch_all(&banco.pool)
    .await
    .expect("eventos de auditoria");
    assert_eq!(linhas.len(), 1, "o login gera exatamente um evento");
    let linha = &linhas[0];
    assert_eq!(linha.get::<String, _>("action"), "login");
    assert_eq!(linha.get::<String, _>("entity_type"), "user");
    let payload = linha.get::<String, _>("payload");
    assert_eq!(payload, "{}", "o payload do login fica vazio");
    assert!(!payload.contains('@'), "o payload não pode guardar e-mail");
    assert!(
        !payload.contains(email),
        "nenhum identificador pessoal no payload"
    );
    assert!(linha.get::<Option<Uuid>, _>("actor_id").is_some());

    banco.descartar().await;
}

#[tokio::test]
async fn auditoria_admin_respeita_o_teto_de_200_eventos() {
    let banco = banco_ou_pular!();
    let servidor = subir_servidor(estado(Config::for_tests(), banco.pool.clone())).await;
    let cliente = reqwest::Client::new();

    // Cria o admin que fará a consulta.
    bootstrap(&servidor, "admin@teste.local").await;

    sqlx::query(
        "INSERT INTO audit_events (entity_type, action, payload, created_at)
         SELECT 'calculation', 'calculate', '{}'::jsonb,
                now() + (g::text || ' seconds')::interval
         FROM generate_series(1, 205) AS g",
    )
    .execute(&banco.pool)
    .await
    .expect("eventos sintéticos");

    let resposta = cliente
        .get(format!("{servidor}/api/admin/audit"))
        .header("x-dev-user", "admin@teste.local")
        .send()
        .await
        .expect("resposta do servidor");
    assert_eq!(resposta.status(), 200);
    let corpo: Value = resposta.json().await.expect("auditoria em json");
    let eventos = corpo["events"].as_array().expect("lista de eventos");
    assert_eq!(eventos.len(), 200, "a consulta limita a 200 linhas");
    assert_eq!(
        eventos[0]["action"],
        json!("calculate"),
        "o evento mais recente vem primeiro"
    );

    banco.descartar().await;
}

#[tokio::test]
async fn admin_users_respeita_o_teto_de_500_usuarios() {
    let banco = banco_ou_pular!();
    let servidor = subir_servidor(estado(Config::for_tests(), banco.pool.clone())).await;

    sqlx::query(
        "INSERT INTO users (cognito_sub, email, created_at)
         SELECT 'lote-' || g, 'lote-' || g || '@teste.local',
                now() + (g::text || ' seconds')::interval
         FROM generate_series(1, 502) AS g",
    )
    .execute(&banco.pool)
    .await
    .expect("usuários sintéticos");

    let resposta = reqwest::Client::new()
        .get(format!("{servidor}/api/admin/users"))
        .header("x-dev-user", "admin@teste.local")
        .send()
        .await
        .expect("resposta do servidor");
    assert_eq!(resposta.status(), 200);
    let corpo: Value = resposta.json().await.expect("usuários em json");
    let usuarios = corpo["users"].as_array().expect("lista de usuários");
    assert_eq!(usuarios.len(), 500, "a consulta limita a 500 linhas");
    assert_eq!(
        usuarios[0]["email"],
        json!("lote-502@teste.local"),
        "o usuário mais recente vem primeiro"
    );

    banco.descartar().await;
}

#[tokio::test]
async fn reports_saved_ignora_expirados_e_respeita_o_teto_de_100() {
    let banco = banco_ou_pular!();
    let servidor = subir_servidor(estado(Config::for_tests(), banco.pool.clone())).await;
    let email = "relatorios@teste.local";

    bootstrap(&servidor, email).await;
    let owner: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE cognito_sub = $1")
        .bind(format!("dev:{email}"))
        .fetch_one(&banco.pool)
        .await
        .expect("usuário criado pelo bootstrap");
    let workspace_id: Uuid = sqlx::query_scalar("SELECT id FROM workspaces WHERE owner_id = $1")
        .bind(owner)
        .fetch_one(&banco.pool)
        .await
        .expect("workspace do usuário");

    sqlx::query(
        "INSERT INTO report_artifacts
           (workspace_id, owner_id, calculation_id, format, s3_key, size_bytes, sha256, created_at, expires_at)
         SELECT $1, $2, g, 'pdf_resumo', 'reports/' || $2 || '/' || g || '.pdf', 1024,
                repeat('a', 64), now() + (g::text || ' seconds')::interval,
                now() + interval '30 days'
         FROM generate_series(1, 103) AS g",
    )
    .bind(workspace_id)
    .bind(owner)
    .execute(&banco.pool)
    .await
    .expect("artefatos válidos");

    // Dois artefatos já expirados não podem aparecer na listagem.
    sqlx::query(
        "INSERT INTO report_artifacts
           (workspace_id, owner_id, calculation_id, format, s3_key, size_bytes, sha256, expires_at)
         VALUES ($1, $2, 900, 'pdf_resumo', 'reports/expirado-a.pdf', 10, repeat('b', 64),
                 now() - interval '1 day'),
                ($1, $2, 901, 'pdf_resumo', 'reports/expirado-b.pdf', 10, repeat('b', 64),
                 now() - interval '2 days')",
    )
    .bind(workspace_id)
    .bind(owner)
    .execute(&banco.pool)
    .await
    .expect("artefatos expirados");

    let resposta = reqwest::Client::new()
        .get(format!("{servidor}/api/reports/saved"))
        .header("x-dev-user", email)
        .send()
        .await
        .expect("resposta do servidor");
    assert_eq!(resposta.status(), 200);
    let corpo: Value = resposta.json().await.expect("relatórios em json");
    let relatorios = corpo["reports"].as_array().expect("lista de relatórios");
    assert_eq!(relatorios.len(), 100, "a listagem limita a 100 linhas");
    assert_eq!(
        relatorios[0]["calculationId"],
        json!(103),
        "o mais recente vem primeiro"
    );
    assert!(
        relatorios
            .iter()
            .all(|item| item["calculationId"] != json!(900) && item["calculationId"] != json!(901)),
        "artefatos expirados ficam fora da listagem"
    );

    banco.descartar().await;
}

// ---------------------------------------------------------------------------
// Admin, termos e entitlement
// ---------------------------------------------------------------------------

#[tokio::test]
async fn admin_entitlement_atualiza_o_usuario_alvo_no_banco() {
    let banco = banco_ou_pular!();
    let servidor = subir_servidor(estado(Config::for_tests(), banco.pool.clone())).await;
    let cliente = reqwest::Client::new();
    let alvo = "alvo@teste.local";

    // O alvo se registra: o entitlement nasce com advanced = true.
    let corpo = bootstrap(&servidor, alvo).await;
    assert!(corpo["workspace"].is_object());
    let resposta = cliente
        .get(format!("{servidor}/api/me"))
        .header("x-dev-user", alvo)
        .send()
        .await
        .expect("resposta do servidor");
    let perfil: Value = resposta.json().await.expect("perfil em json");
    assert_eq!(perfil["advanced"], json!(true));
    assert_eq!(perfil["admin"], json!(false));

    // Admin desliga o acesso avançado.
    let resposta = cliente
        .post(format!("{servidor}/api/admin/entitlement"))
        .header("x-dev-user", "admin@teste.local")
        .json(&json!({ "email": alvo, "advanced": false }))
        .send()
        .await
        .expect("resposta do servidor");
    assert_eq!(resposta.status(), 200);

    let gravado: bool = sqlx::query_scalar(
        "SELECT e.advanced FROM entitlements e JOIN users u ON u.id = e.user_id
         WHERE lower(u.email) = lower($1)",
    )
    .bind(alvo)
    .fetch_one(&banco.pool)
    .await
    .expect("entitlement gravado");
    assert!(!gravado);

    let resposta = cliente
        .get(format!("{servidor}/api/me"))
        .header("x-dev-user", alvo)
        .send()
        .await
        .expect("resposta do servidor");
    let perfil: Value = resposta.json().await.expect("perfil em json");
    assert_eq!(
        perfil["advanced"],
        json!(false),
        "o /api/me reflete o banco"
    );

    // Usuário inexistente é 404, não cria linha órfã.
    let resposta = cliente
        .post(format!("{servidor}/api/admin/entitlement"))
        .header("x-dev-user", "admin@teste.local")
        .json(&json!({ "email": "ninguem@teste.local", "advanced": true }))
        .send()
        .await
        .expect("resposta do servidor");
    assert_eq!(resposta.status(), 404);
    let corpo: Value = resposta.json().await.expect("erro em json");
    assert_eq!(mensagem_erro(&corpo), "Usuário não encontrado.");

    banco.descartar().await;
}

#[tokio::test]
async fn aceite_de_termos_e_idempotente_e_aparece_no_perfil() {
    let banco = banco_ou_pular!();
    let servidor = subir_servidor(estado(Config::for_tests(), banco.pool.clone())).await;
    let cliente = reqwest::Client::new();
    let email = "termos@teste.local";

    bootstrap(&servidor, email).await;

    for tentativa in 0..2 {
        let resposta = cliente
            .post(format!("{servidor}/api/terms/accept"))
            .header("x-dev-user", email)
            .json(&json!({}))
            .send()
            .await
            .expect("resposta do servidor");
        assert_eq!(resposta.status(), 200, "aceite {tentativa}");
        let corpo: Value = resposta.json().await.expect("aceite em json");
        assert_eq!(corpo["accepted"], json!(true));
        assert_eq!(corpo["version"], json!("2026-10-31"));
    }

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM terms_acceptances")
        .fetch_one(&banco.pool)
        .await
        .expect("contagem de aceites");
    assert_eq!(total, 1, "o aceite repetido não duplica a linha");

    let resposta = cliente
        .get(format!("{servidor}/api/me"))
        .header("x-dev-user", email)
        .send()
        .await
        .expect("resposta do servidor");
    let perfil: Value = resposta.json().await.expect("perfil em json");
    assert_eq!(perfil["terms"]["accepted"], json!(true));
    assert_eq!(perfil["terms"]["version"], json!("2026-10-31"));

    banco.descartar().await;
}

// ---------------------------------------------------------------------------
// Sanitização de segredos persistidos
// ---------------------------------------------------------------------------

#[tokio::test]
async fn auditoria_nao_expoe_chave_de_api_do_workspace() {
    let banco = banco_ou_pular!();
    let servidor = subir_servidor(estado(Config::for_tests(), banco.pool.clone())).await;
    let cliente = reqwest::Client::new();
    let email = "segredo@teste.local";
    const CHAVE: &str = "chave-sintetica-de-teste-nao-real";

    let inicial = bootstrap(&servidor, email).await;
    let mut workspace = inicial["workspace"].clone();
    workspace["settings"]["fabrica_api_key"] = json!(CHAVE);

    let resposta = cliente
        .put(format!("{servidor}/api/workspace"))
        .header("x-dev-user", email)
        .json(&json!({ "workspace": workspace }))
        .send()
        .await
        .expect("resposta do servidor");
    assert_eq!(resposta.status(), 200);

    // Nada na auditoria, na listagem de usuários nem na de relatórios pode
    // carregar a chave: são respostas de leitura ampla.
    let trilha = cliente
        .get(format!("{servidor}/api/admin/audit"))
        .header("x-dev-user", "admin@teste.local")
        .send()
        .await
        .expect("resposta do servidor")
        .text()
        .await
        .expect("corpo da auditoria");
    assert!(
        !trilha.contains(CHAVE),
        "a auditoria não pode expor a chave do workspace"
    );

    let usuarios = cliente
        .get(format!("{servidor}/api/admin/users"))
        .header("x-dev-user", "admin@teste.local")
        .send()
        .await
        .expect("resposta do servidor")
        .text()
        .await
        .expect("corpo da lista de usuários");
    assert!(
        !usuarios.contains(CHAVE),
        "a lista de usuários não pode expor a chave do workspace"
    );

    let payloads: Vec<String> = sqlx::query_scalar("SELECT payload::text FROM audit_events")
        .fetch_all(&banco.pool)
        .await
        .expect("payloads de auditoria");
    for payload in payloads {
        assert!(
            !payload.contains(CHAVE),
            "nenhum payload de auditoria pode guardar a chave: {payload}"
        );
    }

    // Achado registrado no relatório, não corrigido aqui: o documento do
    // workspace é gravado inteiro em `workspaces.document` (jsonb), inclusive
    // `settings.fabrica_api_key`, sem cifra. O teste não afirma esse defeito
    // para não travar a correção; só verifica o que já deve valer hoje: vazamento
    // zero para auditoria e para outro usuário.
    let documento: Value = sqlx::query_scalar(
        "SELECT document FROM workspaces WHERE owner_id = (SELECT id FROM users WHERE email = $1)",
    )
    .bind(email)
    .fetch_one(&banco.pool)
    .await
    .expect("documento do workspace");
    if documento["settings"]["fabrica_api_key"] == json!(CHAVE) {
        eprintln!(
            "[observação] settings.fabrica_api_key está gravada em claro em workspaces.document; \
             trocar por cifra ou remover do documento antes de persistir."
        );
    }

    // Outro usuário não recebe o documento nem a chave de quem gravou.
    let outro = bootstrap(&servidor, "outro-segredo@teste.local").await;
    assert!(
        !outro.to_string().contains(CHAVE),
        "o workspace de um usuário não pode expor a chave de outro"
    );

    banco.descartar().await;
}

// ---------------------------------------------------------------------------
// Sanitização de erro (não depende de banco)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn erro_de_banco_nao_vaza_detalhe_tecnico_para_o_cliente() {
    // Pool que nunca conecta: força a falha de persistência.
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_millis(500))
        .connect_lazy("postgres://teste:inexistente@127.0.0.1:9/sem_banco")
        .expect("url de banco sintética válida");
    let servidor = subir_servidor(estado(Config::for_tests(), pool)).await;

    let resposta = reqwest::Client::new()
        .get(format!("{servidor}/api/me"))
        .header("x-dev-user", "erro@teste.local")
        .send()
        .await
        .expect("resposta do servidor");
    assert_eq!(resposta.status(), 500);
    let headers: &HeaderMap = resposta.headers();
    assert!(
        headers
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("application/json")),
        "erro de servidor continua em JSON"
    );
    let corpo: Value = resposta.json().await.expect("erro em json");
    assert_eq!(
        mensagem_erro(&corpo),
        "Não foi possível concluir a operação."
    );
    let texto = corpo.to_string();
    for proibido in ["postgres", "127.0.0.1", "connection", "pool", "sqlx"] {
        assert!(
            !texto.to_lowercase().contains(proibido),
            "a mensagem vazou detalhe técnico: {texto}"
        );
    }
}
