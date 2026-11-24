use crate::auth::AuthUser;
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub async fn ensure_user(pool: &PgPool, user: &AuthUser) -> Result<Uuid, sqlx::Error> {
    // Identidade já ligada a uma conta (por exemplo, a entrada pelo Atrium de
    // quem já tinha conta pelo Google ou por e-mail e senha).
    if let Some(id) = sqlx::query_scalar::<_, Uuid>("SELECT user_id FROM user_identities WHERE subject = $1")
        .bind(&user.subject)
        .fetch_optional(pool)
        .await?
    {
        return Ok(id);
    }
    let inserted = sqlx::query(
        "INSERT INTO users (cognito_sub, email) VALUES ($1, $2)
         ON CONFLICT (cognito_sub) DO UPDATE SET email = EXCLUDED.email
         RETURNING id",
    )
    .bind(&user.subject)
    .bind(&user.email)
    .fetch_one(pool)
    .await;
    match inserted {
        Ok(row) => Ok(row.get("id")),
        Err(error) if email_taken(&error) && may_link_by_email(user) => link_identity_by_email(pool, user).await,
        Err(error) => Err(error),
    }
}

fn email_taken(error: &sqlx::Error) -> bool {
    matches!(error, sqlx::Error::Database(db) if db.constraint() == Some("users_email_lower_idx"))
}

/// Só a entrada pelo Atrium se liga por e-mail: o servidor confere usuário,
/// senha e assinatura no Atrium antes de emitir essa identidade. Uma conta
/// nova do Cognito com e-mail repetido continua recusada.
fn may_link_by_email(user: &AuthUser) -> bool {
    user.subject.starts_with("atrium:") && !user.email.trim().is_empty()
}

async fn link_identity_by_email(pool: &PgPool, user: &AuthUser) -> Result<Uuid, sqlx::Error> {
    let owner: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE lower(email) = lower($1)")
        .bind(&user.email)
        .fetch_one(pool)
        .await?;
    sqlx::query_scalar(
        "INSERT INTO user_identities (subject, user_id) VALUES ($1, $2)
         ON CONFLICT (subject) DO UPDATE SET subject = EXCLUDED.subject
         RETURNING user_id",
    )
    .bind(&user.subject)
    .bind(owner)
    .fetch_one(pool)
    .await
}

fn strip_workspace_secrets(doc: &mut Value) {
    if let Some(settings) = doc.get_mut("settings").and_then(Value::as_object_mut) {
        if settings.contains_key("fabrica_api_key") {
            settings.insert("fabrica_api_key".into(), Value::String(String::new()));
        }
    }
}

pub async fn bootstrap_workspace(pool: &PgPool, owner_id: Uuid) -> Result<(Uuid, Value, i64), sqlx::Error> {
    let mut tx = pool.begin().await?;

    // Serializa somente o primeiro acesso do mesmo usuário. A linha de users
    // já existe (ensure_user) e é o ponto de coordenação sem nova migration.
    let owner = sqlx::query("SELECT id FROM users WHERE id = $1 FOR UPDATE")
        .bind(owner_id)
        .fetch_optional(&mut *tx)
        .await?;
    if owner.is_none() {
        return Err(sqlx::Error::RowNotFound);
    }

    if let Some(row) = sqlx::query(
        "SELECT id, document, revision FROM workspaces
         WHERE owner_id = $1 ORDER BY created_at ASC LIMIT 1",
    )
    .bind(owner_id)
    .fetch_optional(&mut *tx)
    .await?
    {
        let mut document: Value = row.get("document");
        strip_workspace_secrets(&mut document);
        let result = (row.get("id"), document, row.get("revision"));
        tx.commit().await?;
        return Ok(result);
    }

    let document = serde_json::to_value(previdenciar_workspace::Workspace::default())
        .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;
    let row = sqlx::query(
        "INSERT INTO workspaces (owner_id, document) VALUES ($1, $2)
         RETURNING id, document, revision",
    )
    .bind(owner_id)
    .bind(&document)
    .fetch_one(&mut *tx)
    .await?;
    let result = (row.get("id"), row.get("document"), row.get("revision"));
    tx.commit().await?;
    Ok(result)
}

pub async fn save_workspace(
    pool: &PgPool,
    owner_id: Uuid,
    mut workspace: previdenciar_workspace::Workspace,
) -> Result<previdenciar_workspace::Workspace, SaveError> {
    let current = sqlx::query("SELECT id, revision FROM workspaces WHERE owner_id = $1 ORDER BY created_at ASC LIMIT 1")
        .bind(owner_id)
        .fetch_optional(pool)
        .await
        .map_err(SaveError::Database)?
        .ok_or(SaveError::NotFound)?;
    let id: Uuid = current.get("id");
    let revision: i64 = current.get("revision");
    if workspace.revision != revision as u64 {
        return Err(SaveError::Conflict);
    }
    workspace.revision = (revision + 1) as u64;
    // Segurança (R4): jamais gravar chaves de API do cliente em workspaces.document (jsonb).
    workspace.settings.fabrica_api_key.clear();
    let document = serde_json::to_value(&workspace)
        .map_err(|error| SaveError::Serialization(error.to_string()))?;
    let result = sqlx::query(
        "UPDATE workspaces SET document = $3, revision = revision + 1, updated_at = now()
         WHERE id = $1 AND owner_id = $2 AND revision = $4
         RETURNING revision",
    )
    .bind(id)
    .bind(owner_id)
    .bind(&document)
    .bind(revision)
    .fetch_optional(pool)
    .await
    .map_err(SaveError::Database)?;
    if result.is_none() {
        return Err(SaveError::Conflict);
    }
    Ok(workspace)
}

#[derive(Debug)]
pub enum SaveError {
    NotFound,
    Conflict,
    Database(sqlx::Error),
    Serialization(String),
}

pub async fn ensure_entitlement(pool: &PgPool, user_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO entitlements (user_id, advanced) VALUES ($1, true) ON CONFLICT (user_id) DO NOTHING")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn session_info(pool: &PgPool, user_id: Uuid, terms_version: &str) -> Result<(bool, bool), sqlx::Error> {
    ensure_entitlement(pool, user_id).await?;
    let advanced = sqlx::query_scalar::<_, bool>("SELECT advanced FROM entitlements WHERE user_id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await?;
    let accepted = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM terms_acceptances WHERE user_id = $1 AND version = $2)",
    )
    .bind(user_id)
    .bind(terms_version)
    .fetch_one(pool)
    .await?;
    Ok((advanced, accepted))
}

pub async fn accept_terms(pool: &PgPool, user_id: Uuid, version: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO terms_acceptances (user_id, version) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(user_id)
        .bind(version)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn audit(
    pool: &PgPool,
    workspace_id: Option<Uuid>,
    actor_id: Uuid,
    entity_type: &str,
    entity_id: Option<&str>,
    action: &str,
    payload: serde_json::Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO audit_events (workspace_id, actor_id, entity_type, entity_id, action, payload)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(workspace_id)
    .bind(actor_id)
    .bind(entity_type)
    .bind(entity_id)
    .bind(action)
    .bind(payload)
    .execute(pool)
    .await?;
    Ok(())
}

