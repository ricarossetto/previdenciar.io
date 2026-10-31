use crate::{auth, db, AppState};
use axum::{
    body::Body,
    extract::{Multipart, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{sse::{Event, KeepAlive, Sse}, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use sqlx::Row;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use std::{convert::Infallible, sync::{atomic::AtomicBool, Arc}, time::{Duration, Instant}};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};

pub fn router(state: AppState) -> Router {
    let mut router = Router::new()
        .route("/api/health", get(health))
        .route("/api/auth/token", post(auth_token))
        .route("/api/auth/refresh", post(auth_refresh))
        .route("/api/bootstrap", get(bootstrap))
        .route("/api/me", get(me))
        .route("/api/terms/accept", post(accept_terms))
        .route("/api/admin/users", get(admin_users))
        .route("/api/admin/entitlement", post(admin_entitlement))
        .route("/api/admin/audit", get(admin_audit))
        .route("/api/reports/save", post(save_report))
        .route("/api/reports/saved", get(saved_reports))
        .route("/api/reports/saved/{id}", get(saved_report_url).delete(delete_saved_report))
        .route("/api/workspace", axum::routing::put(save_workspace))
        .route("/api/calculate", post(calculate))
        .route("/api/pcd/snapshot", get(pcd_snapshot))
        .route("/api/cnis/import", post(import_cnis))
        .route("/api/case-pdf/text", post(case_pdf_text))
        .route("/api/reports/preview", post(preview_report))
        .route("/api/prepare-from-cnis", post(prepare_from_cnis))
        .route("/api/ai/status", get(ai_status))
        .route("/api/ai/chat", post(ai_chat))
        .route("/api/tools/{tool}", post(run_tool))
        .route("/api/finance/{tool}", post(run_finance))
        .with_state(state.clone());

    if let Some(dist) = &state.config.web_dist {
        let index = std::path::Path::new(dist).join("index.html");
        router = router.fallback_service(
            tower_http::services::ServeDir::new(dist)
                .append_index_html_on_directories(true)
                .fallback(tower_http::services::ServeFile::new(index)),
        );
    }
    router
}

#[derive(Deserialize)]
struct TokenRequest {
    code: String,
    code_verifier: String,
    redirect_uri: String,
}

async fn auth_token(
    State(state): State<AppState>,
    Json(body): Json<TokenRequest>,
) -> Result<Json<Value>, ApiError> {
    if state.config.cognito_domain.is_empty() {
        return Err(ApiError::internal("COGNITO_DOMAIN nÃ£o configurado."));
    }
    let response = state.http
        .post(format!("https://{}/oauth2/token", state.config.cognito_domain))
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", state.config.cognito_client_id.as_str()),
            ("code", body.code.as_str()),
            ("redirect_uri", body.redirect_uri.as_str()),
            ("code_verifier", body.code_verifier.as_str()),
        ])
        .send()
        .await
        .map_err(ApiError::internal)?;
    let status = response.status();
    let text = response.text().await.map_err(ApiError::internal)?;
    if !status.is_success() {
        tracing::warn!(status = %status, "cognito token exchange failed");
        return Err(ApiError::unauthorized("NÃ£o foi possÃ­vel concluir o login.".into()));
    }
    let value: Value = serde_json::from_str(&text).map_err(ApiError::internal)?;
    Ok(Json(value))
}

#[derive(Deserialize)]
struct RefreshRequest {
    refresh_token: String,
}

async fn auth_refresh(
    State(state): State<AppState>,
    Json(body): Json<RefreshRequest>,
) -> Result<Json<Value>, ApiError> {
    if state.config.cognito_domain.is_empty() {
        return Err(ApiError::internal("COGNITO_DOMAIN nÃ£o configurado."));
    }
    if body.refresh_token.trim().is_empty() {
        return Err(ApiError::unauthorized("SessÃ£o expirada. Entre novamente.".into()));
    }
    let response = state.http
        .post(format!("https://{}/oauth2/token", state.config.cognito_domain))
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", state.config.cognito_client_id.as_str()),
            ("refresh_token", body.refresh_token.as_str()),
        ])
        .send()
        .await
        .map_err(ApiError::internal)?;
    let status = response.status();
    let text = response.text().await.map_err(ApiError::internal)?;
    if !status.is_success() {
        tracing::warn!(status = %status, "cognito refresh failed");
        return Err(ApiError::unauthorized("SessÃ£o expirada. Entre novamente.".into()));
    }
    let value: Value = serde_json::from_str(&text).map_err(ApiError::internal)?;
    Ok(Json(value))
}

async fn ai_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let _user = auth::authenticate(&headers, &state).await.map_err(ApiError::unauthorized)?;
    Ok(Json(json!({
        "enabled": state.config.ai_enabled(),
        "providerId": state.config.ai_provider_id,
        "baseUrl": state.config.ai_base_url,
        "model": state.config.ai_model,
        "message": if state.config.ai_enabled() { Value::Null } else { json!("Configure AI_API_KEY no servidor para habilitar o Copiloto.") }
    })))
}

async fn ai_chat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut body): Json<previdenciar_ai::ChatRequest>,
) -> Result<Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let user = auth::authenticate(&headers, &state).await.map_err(ApiError::unauthorized)?;
    rate_limit(&state, format!("ai:{}", user.email), 30, Duration::from_secs(3600)).await?;
    if !state.config.ai_enabled() {
        return Err(ApiError::unavailable("O Copiloto ainda nÃ£o foi habilitado neste servidor.".into()));
    }
    body.provider_id = state.config.ai_provider_id.clone();
    body.base_url = state.config.ai_base_url.clone();
    body.model = state.config.ai_model.clone();
    let key = state.config.ai_api_key.clone();
    let data_root = state.config.ai_data_root.clone();
    let request_id = body.request_id.clone();
    let (tx, rx) = mpsc::channel::<Value>(64);
    let cancelled = Arc::new(AtomicBool::new(false));
    tokio::task::spawn_blocking(move || {
        let data = previdenciar_ai::DataStore::new(data_root.into());
        let result = previdenciar_ai::run_with_key(body, &key, &data, &cancelled, |event| {
            if let Ok(value) = serde_json::to_value(event) {
                let _ = tx.blocking_send(value);
            }
        });
        if let Err(error) = result {
            tracing::warn!(request = %request_id, error = %error, "AI request failed");
            let _ = tx.blocking_send(json!({"type": "error", "message": error.to_string()}));
        }
    });
    let stream = ReceiverStream::new(rx).map(|value| {
        Ok::<Event, Infallible>(Event::default().json_data(value).unwrap_or_else(|_| Event::default().data("{}")))
    });
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

async fn health() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "previdenciar-web-server",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn bootstrap(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let user = auth::authenticate(&headers, &state).await.map_err(ApiError::unauthorized)?;
    let owner = db::ensure_user(&state.db, &user).await.map_err(ApiError::internal)?;
    let (_, document, revision) = db::bootstrap_workspace(&state.db, owner)
        .await
        .map_err(ApiError::internal)?;
    let mut workspace = document;
    if workspace.get("revision").and_then(Value::as_u64).is_none() {
        workspace["revision"] = json!(revision);
    }
    Ok(Json(json!({
        "workspace": workspace,
        "storage_ready": true,
        "message": null
    })))
}

async fn me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let user = auth::authenticate(&headers, &state).await.map_err(ApiError::unauthorized)?;
    let owner = db::ensure_user(&state.db, &user).await.map_err(ApiError::internal)?;
    let (advanced, accepted) = db::session_info(&state.db, owner, &state.config.terms_version)
        .await
        .map_err(ApiError::internal)?;
    let _ = db::audit(&state.db, None, owner, "user", Some(&owner.to_string()), "login", json!({})).await;
    Ok(Json(json!({
        "user": { "email": user.email, "subject": user.subject },
        "advanced": advanced,
        "admin": state.config.is_admin(&user.email),
        "terms": { "version": state.config.terms_version, "accepted": accepted }
    })))
}

#[derive(Deserialize)]
struct TermsRequest {}

async fn accept_terms(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(_body): Json<TermsRequest>,
) -> Result<Json<Value>, ApiError> {
    let user = auth::authenticate(&headers, &state).await.map_err(ApiError::unauthorized)?;
    let owner = db::ensure_user(&state.db, &user).await.map_err(ApiError::internal)?;
    db::accept_terms(&state.db, owner, &state.config.terms_version)
        .await
        .map_err(ApiError::internal)?;
    let _ = db::audit(&state.db, None, owner, "terms", Some(&state.config.terms_version), "accept", json!({})).await;
    Ok(Json(json!({ "accepted": true, "version": state.config.terms_version })))
}

async fn require_admin(headers: &HeaderMap, state: &AppState) -> Result<Uuid, ApiError> {
    let user = auth::authenticate(headers, state).await.map_err(ApiError::unauthorized)?;
    if !state.config.is_admin(&user.email) {
        return Err(ApiError::forbidden("Acesso administrativo restrito.".into()));
    }
    db::ensure_user(&state.db, &user).await.map_err(ApiError::internal)
}

async fn admin_users(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let _admin = require_admin(&headers, &state).await?;
    let rows = sqlx::query(
        "SELECT u.id, u.email, COALESCE(e.advanced, true) AS advanced, u.created_at
         FROM users u LEFT JOIN entitlements e ON e.user_id = u.id
         ORDER BY u.created_at DESC LIMIT 500",
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::internal)?;
    let users = rows.into_iter().map(|row| json!({
        "id": row.get::<Uuid, _>("id"),
        "email": row.get::<String, _>("email"),
        "advanced": row.get::<bool, _>("advanced"),
        "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at")
    })).collect::<Vec<_>>();
    Ok(Json(json!({ "users": users })))
}

#[derive(Deserialize)]
struct EntitlementRequest {
    email: String,
    advanced: bool,
}

async fn admin_entitlement(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<EntitlementRequest>,
) -> Result<Json<Value>, ApiError> {
    let admin = require_admin(&headers, &state).await?;
    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE lower(email) = lower($1)")
        .bind(body.email.trim())
        .fetch_optional(&state.db)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("Usuário não encontrado."))?;
    sqlx::query(
        "INSERT INTO entitlements (user_id, advanced) VALUES ($1, $2)
         ON CONFLICT (user_id) DO UPDATE SET advanced = EXCLUDED.advanced, updated_at = now()",
    )
    .bind(user_id)
    .bind(body.advanced)
    .execute(&state.db)
    .await
    .map_err(ApiError::internal)?;
    let _ = db::audit(&state.db, None, admin, "entitlement", Some(&user_id.to_string()), "update", json!({ "advanced": body.advanced })).await;
    Ok(Json(json!({ "email": body.email, "advanced": body.advanced })))
}

async fn admin_audit(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let _admin = require_admin(&headers, &state).await?;
    let rows = sqlx::query(
        "SELECT id, actor_id, entity_type, entity_id, action, payload, created_at
         FROM audit_events ORDER BY created_at DESC LIMIT 200",
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::internal)?;
    let events = rows.into_iter().map(|row| json!({
        "id": row.get::<i64, _>("id"),
        "actor_id": row.get::<Option<Uuid>, _>("actor_id"),
        "entity_type": row.get::<String, _>("entity_type"),
        "entity_id": row.get::<Option<String>, _>("entity_id"),
        "action": row.get::<String, _>("action"),
        "payload": row.get::<Value, _>("payload"),
        "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at")
    })).collect::<Vec<_>>();
    Ok(Json(json!({ "events": events })))
}

fn render_report_bytes(body: &ReportRequest) -> Result<Vec<u8>, ApiError> {
    let kind_name = body.format.strip_prefix("pdf_").unwrap_or(&body.format);
    let kind = previdenciar_reports::ReportKind::parse(kind_name).map_err(ApiError::bad_request)?;
    let outcome = previdenciar_rgps::calculate_case(&body.calculation).map_err(ApiError::bad_request)?;
    let presentation = body.presentation.as_ref();
    let options = previdenciar_reports::ReportOptions {
        kind,
        include_personal: body.include_personal,
        presentation: previdenciar_reports::Presentation {
            include_appendix: presentation.and_then(|p| p.include_appendix).unwrap_or(true),
            include_quality: presentation.and_then(|p| p.include_quality).unwrap_or(true),
            include_planning: presentation.and_then(|p| p.include_planning).unwrap_or(true),
        },
    };
    previdenciar_reports::render_pdf(&body.calculation, &body.workspace, options, Some(&outcome))
        .map_err(ApiError::bad_request)
}

async fn save_report(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ReportRequest>,
) -> Result<Json<Value>, ApiError> {
    let user = auth::authenticate(&headers, &state).await.map_err(ApiError::unauthorized)?;
    rate_limit(&state, format!("pdf:{}", user.email), 20, Duration::from_secs(3600)).await?;
    let owner = db::ensure_user(&state.db, &user).await.map_err(ApiError::internal)?;
    let store = state.storage.as_ref().ok_or_else(|| ApiError::unavailable("Armazenamento S3 não configurado.".into()))?;
    let bytes = render_report_bytes(&body)?;
    let id = Uuid::new_v4();
    let key = format!("reports/{owner}/{id}.pdf");
    store.put_bytes(&key, bytes.clone(), "application/pdf")
        .await
        .map_err(ApiError::internal)?;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    let workspace_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM workspaces WHERE owner_id = $1 ORDER BY created_at ASC LIMIT 1")
        .bind(owner)
        .fetch_optional(&state.db)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("Workspace não encontrado."))?;
    sqlx::query(
        "INSERT INTO report_artifacts (id, workspace_id, owner_id, calculation_id, format, s3_key, size_bytes, sha256)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(id)
    .bind(workspace_id)
    .bind(owner)
    .bind(body.calculation.id as i64)
    .bind(&body.format)
    .bind(&key)
    .bind(bytes.len() as i64)
    .bind(&hash)
    .execute(&state.db)
    .await
    .map_err(ApiError::internal)?;
    let _ = db::audit(&state.db, Some(workspace_id), owner, "report", Some(&id.to_string()), "save", json!({ "format": body.format })).await;
    Ok(Json(json!({
        "id": id,
        "format": body.format,
        "sizeBytes": bytes.len(),
        "sha256": hash,
        "expiresInDays": 30
    })))
}

async fn saved_reports(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let user = auth::authenticate(&headers, &state).await.map_err(ApiError::unauthorized)?;
    let owner = db::ensure_user(&state.db, &user).await.map_err(ApiError::internal)?;
    let rows = sqlx::query(
        "SELECT id, calculation_id, format, size_bytes, sha256, created_at, expires_at
         FROM report_artifacts WHERE owner_id = $1 AND expires_at > now()
         ORDER BY created_at DESC LIMIT 100",
    )
    .bind(owner)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::internal)?;
    let reports = rows.into_iter().map(|row| json!({
        "id": row.get::<Uuid, _>("id"),
        "calculationId": row.get::<Option<i64>, _>("calculation_id"),
        "format": row.get::<String, _>("format"),
        "sizeBytes": row.get::<i64, _>("size_bytes"),
        "sha256": row.get::<String, _>("sha256"),
        "createdAt": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at"),
        "expiresAt": row.get::<chrono::DateTime<chrono::Utc>, _>("expires_at")
    })).collect::<Vec<_>>();
    Ok(Json(json!({ "reports": reports })))
}

async fn saved_report_url(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let user = auth::authenticate(&headers, &state).await.map_err(ApiError::unauthorized)?;
    let owner = db::ensure_user(&state.db, &user).await.map_err(ApiError::internal)?;
    let store = state.storage.as_ref().ok_or_else(|| ApiError::unavailable("Armazenamento S3 não configurado.".into()))?;
    let key = sqlx::query_scalar::<_, String>(
        "SELECT s3_key FROM report_artifacts WHERE id = $1 AND owner_id = $2 AND expires_at > now()",
    )
    .bind(id)
    .bind(owner)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::internal)?
    .ok_or_else(|| ApiError::not_found("PDF não encontrado."))?;
    let url = store.presigned_get(&key, 300).await.map_err(ApiError::internal)?;
    Ok(Json(json!({ "url": url, "expiresInSeconds": 300 })))
}

async fn delete_saved_report(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let user = auth::authenticate(&headers, &state).await.map_err(ApiError::unauthorized)?;
    let owner = db::ensure_user(&state.db, &user).await.map_err(ApiError::internal)?;
    let store = state.storage.as_ref().ok_or_else(|| ApiError::unavailable("Armazenamento S3 não configurado.".into()))?;
    let key = sqlx::query_scalar::<_, String>(
        "SELECT s3_key FROM report_artifacts WHERE id = $1 AND owner_id = $2",
    )
    .bind(id)
    .bind(owner)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::internal)?
    .ok_or_else(|| ApiError::not_found("PDF não encontrado."))?;
    store.delete(&key).await.map_err(ApiError::internal)?;
    sqlx::query("DELETE FROM report_artifacts WHERE id = $1 AND owner_id = $2")
        .bind(id)
        .bind(owner)
        .execute(&state.db)
        .await
        .map_err(ApiError::internal)?;
    let _ = db::audit(&state.db, None, owner, "report", Some(&id.to_string()), "delete", json!({})).await;
    Ok(Json(json!({ "deleted": true })))
}

#[derive(Deserialize)]
struct SaveRequest {
    workspace: previdenciar_workspace::Workspace,
}

async fn save_workspace(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<SaveRequest>,
) -> Result<Json<previdenciar_workspace::Workspace>, ApiError> {
    let user = auth::authenticate(&headers, &state).await.map_err(ApiError::unauthorized)?;
    let owner = db::ensure_user(&state.db, &user).await.map_err(ApiError::internal)?;
    let workspace = db::save_workspace(&state.db, owner, body.workspace)
        .await
        .map_err(|error| match error {
            db::SaveError::Conflict => ApiError::conflict("A base foi alterada em outra sessÃ£o. Recarregue antes de salvar."),
            db::SaveError::NotFound => ApiError::not_found("Workspace nÃ£o encontrado."),
            db::SaveError::Database(error) => ApiError::internal(error),
            db::SaveError::Serialization(message) => ApiError::bad_request(message),
        })?;
    let _ = db::audit(&state.db, None, owner, "workspace", None, "save", json!({ "revision": workspace.revision })).await;
    Ok(Json(workspace))
}

#[derive(Deserialize)]
struct CalculateRequest {
    calculation: previdenciar_workspace::Calculation,
}

async fn calculate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CalculateRequest>,
) -> Result<Json<Value>, ApiError> {
    let user = auth::authenticate(&headers, &state).await.map_err(ApiError::unauthorized)?;
    let owner = db::ensure_user(&state.db, &user).await.map_err(ApiError::internal)?;
    let result = previdenciar_rgps::calculate_case(&body.calculation)
        .map_err(ApiError::bad_request)?;
    let _ = db::audit(&state.db, None, owner, "calculation", Some(&body.calculation.id.to_string()), "calculate", json!({ "kind": body.calculation.kind })).await;
    Ok(Json(result))
}

#[derive(Deserialize)]
struct SnapshotQuery {
    through: String,
}

async fn pcd_snapshot(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<SnapshotQuery>,
) -> Result<Json<previdenciar_rgps::pcd::Snapshot>, ApiError> {
    let _user = auth::authenticate(&headers, &state).await.map_err(ApiError::unauthorized)?;
    let snapshot = previdenciar_rgps::pcd::local_snapshot(&query.through)
        .map_err(ApiError::bad_request)?;
    Ok(Json(snapshot))
}

async fn import_cnis(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<previdenciar_cnis::ImportResult>, ApiError> {
    let _user = auth::authenticate(&headers, &state).await.map_err(ApiError::unauthorized)?;
    while let Some(field) = multipart.next_field().await.map_err(|_| ApiError::bad_request("Upload invÃ¡lido.".into()))? {
        if field.name() != Some("file") {
            continue;
        }
        let bytes = field.bytes().await.map_err(|_| ApiError::bad_request("NÃ£o foi possÃ­vel ler o PDF.".into()))?;
        if bytes.is_empty() || bytes.len() > 32 * 1024 * 1024 {
            return Err(ApiError::bad_request("O PDF deve ter atÃ© 32 MiB.".into()));
        }
        let mut file = tempfile::Builder::new()
            .prefix("previdenciar-cnis-")
            .suffix(".pdf")
            .tempfile()
            .map_err(ApiError::internal)?;
        std::io::Write::write_all(&mut file, &bytes).map_err(ApiError::internal)?;
        std::io::Write::flush(&mut file).map_err(ApiError::internal)?;
        let result = match previdenciar_cnis::import_pdf(file.path()) {
            Ok(result) => result,
            Err(previdenciar_cnis::ImportError::NoExtractableText) => {
                let extracted = previdenciar_ai::extract_case_pdf(file.path())
                    .map_err(|error| ApiError::bad_request(error.to_string()))?;
                previdenciar_cnis::import_text(&extracted.text)
                    .map_err(|error| ApiError::bad_request(error.to_string()))?
            }
            Err(error) => return Err(ApiError::bad_request(error.to_string())),
        };
        return Ok(Json(result));
    }
    Err(ApiError::bad_request("Envie o arquivo no campo file.".into()))
}

async fn case_pdf_text(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<Value>, ApiError> {
    let _user = auth::authenticate(&headers, &state).await.map_err(ApiError::unauthorized)?;
    while let Some(field) = multipart.next_field().await.map_err(|_| ApiError::bad_request("Upload invÃ¡lido.".into()))? {
        if field.name() != Some("file") {
            continue;
        }
        let file_name = field.file_name().unwrap_or("documento.pdf").to_owned();
        let bytes = field.bytes().await.map_err(|_| ApiError::bad_request("NÃ£o foi possÃ­vel ler o PDF.".into()))?;
        if bytes.is_empty() || bytes.len() > 64 * 1024 * 1024 {
            return Err(ApiError::bad_request("O PDF deve ter atÃ© 64 MiB.".into()));
        }
        let mut file = tempfile::Builder::new()
            .prefix("previdenciar-case-")
            .suffix(".pdf")
            .tempfile()
            .map_err(ApiError::internal)?;
        std::io::Write::write_all(&mut file, &bytes).map_err(ApiError::internal)?;
        std::io::Write::flush(&mut file).map_err(ApiError::internal)?;
        let extracted = previdenciar_ai::extract_case_pdf(file.path())
            .map_err(|error| ApiError::bad_request(error.to_string()))?;
        let hash = format!("{:x}", Sha256::digest(&bytes));
        return Ok(Json(json!({
            "fileName": std::path::Path::new(&file_name).file_name().and_then(|name| name.to_str()).unwrap_or("documento.pdf"),
            "text": extracted.text,
            "usedOcr": extracted.used_ocr,
            "pages": extracted.pages,
            "hashSha256": hash
        })));
    }
    Err(ApiError::bad_request("Envie o arquivo no campo file.".into()))
}

#[derive(Deserialize)]
struct ReportPresentation {
    include_appendix: Option<bool>,
    include_quality: Option<bool>,
    include_planning: Option<bool>,
}

#[derive(Deserialize)]
struct ReportRequest {
    calculation: previdenciar_workspace::Calculation,
    workspace: previdenciar_workspace::Workspace,
    format: String,
    include_personal: bool,
    presentation: Option<ReportPresentation>,
}

async fn preview_report(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ReportRequest>,
) -> Result<Response, ApiError> {
    let user = auth::authenticate(&headers, &state).await.map_err(ApiError::unauthorized)?;
    rate_limit(&state, format!("pdf:{}", user.email), 20, Duration::from_secs(3600)).await?;
    let kind_name = body.format.strip_prefix("pdf_").unwrap_or(&body.format);
    let kind = previdenciar_reports::ReportKind::parse(kind_name).map_err(ApiError::bad_request)?;
    let outcome = previdenciar_rgps::calculate_case(&body.calculation).map_err(ApiError::bad_request)?;
    let presentation = body.presentation.unwrap_or(ReportPresentation { include_appendix: None, include_quality: None, include_planning: None });
    let options = previdenciar_reports::ReportOptions {
        kind,
        include_personal: body.include_personal,
        presentation: previdenciar_reports::Presentation {
            include_appendix: presentation.include_appendix.unwrap_or(true),
            include_quality: presentation.include_quality.unwrap_or(true),
            include_planning: presentation.include_planning.unwrap_or(true),
        },
    };
    let bytes = previdenciar_reports::render_pdf(&body.calculation, &body.workspace, options, Some(&outcome))
        .map_err(ApiError::bad_request)?;
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/pdf")
        .header(header::CONTENT_DISPOSITION, format!("inline; filename=\"previdenciar-{}.pdf\"", kind.slug()))
        .body(Body::from(bytes))
        .map_err(ApiError::internal)
}

#[derive(Deserialize)]
struct PrepareRequest {
    calculation: previdenciar_workspace::Calculation,
    reference: String,
}

async fn prepare_from_cnis(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<PrepareRequest>,
) -> Result<Json<Value>, ApiError> {
    let _user = auth::authenticate(&headers, &state).await.map_err(ApiError::unauthorized)?;
    let (calculation, notes) = previdenciar_rgps::prepare_from_cnis(&body.calculation, &body.reference)
        .map_err(ApiError::bad_request)?;
    Ok(Json(json!({ "calculation": calculation, "notes": notes })))
}

async fn run_tool(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tool): Path<String>,
    Json(input): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let _user = auth::authenticate(&headers, &state).await.map_err(ApiError::unauthorized)?;
    let result = previdenciar_core::run_tool(&tool, input).map_err(ApiError::bad_request)?;
    Ok(Json(result))
}

async fn run_finance(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tool): Path<String>,
    Json(input): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let _user = auth::authenticate(&headers, &state).await.map_err(ApiError::unauthorized)?;
    let result = previdenciar_core::finance::run_finance(&tool, input)
        .map_err(ApiError::bad_request)?;
    Ok(Json(result))
}

async fn rate_limit(state: &AppState, key: String, max: usize, window: Duration) -> Result<(), ApiError> {
    let mut limits = state.limits.lock().await;
    let now = Instant::now();
    let values = limits.entry(key).or_default();
    values.retain(|time| now.duration_since(*time) < window);
    if values.len() >= max {
        return Err(ApiError::too_many("Limite temporário atingido. Tente novamente mais tarde.".into()));
    }
    values.push(now);
    Ok(())
}

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn unauthorized(message: String) -> Self {
        Self { status: StatusCode::UNAUTHORIZED, message }
    }
    fn bad_request(message: String) -> Self {
        Self { status: StatusCode::BAD_REQUEST, message }
    }
    fn forbidden(message: String) -> Self {
        Self { status: StatusCode::FORBIDDEN, message }
    }
    fn not_found(message: &str) -> Self {
        Self { status: StatusCode::NOT_FOUND, message: message.to_owned() }
    }
    fn conflict(message: &str) -> Self {
        Self { status: StatusCode::CONFLICT, message: message.to_owned() }
    }
    fn unavailable(message: String) -> Self {
        Self { status: StatusCode::SERVICE_UNAVAILABLE, message }
    }
    fn too_many(message: String) -> Self {
        Self { status: StatusCode::TOO_MANY_REQUESTS, message }
    }
    fn internal(error: impl std::fmt::Display) -> Self {
        tracing::error!(error = %error, "request failed");
        Self { status: StatusCode::INTERNAL_SERVER_ERROR, message: "NÃ£o foi possÃ­vel concluir a operaÃ§Ã£o.".into() }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({ "error": self.message }))).into_response()
    }
}






