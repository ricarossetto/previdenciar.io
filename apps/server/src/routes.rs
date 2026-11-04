use crate::{auth, db, AppState};
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    middleware::Next,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::{
    convert::Infallible,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use uuid::Uuid;

use crate::limits::{ClientIp, RateLimitError, RateWindow};

const MAX_BODY_BYTES: usize = 64 * 1024 * 1024;
const MAX_CNIS_PDF_BYTES: usize = 32 * 1024 * 1024;
const MAX_CASE_PDF_BYTES: usize = 64 * 1024 * 1024;

const LOGIN_WINDOW: RateWindow = RateWindow::new(
    20,
    Duration::from_secs(15 * 60),
    "Muitas tentativas de login a partir deste endereço. Aguarde alguns minutos e tente novamente.",
);
const CADASTRO_IP_WINDOW: RateWindow = RateWindow::new(
    3,
    Duration::from_secs(60 * 60),
    "Muitos cadastros novos a partir deste endereço. Aguarde uma hora e tente novamente.",
);
const CADASTRO_EMAIL_WINDOW: RateWindow = RateWindow::new(
    5,
    Duration::from_secs(24 * 60 * 60),
    "Este e-mail atingiu o limite de cadastros por hoje.",
);
const CNIS_WINDOW: RateWindow = RateWindow::new(
    12,
    Duration::from_secs(60 * 60),
    "Limite de importações de CNIS atingido. Aguarde uma hora e tente novamente.",
);
const CNIS_IP_WINDOW: RateWindow = RateWindow::new(
    30,
    Duration::from_secs(60 * 60),
    "Limite de importações de CNIS atingido para este endereço. Aguarde uma hora.",
);
const OCR_WINDOW: RateWindow = RateWindow::new(
    10,
    Duration::from_secs(60 * 60),
    "Limite de leituras de PDF atingido. Aguarde uma hora e tente novamente.",
);
const CALC_WINDOW: RateWindow = RateWindow::new(
    60,
    Duration::from_secs(60 * 60),
    "Limite de cálculos atingido. Aguarde uma hora e tente novamente.",
);
const TOOL_WINDOW: RateWindow = RateWindow::new(
    120,
    Duration::from_secs(60 * 60),
    "Limite de uso das ferramentas atingido. Aguarde uma hora e tente novamente.",
);
const AI_WINDOW: RateWindow = RateWindow::new(
    30,
    Duration::from_secs(60 * 60),
    "Limite de mensagens ao Copiloto atingido. Aguarde uma hora e tente novamente.",
);
const PDF_WINDOW: RateWindow = RateWindow::new(
    20,
    Duration::from_secs(60 * 60),
    "Limite de geração de PDFs atingido. Aguarde uma hora e tente novamente.",
);

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
        .route(
            "/api/reports/saved/{id}",
            get(saved_report_url).delete(delete_saved_report),
        )
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

    router = router
        .layer(axum::middleware::from_fn(json_error_envelope))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES));

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
    ip: ClientIp,
    Json(body): Json<TokenRequest>,
) -> Result<Json<Value>, ApiError> {
    enforce(
        &state,
        vec![(format!("login:ip:{}", ip.0), LOGIN_WINDOW)],
    )?;
    if state.config.cognito_domain.is_empty() {
        return Err(ApiError::internal("COGNITO_DOMAIN não configurado."));
    }
    let response = state
        .http
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
        return Err(ApiError::unauthorized(
            "Não foi possível concluir o login.".into(),
        ));
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
    ip: ClientIp,
    Json(body): Json<RefreshRequest>,
) -> Result<Json<Value>, ApiError> {
    enforce(
        &state,
        vec![(format!("login:ip:{}", ip.0), LOGIN_WINDOW)],
    )?;
    if state.config.cognito_domain.is_empty() {
        return Err(ApiError::internal("COGNITO_DOMAIN não configurado."));
    }
    if body.refresh_token.trim().is_empty() {
        return Err(ApiError::unauthorized(
            "Sessão expirada. Entre novamente.".into(),
        ));
    }
    let response = state
        .http
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
        return Err(ApiError::unauthorized(
            "Sessão expirada. Entre novamente.".into(),
        ));
    }
    let value: Value = serde_json::from_str(&text).map_err(ApiError::internal)?;
    Ok(Json(value))
}

async fn ai_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let _user = auth::authenticate(&headers, &state)
        .await
        .map_err(ApiError::unauthorized)?;
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
    let user = auth::authenticate(&headers, &state)
        .await
        .map_err(ApiError::unauthorized)?;
    enforce(
        &state,
        vec![(format!("ai:{}", user.email.to_lowercase()), AI_WINDOW)],
    )?;
    if !state.config.ai_enabled() {
        return Err(ApiError::unavailable(
            "O Copiloto ainda não foi habilitado neste servidor.".into(),
        ));
    }
    body.provider_id = state.config.ai_provider_id.clone();
    body.base_url = state.config.ai_base_url.clone();
    body.model = state.config.ai_model.clone();
    let key = state.config.ai_api_key.clone();
    let data_root = state.config.ai_data_root.clone();
    let request_id = body.request_id.clone();
    let (tx, rx) = mpsc::channel::<Value>(64);
    let cancelled = Arc::new(AtomicBool::new(false));
    let gate = state.blocking.clone();
    let stream_tx = tx.clone();
    tokio::spawn(async move {
        let worker_request_id = request_id.clone();
        let outcome = gate
            .run(move || {
                let data = previdenciar_ai::DataStore::new(data_root.into());
                if let Err(error) = previdenciar_ai::run_with_key(body, &key, &data, &cancelled, |event| {
                    if let Ok(value) = serde_json::to_value(event) {
                        let _ = tx.blocking_send(value);
                    }
                }) {
                    tracing::warn!(request = %worker_request_id, error = %error, "AI request failed");
                    let _ = tx
                        .blocking_send(json!({"type": "error", "message": error.to_string()}));
                }
            })
            .await;
        if let Err(error) = outcome {
            tracing::warn!(request = %request_id, error = %error, "AI worker failed");
            let _ = stream_tx.blocking_send(json!({
                "type": "error",
                "message": "Não foi possível concluir a consulta ao Copiloto."
            }));
        }
    });
    let stream = ReceiverStream::new(rx).map(|value| {
        Ok::<Event, Infallible>(
            Event::default()
                .json_data(value)
                .unwrap_or_else(|_| Event::default().data("{}")),
        )
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
    ip: ClientIp,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let user = auth::authenticate(&headers, &state)
        .await
        .map_err(ApiError::unauthorized)?;
    let owner = ensure_user_guarded(&state, &ip, &user).await?;
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
    ip: ClientIp,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let user = auth::authenticate(&headers, &state)
        .await
        .map_err(ApiError::unauthorized)?;
    let owner = ensure_user_guarded(&state, &ip, &user).await?;
    let (advanced, accepted) = db::session_info(&state.db, owner, &state.config.terms_version)
        .await
        .map_err(ApiError::internal)?;
    let _ = db::audit(
        &state.db,
        None,
        owner,
        "user",
        Some(&owner.to_string()),
        "login",
        json!({}),
    )
    .await;
    Ok(Json(json!({
        "user": { "email": user.email, "subject": user.subject },
        "advanced": advanced,
        "admin": user
            .admin_email
            .as_deref()
            .is_some_and(|email| state.config.is_admin(email)),
        "terms": { "version": state.config.terms_version, "accepted": accepted }
    })))
}

#[derive(Deserialize)]
struct TermsRequest {}

async fn accept_terms(
    State(state): State<AppState>,
    ip: ClientIp,
    headers: HeaderMap,
    Json(_body): Json<TermsRequest>,
) -> Result<Json<Value>, ApiError> {
    let user = auth::authenticate(&headers, &state)
        .await
        .map_err(ApiError::unauthorized)?;
    let owner = ensure_user_guarded(&state, &ip, &user).await?;
    db::accept_terms(&state.db, owner, &state.config.terms_version)
        .await
        .map_err(ApiError::internal)?;
    let _ = db::audit(
        &state.db,
        None,
        owner,
        "terms",
        Some(&state.config.terms_version),
        "accept",
        json!({}),
    )
    .await;
    Ok(Json(json!({ "accepted": true, "version": state.config.terms_version })))
}

async fn require_admin(
    headers: &HeaderMap,
    ip: &ClientIp,
    state: &AppState,
) -> Result<Uuid, ApiError> {
    let user = auth::authenticate(headers, state)
        .await
        .map_err(ApiError::unauthorized)?;
    let Some(admin_email) = user.admin_email.as_deref() else {
        return Err(ApiError::forbidden("Acesso administrativo restrito.".into()));
    };
    if !state.config.is_admin(admin_email) {
        return Err(ApiError::forbidden("Acesso administrativo restrito.".into()));
    }
    ensure_user_guarded(state, ip, &user).await
}

async fn admin_users(
    State(state): State<AppState>,
    ip: ClientIp,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let _admin = require_admin(&headers, &ip, &state).await?;
    let rows = sqlx::query(
        "SELECT u.id, u.email, COALESCE(e.advanced, true) AS advanced, u.created_at
         FROM users u LEFT JOIN entitlements e ON e.user_id = u.id
         ORDER BY u.created_at DESC LIMIT 500",
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::internal)?;
    let users = rows
        .into_iter()
        .map(|row| {
            json!({
                "id": row.get::<Uuid, _>("id"),
                "email": row.get::<String, _>("email"),
                "advanced": row.get::<bool, _>("advanced"),
                "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at")
            })
        })
        .collect::<Vec<_>>();
    Ok(Json(json!({ "users": users })))
}

#[derive(Deserialize)]
struct EntitlementRequest {
    email: String,
    advanced: bool,
}

async fn admin_entitlement(
    State(state): State<AppState>,
    ip: ClientIp,
    headers: HeaderMap,
    Json(body): Json<EntitlementRequest>,
) -> Result<Json<Value>, ApiError> {
    let admin = require_admin(&headers, &ip, &state).await?;
    let user_id =
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE lower(email) = lower($1)")
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
    let _ = db::audit(
        &state.db,
        None,
        admin,
        "entitlement",
        Some(&user_id.to_string()),
        "update",
        json!({ "advanced": body.advanced }),
    )
    .await;
    Ok(Json(json!({ "email": body.email, "advanced": body.advanced })))
}

async fn admin_audit(
    State(state): State<AppState>,
    ip: ClientIp,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let _admin = require_admin(&headers, &ip, &state).await?;
    let rows = sqlx::query(
        "SELECT id, actor_id, entity_type, entity_id, action, payload, created_at
         FROM audit_events ORDER BY created_at DESC LIMIT 200",
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::internal)?;
    let events = rows
        .into_iter()
        .map(|row| {
            json!({
                "id": row.get::<i64, _>("id"),
                "actor_id": row.get::<Option<Uuid>, _>("actor_id"),
                "entity_type": row.get::<String, _>("entity_type"),
                "entity_id": row.get::<Option<String>, _>("entity_id"),
                "action": row.get::<String, _>("action"),
                "payload": row.get::<Value, _>("payload"),
                "created_at": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at")
            })
        })
        .collect::<Vec<_>>();
    Ok(Json(json!({ "events": events })))
}

fn render_report_bytes(body: &ReportRequest) -> Result<Vec<u8>, ApiError> {
    let kind_name = body.format.strip_prefix("pdf_").unwrap_or(&body.format);
    let kind = previdenciar_reports::ReportKind::parse(kind_name).map_err(ApiError::bad_request)?;
    let outcome =
        previdenciar_rgps::calculate_case(&body.calculation).map_err(ApiError::bad_request)?;
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
    ip: ClientIp,
    headers: HeaderMap,
    Json(body): Json<ReportRequest>,
) -> Result<Json<Value>, ApiError> {
    let user = auth::authenticate(&headers, &state)
        .await
        .map_err(ApiError::unauthorized)?;
    enforce(
        &state,
        vec![(format!("pdf:{}", user.email.to_lowercase()), PDF_WINDOW)],
    )?;
    let store = state
        .storage
        .as_ref()
        .ok_or_else(|| ApiError::unavailable("Armazenamento S3 não configurado.".into()))?;
    let owner = ensure_user_guarded(&state, &ip, &user).await?;
    let format = body.format.clone();
    let calculation_id = body.calculation.id;
    let bytes = state
        .blocking
        .run(move || render_report_bytes(&body))
        .await
        .map_err(ApiError::internal)??;
    let id = Uuid::new_v4();
    let key = format!("reports/{owner}/{id}.pdf");
    store
        .put_bytes(&key, bytes.clone(), "application/pdf")
        .await
        .map_err(ApiError::internal)?;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    let workspace_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM workspaces WHERE owner_id = $1 ORDER BY created_at ASC LIMIT 1",
    )
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
    .bind(calculation_id as i64)
    .bind(&format)
    .bind(&key)
    .bind(bytes.len() as i64)
    .bind(&hash)
    .execute(&state.db)
    .await
    .map_err(ApiError::internal)?;
    let _ = db::audit(
        &state.db,
        Some(workspace_id),
        owner,
        "report",
        Some(&id.to_string()),
        "save",
        json!({ "format": format }),
    )
    .await;
    Ok(Json(json!({
        "id": id,
        "format": format,
        "sizeBytes": bytes.len(),
        "sha256": hash,
        "expiresInDays": 30
    })))
}

async fn saved_reports(
    State(state): State<AppState>,
    ip: ClientIp,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let user = auth::authenticate(&headers, &state)
        .await
        .map_err(ApiError::unauthorized)?;
    let owner = ensure_user_guarded(&state, &ip, &user).await?;
    let rows = sqlx::query(
        "SELECT id, calculation_id, format, size_bytes, sha256, created_at, expires_at
         FROM report_artifacts WHERE owner_id = $1 AND expires_at > now()
         ORDER BY created_at DESC LIMIT 100",
    )
    .bind(owner)
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::internal)?;
    let reports = rows
        .into_iter()
        .map(|row| {
            json!({
                "id": row.get::<Uuid, _>("id"),
                "calculationId": row.get::<Option<i64>, _>("calculation_id"),
                "format": row.get::<String, _>("format"),
                "sizeBytes": row.get::<i64, _>("size_bytes"),
                "sha256": row.get::<String, _>("sha256"),
                "createdAt": row.get::<chrono::DateTime<chrono::Utc>, _>("created_at"),
                "expiresAt": row.get::<chrono::DateTime<chrono::Utc>, _>("expires_at")
            })
        })
        .collect::<Vec<_>>();
    Ok(Json(json!({ "reports": reports })))
}

async fn saved_report_url(
    State(state): State<AppState>,
    ip: ClientIp,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let user = auth::authenticate(&headers, &state)
        .await
        .map_err(ApiError::unauthorized)?;
    let owner = ensure_user_guarded(&state, &ip, &user).await?;
    let store = state
        .storage
        .as_ref()
        .ok_or_else(|| ApiError::unavailable("Armazenamento S3 não configurado.".into()))?;
    let key = sqlx::query_scalar::<_, String>(
        "SELECT s3_key FROM report_artifacts WHERE id = $1 AND owner_id = $2 AND expires_at > now()",
    )
    .bind(id)
    .bind(owner)
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::internal)?
    .ok_or_else(|| ApiError::not_found("PDF não encontrado."))?;
    let url = store
        .presigned_get(&key, 300)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(json!({ "url": url, "expiresInSeconds": 300 })))
}

async fn delete_saved_report(
    State(state): State<AppState>,
    ip: ClientIp,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
    let user = auth::authenticate(&headers, &state)
        .await
        .map_err(ApiError::unauthorized)?;
    let owner = ensure_user_guarded(&state, &ip, &user).await?;
    let store = state
        .storage
        .as_ref()
        .ok_or_else(|| ApiError::unavailable("Armazenamento S3 não configurado.".into()))?;
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
    let _ = db::audit(
        &state.db,
        None,
        owner,
        "report",
        Some(&id.to_string()),
        "delete",
        json!({}),
    )
    .await;
    Ok(Json(json!({ "deleted": true })))
}

#[derive(Deserialize)]
struct SaveRequest {
    workspace: previdenciar_workspace::Workspace,
}

async fn save_workspace(
    State(state): State<AppState>,
    ip: ClientIp,
    headers: HeaderMap,
    Json(body): Json<SaveRequest>,
) -> Result<Json<previdenciar_workspace::Workspace>, ApiError> {
    let user = auth::authenticate(&headers, &state)
        .await
        .map_err(ApiError::unauthorized)?;
    let owner = ensure_user_guarded(&state, &ip, &user).await?;
    let workspace = db::save_workspace(&state.db, owner, body.workspace)
        .await
        .map_err(|error| match error {
            db::SaveError::Conflict => {
                ApiError::conflict("A base foi alterada em outra sessão. Recarregue antes de salvar.")
            }
            db::SaveError::NotFound => ApiError::not_found("Workspace não encontrado."),
            db::SaveError::Database(error) => ApiError::internal(error),
            db::SaveError::Serialization(message) => ApiError::bad_request(message),
        })?;
    let _ = db::audit(
        &state.db,
        None,
        owner,
        "workspace",
        None,
        "save",
        json!({ "revision": workspace.revision }),
    )
    .await;
    Ok(Json(workspace))
}

#[derive(Deserialize)]
struct CalculateRequest {
    calculation: previdenciar_workspace::Calculation,
}

async fn calculate(
    State(state): State<AppState>,
    ip: ClientIp,
    headers: HeaderMap,
    Json(body): Json<CalculateRequest>,
) -> Result<Json<Value>, ApiError> {
    let user = auth::authenticate(&headers, &state)
        .await
        .map_err(ApiError::unauthorized)?;
    enforce(
        &state,
        vec![(format!("calc:{}", user.email.to_lowercase()), CALC_WINDOW)],
    )?;
    let owner = ensure_user_guarded(&state, &ip, &user).await?;
    let calculation_id = body.calculation.id;
    let calculation_kind = body.calculation.kind.clone();
    let result = state
        .blocking
        .run(move || previdenciar_rgps::calculate_case(&body.calculation))
        .await
        .map_err(ApiError::internal)?
        .map_err(ApiError::bad_request)?;
    let _ = db::audit(
        &state.db,
        None,
        owner,
        "calculation",
        Some(&calculation_id.to_string()),
        "calculate",
        json!({ "kind": calculation_kind }),
    )
    .await;
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
    let user = auth::authenticate(&headers, &state)
        .await
        .map_err(ApiError::unauthorized)?;
    enforce(
        &state,
        vec![(format!("calc:{}", user.email.to_lowercase()), CALC_WINDOW)],
    )?;
    let snapshot = state
        .blocking
        .run(move || previdenciar_rgps::pcd::local_snapshot(&query.through))
        .await
        .map_err(ApiError::internal)?
        .map_err(ApiError::bad_request)?;
    Ok(Json(snapshot))
}

async fn import_cnis(
    State(state): State<AppState>,
    ip: ClientIp,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<previdenciar_cnis::ImportResult>, ApiError> {
    let user = auth::authenticate(&headers, &state)
        .await
        .map_err(ApiError::unauthorized)?;
    enforce(
        &state,
        vec![
            (
                format!("cnis:{}", user.email.to_lowercase()),
                CNIS_WINDOW,
            ),
            (format!("cnis:ip:{}", ip.0), CNIS_IP_WINDOW),
        ],
    )?;
    while let Some(field) = multipart.next_field().await.map_err(multipart_error)? {
        if field.name() != Some("file") {
            continue;
        }
        let bytes = field.bytes().await.map_err(multipart_error)?;
        if bytes.is_empty() {
            return Err(ApiError::bad_request("O PDF está vazio.".into()));
        }
        if bytes.len() > MAX_CNIS_PDF_BYTES {
            return Err(ApiError::payload_too_large(format!(
                "O PDF deve ter até {} MiB.",
                MAX_CNIS_PDF_BYTES / (1024 * 1024)
            )));
        }
        let result = state
            .blocking
            .run(move || import_cnis_bytes(bytes))
            .await
            .map_err(ApiError::internal)??;
        return Ok(Json(result));
    }
    Err(ApiError::bad_request(
        "Envie o arquivo no campo file.".into(),
    ))
}

fn import_cnis_bytes(
    bytes: axum::body::Bytes,
) -> Result<previdenciar_cnis::ImportResult, ApiError> {
    let mut file = tempfile::Builder::new()
        .prefix("previdenciar-cnis-")
        .suffix(".pdf")
        .tempfile()
        .map_err(ApiError::internal)?;
    std::io::Write::write_all(&mut file, &bytes).map_err(ApiError::internal)?;
    std::io::Write::flush(&mut file).map_err(ApiError::internal)?;
    match previdenciar_cnis::import_pdf(file.path()) {
        Ok(result) => Ok(result),
        Err(previdenciar_cnis::ImportError::NoExtractableText) => {
            let extracted = previdenciar_ai::extract_case_pdf(file.path())
                .map_err(|error| ApiError::bad_request(error.to_string()))?;
            previdenciar_cnis::import_text(&extracted.text)
                .map_err(|error| ApiError::bad_request(error.to_string()))
        }
        Err(error) => Err(ApiError::bad_request(error.to_string())),
    }
}

async fn case_pdf_text(
    State(state): State<AppState>,
    ip: ClientIp,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<Value>, ApiError> {
    let user = auth::authenticate(&headers, &state)
        .await
        .map_err(ApiError::unauthorized)?;
    enforce(
        &state,
        vec![
            (format!("ocr:{}", user.email.to_lowercase()), OCR_WINDOW),
            (format!("ocr:ip:{}", ip.0), OCR_WINDOW),
        ],
    )?;
    while let Some(field) = multipart.next_field().await.map_err(multipart_error)? {
        if field.name() != Some("file") {
            continue;
        }
        let file_name = field.file_name().unwrap_or("documento.pdf").to_owned();
        let bytes = field.bytes().await.map_err(multipart_error)?;
        if bytes.is_empty() {
            return Err(ApiError::bad_request("O PDF está vazio.".into()));
        }
        if bytes.len() > MAX_CASE_PDF_BYTES {
            return Err(ApiError::payload_too_large(format!(
                "O PDF deve ter até {} MiB.",
                MAX_CASE_PDF_BYTES / (1024 * 1024)
            )));
        }
        let extracted = state
            .blocking
            .run(move || extract_case_pdf_bytes(bytes))
            .await
            .map_err(ApiError::internal)??;
        let hash = format!("{:x}", Sha256::digest(&extracted.0));
        return Ok(Json(json!({
            "fileName": std::path::Path::new(&file_name)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("documento.pdf"),
            "text": extracted.1.text,
            "usedOcr": extracted.1.used_ocr,
            "pages": extracted.1.pages,
            "hashSha256": hash
        })));
    }
    Err(ApiError::bad_request(
        "Envie o arquivo no campo file.".into(),
    ))
}

fn extract_case_pdf_bytes(
    bytes: axum::body::Bytes,
) -> Result<(axum::body::Bytes, previdenciar_ai::ExtractedPdf), ApiError> {
    let mut file = tempfile::Builder::new()
        .prefix("previdenciar-case-")
        .suffix(".pdf")
        .tempfile()
        .map_err(ApiError::internal)?;
    std::io::Write::write_all(&mut file, &bytes).map_err(ApiError::internal)?;
    std::io::Write::flush(&mut file).map_err(ApiError::internal)?;
    let extracted = previdenciar_ai::extract_case_pdf(file.path())
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    Ok((bytes, extracted))
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
    let user = auth::authenticate(&headers, &state)
        .await
        .map_err(ApiError::unauthorized)?;
    enforce(
        &state,
        vec![(format!("pdf:{}", user.email.to_lowercase()), PDF_WINDOW)],
    )?;
    let rendered = state
        .blocking
        .run(move || preview_report_bytes(body))
        .await
        .map_err(ApiError::internal)??;
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/pdf")
        .header(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"previdenciar-{}.pdf\"", rendered.1),
        )
        .body(Body::from(rendered.0))
        .map_err(ApiError::internal)
}

fn preview_report_bytes(body: ReportRequest) -> Result<(Vec<u8>, &'static str), ApiError> {
    let kind_name = body.format.strip_prefix("pdf_").unwrap_or(&body.format);
    let kind = previdenciar_reports::ReportKind::parse(kind_name).map_err(ApiError::bad_request)?;
    let outcome =
        previdenciar_rgps::calculate_case(&body.calculation).map_err(ApiError::bad_request)?;
    let presentation = body.presentation.unwrap_or(ReportPresentation {
        include_appendix: None,
        include_quality: None,
        include_planning: None,
    });
    let options = previdenciar_reports::ReportOptions {
        kind,
        include_personal: body.include_personal,
        presentation: previdenciar_reports::Presentation {
            include_appendix: presentation.include_appendix.unwrap_or(true),
            include_quality: presentation.include_quality.unwrap_or(true),
            include_planning: presentation.include_planning.unwrap_or(true),
        },
    };
    let bytes = previdenciar_reports::render_pdf(
        &body.calculation,
        &body.workspace,
        options,
        Some(&outcome),
    )
    .map_err(ApiError::bad_request)?;
    Ok((bytes, kind.slug()))
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
    let user = auth::authenticate(&headers, &state)
        .await
        .map_err(ApiError::unauthorized)?;
    enforce(
        &state,
        vec![(format!("calc:{}", user.email.to_lowercase()), CALC_WINDOW)],
    )?;
    let (calculation, notes) = state
        .blocking
        .run(move || previdenciar_rgps::prepare_from_cnis(&body.calculation, &body.reference))
        .await
        .map_err(ApiError::internal)?
        .map_err(ApiError::bad_request)?;
    Ok(Json(json!({ "calculation": calculation, "notes": notes })))
}

async fn run_tool(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tool): Path<String>,
    Json(input): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let user = auth::authenticate(&headers, &state)
        .await
        .map_err(ApiError::unauthorized)?;
    enforce(
        &state,
        vec![(format!("tool:{}", user.email.to_lowercase()), TOOL_WINDOW)],
    )?;
    let result = state
        .blocking
        .run(move || previdenciar_core::run_tool(&tool, input))
        .await
        .map_err(ApiError::internal)?
        .map_err(ApiError::bad_request)?;
    Ok(Json(result))
}

async fn run_finance(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tool): Path<String>,
    Json(input): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let user = auth::authenticate(&headers, &state)
        .await
        .map_err(ApiError::unauthorized)?;
    enforce(
        &state,
        vec![(format!("tool:{}", user.email.to_lowercase()), TOOL_WINDOW)],
    )?;
    let result = state
        .blocking
        .run(move || previdenciar_core::finance::run_finance(&tool, input))
        .await
        .map_err(ApiError::internal)?
        .map_err(ApiError::bad_request)?;
    Ok(Json(result))
}

fn enforce(state: &AppState, entries: Vec<(String, RateWindow)>) -> Result<(), ApiError> {
    state.limits.check(&entries).map_err(ApiError::rate_limited)
}

async fn ensure_user_guarded(
    state: &AppState,
    ip: &ClientIp,
    user: &auth::AuthUser,
) -> Result<Uuid, ApiError> {
    let known = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE cognito_sub = $1")
        .bind(&user.subject)
        .fetch_optional(&state.db)
        .await
        .map_err(ApiError::internal)?;
    if known.is_none() {
        enforce(
            state,
            vec![
                (format!("cadastro:ip:{}", ip.0), CADASTRO_IP_WINDOW),
                (
                    format!("cadastro:email:{}", user.email.to_lowercase()),
                    CADASTRO_EMAIL_WINDOW,
                ),
            ],
        )?;
    }
    db::ensure_user(&state.db, user)
        .await
        .map_err(ApiError::internal)
}

fn multipart_error(error: axum::extract::multipart::MultipartError) -> ApiError {
    if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
        ApiError::payload_too_large(oversized_body_message())
    } else {
        ApiError::bad_request("Upload inválido.".into())
    }
}

fn oversized_body_message() -> String {
    format!(
        "O corpo da requisição excede o limite de {} MiB.",
        MAX_BODY_BYTES / (1024 * 1024)
    )
}

async fn json_error_envelope(
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let response = next.run(request).await;
    let status = response.status();
    let message = match status {
        StatusCode::PAYLOAD_TOO_LARGE => Some(oversized_body_message()),
        StatusCode::BAD_REQUEST => {
            Some("Não foi possível interpretar a requisição. Verifique os dados enviados.".to_owned())
        }
        StatusCode::UNPROCESSABLE_ENTITY => {
            Some("Os dados enviados não correspondem ao formato esperado.".to_owned())
        }
        StatusCode::UNSUPPORTED_MEDIA_TYPE => {
            Some("Tipo de conteúdo não suportado. Envie JSON ou multipart/form-data.".to_owned())
        }
        StatusCode::METHOD_NOT_ALLOWED => {
            Some("Método não permitido para esta rota.".to_owned())
        }
        _ => None,
    };
    let Some(message) = message else {
        return response;
    };
    if response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("application/json"))
    {
        return response;
    }
    (status, Json(json!({ "error": message }))).into_response()
}

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: String,
    retry_after: Option<Duration>,
}

impl ApiError {
    fn unauthorized(message: String) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message,
            retry_after: None,
        }
    }
    fn bad_request(message: String) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message,
            retry_after: None,
        }
    }
    fn forbidden(message: String) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            message,
            retry_after: None,
        }
    }
    fn not_found(message: &str) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.to_owned(),
            retry_after: None,
        }
    }
    fn conflict(message: &str) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            message: message.to_owned(),
            retry_after: None,
        }
    }
    fn unavailable(message: String) -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            message,
            retry_after: None,
        }
    }
    fn payload_too_large(message: String) -> Self {
        Self {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            message,
            retry_after: None,
        }
    }
    fn rate_limited(error: RateLimitError) -> Self {
        Self {
            status: StatusCode::TOO_MANY_REQUESTS,
            message: error.message,
            retry_after: Some(error.retry_after),
        }
    }
    fn internal(error: impl std::fmt::Display) -> Self {
        tracing::error!(error = %error, "request failed");
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "Não foi possível concluir a operação.".into(),
            retry_after: None,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut response = (self.status, Json(json!({ "error": self.message }))).into_response();
        if let Some(retry_after) = self.retry_after {
            let seconds = retry_after.as_secs().max(1);
            if let Ok(value) = header::HeaderValue::from_str(&seconds.to_string()) {
                response.headers_mut().insert(header::RETRY_AFTER, value);
            }
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocking::BlockingGate;
    use crate::config;
    use crate::limits::RateLimiter;
    use std::net::SocketAddr;

    fn test_state(config: config::Config) -> AppState {
        let db = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://teste:inexistente@127.0.0.1:9/sem_banco")
            .expect("url de banco valida");
        AppState {
            config: Arc::new(config),
            db,
            http: reqwest::Client::new(),
            jwks: Arc::new(tokio::sync::RwLock::new(None)),
            storage: None,
            limits: Arc::new(RateLimiter::new()),
            blocking: Arc::new(BlockingGate::new()),
        }
    }

    fn test_state_with_db(config: config::Config, db: sqlx::PgPool) -> AppState {
        AppState {
            config: Arc::new(config),
            db,
            http: reqwest::Client::new(),
            jwks: Arc::new(tokio::sync::RwLock::new(None)),
            storage: None,
            limits: Arc::new(RateLimiter::new()),
            blocking: Arc::new(BlockingGate::new()),
        }
    }

    async fn spawn_server(state: AppState) -> String {
        let app = crate::security_headers(router(state));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("porta livre");
        let addr = listener.local_addr().expect("endereco do listener");
        tokio::spawn(async move {
            let _ = axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await;
        });
        format!("http://{addr}")
    }

    fn error_message(body: &Value) -> String {
        body.get("error")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_owned()
    }

    fn assert_header(headers: &HeaderMap, name: &str, expected: &str) {
        let value = headers
            .get(name)
            .unwrap_or_else(|| panic!("header {name} ausente na resposta"))
            .to_str()
            .unwrap_or_else(|error| panic!("header {name} com valor inválido: {error}"));
        assert_eq!(value, expected, "valor inesperado no header {name}");
    }

    fn multipart_body(boundary: &str, file: &[u8]) -> Vec<u8> {
        let mut body = Vec::new();
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            b"Content-Disposition: form-data; name=\"file\"; filename=\"documento.pdf\"\r\n",
        );
        body.extend_from_slice(b"Content-Type: application/pdf\r\n\r\n");
        body.extend_from_slice(file);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        body
    }

    /// Banco de teste isolado.
    ///
    /// Usa o mesmo helper de `integration_db`: exige `TEST_DATABASE_URL`
    /// explícita, com host literal de loopback e banco `previdenciar_test_*`.
    /// Sem banco válido o teste falha antes de SQL; não pula.
    async fn test_db() -> crate::integration_db::BancoTemporario {
        crate::integration_db::abrir_banco()
            .await
            .expect("TEST_DATABASE_URL de banco isolado válido é obrigatória")
    }

    #[tokio::test]
    async fn auth_invalido_retorna_401_json() {
        let mut config = config::Config::for_tests();
        config.auth_disabled = false;
        config.cognito_domain = "dominio.invalido".to_owned();
        let base = spawn_server(test_state(config)).await;
        let client = reqwest::Client::new();

        let response = client
            .get(format!("{base}/api/me"))
            .send()
            .await
            .expect("resposta do servidor");
        assert_eq!(response.status(), 401);
        assert!(response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("application/json")));
        let body: Value = response.json().await.expect("erro em json");
        assert_eq!(error_message(&body), "Sessão ausente.");

        let response = client
            .get(format!("{base}/api/me"))
            .header(header::AUTHORIZATION, "Bearer token-invalido")
            .send()
            .await
            .expect("resposta do servidor");
        assert_eq!(response.status(), 401);
        let body: Value = response.json().await.expect("erro em json");
        assert_eq!(error_message(&body), "Token inválido.");
    }

    #[tokio::test]
    async fn rate_limit_de_login_retorna_429_com_retry_after() {
        let mut config = config::Config::for_tests();
        config.cognito_domain = String::new();
        let base = spawn_server(test_state(config)).await;
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "code": "codigo-teste",
            "code_verifier": "verificador-teste",
            "redirect_uri": "http://127.0.0.1/auth/callback"
        });
        for attempt in 0..LOGIN_WINDOW.max {
            let response = client
                .post(format!("{base}/api/auth/token"))
                .json(&body)
                .send()
                .await
                .expect("resposta do servidor");
            assert_eq!(response.status(), 500, "tentativa {attempt}");
        }
        let response = client
            .post(format!("{base}/api/auth/token"))
            .json(&body)
            .send()
            .await
            .expect("resposta do servidor");
        assert_eq!(response.status(), 429);
        assert!(response.headers().get(header::RETRY_AFTER).is_some());
        let body: Value = response.json().await.expect("erro em json");
        assert_eq!(error_message(&body), LOGIN_WINDOW.message);
    }

    #[tokio::test]
    async fn rate_limit_de_cnis_retorna_429() {
        let base = spawn_server(test_state(config::Config::for_tests())).await;
        let client = reqwest::Client::new();
        for attempt in 0..CNIS_WINDOW.max {
            let response = client
                .post(format!("{base}/api/cnis/import"))
                .header(
                    header::CONTENT_TYPE,
                    "multipart/form-data; boundary=fronteira-teste",
                )
                .body(multipart_body("fronteira-teste", b"nao-e-pdf"))
                .send()
                .await
                .expect("resposta do servidor");
            assert_eq!(response.status(), 400, "tentativa {attempt}");
        }
        let response = client
            .post(format!("{base}/api/cnis/import"))
            .header(
                header::CONTENT_TYPE,
                "multipart/form-data; boundary=fronteira-teste",
            )
            .body(multipart_body("fronteira-teste", b"nao-e-pdf"))
            .send()
            .await
            .expect("resposta do servidor");
        assert_eq!(response.status(), 429);
        let body: Value = response.json().await.expect("erro em json");
        assert_eq!(error_message(&body), CNIS_WINDOW.message);
    }

    #[tokio::test]
    async fn payload_acima_do_limite_retorna_413_json() {
        let base = spawn_server(test_state(config::Config::for_tests())).await;
        let mut body = Vec::with_capacity(MAX_BODY_BYTES + 2048);
        body.extend_from_slice(b"{\"calculation\":");
        body.resize(MAX_BODY_BYTES + 1024, b'x');
        let response = reqwest::Client::new()
            .post(format!("{base}/api/calculate"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(body)
            .send()
            .await
            .expect("resposta do servidor");
        assert_eq!(response.status(), 413);
        assert!(response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("application/json")));
        let body: Value = response.json().await.expect("erro em json");
        assert_eq!(error_message(&body), oversized_body_message());
    }

    #[test]
    fn erro_interno_nao_ecoa_chave_sintetica() {
        let erro = ApiError::internal("falha com CHAVEFABRICAMARCADOR");
        assert_eq!(
            erro.message,
            "Não foi possível concluir a operação."
        );
        assert!(!erro.message.contains("CHAVEFABRICAMARCADOR"));
    }

    #[tokio::test]
    async fn admin_sem_permissao_retorna_403_json() {
        let base = spawn_server(test_state(config::Config::for_tests())).await;
        let response = reqwest::Client::new()
            .get(format!("{base}/api/admin/users"))
            .header("x-dev-user", "usuario@teste.local")
            .send()
            .await
            .expect("resposta do servidor");
        assert_eq!(response.status(), 403);
        let body: Value = response.json().await.expect("erro em json");
        assert_eq!(error_message(&body), "Acesso administrativo restrito.");
    }

    #[tokio::test]
    async fn copiloto_sse_entrega_evento_erro() {
        let mut config = config::Config::for_tests();
        config.ai_api_key = "chave-teste".to_owned();
        config.ai_provider_id = "provedor-inexistente".to_owned();
        let base = spawn_server(test_state(config)).await;
        let request = serde_json::json!({
            "requestId": "req-teste-1",
            "providerId": "opencode-go",
            "baseUrl": "http://127.0.0.1:9",
            "model": "modelo-teste",
            "messages": [{ "role": "user", "content": "Olá" }],
            "consent": true,
            "toolsEnabled": false,
            "maxTokens": 256,
            "caseContext": null
        });
        let response = reqwest::Client::new()
            .post(format!("{base}/api/ai/chat"))
            .json(&request)
            .send()
            .await
            .expect("resposta do servidor");
        assert_eq!(response.status(), 200);
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_owned();
        assert!(
            content_type.starts_with("text/event-stream"),
            "content-type: {content_type}"
        );
        let body = response.text().await.expect("corpo sse");
        assert!(
            body.contains("\"type\":\"error\""),
            "corpo SSE sem evento de erro: {body}"
        );
    }

    #[tokio::test]
    async fn upload_s3_sem_configuracao_retorna_503_e_pdf_tem_limite() {
        let base = spawn_server(test_state(config::Config::for_tests())).await;
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "calculation": serde_json::to_value(previdenciar_workspace::Calculation::default())
                .expect("calculo serializado"),
            "workspace": serde_json::to_value(previdenciar_workspace::Workspace::default())
                .expect("workspace serializado"),
            "format": "pdf_resumo",
            "include_personal": false,
            "presentation": null
        });
        for attempt in 0..PDF_WINDOW.max {
            let response = client
                .post(format!("{base}/api/reports/save"))
                .json(&body)
                .send()
                .await
                .expect("resposta do servidor");
            assert_eq!(response.status(), 503, "tentativa {attempt}");
            let parsed: Value = response.json().await.expect("erro em json");
            assert_eq!(error_message(&parsed), "Armazenamento S3 não configurado.");
        }
        let response = client
            .post(format!("{base}/api/reports/save"))
            .json(&body)
            .send()
            .await
            .expect("resposta do servidor");
        assert_eq!(response.status(), 429);
        let parsed: Value = response.json().await.expect("erro em json");
        assert_eq!(error_message(&parsed), PDF_WINDOW.message);
    }

    #[tokio::test]
    async fn upload_acima_do_limite_de_arquivo_retorna_413_json() {
        let base = spawn_server(test_state(config::Config::for_tests())).await;
        let file = vec![b'x'; MAX_CNIS_PDF_BYTES + 1024];
        let response = reqwest::Client::new()
            .post(format!("{base}/api/cnis/import"))
            .header(
                header::CONTENT_TYPE,
                "multipart/form-data; boundary=fronteira-teste",
            )
            .body(multipart_body("fronteira-teste", &file))
            .send()
            .await
            .expect("resposta do servidor");
        assert_eq!(response.status(), 413);
        let body: Value = response.json().await.expect("erro em json");
        assert_eq!(
            error_message(&body),
            format!(
                "O PDF deve ter até {} MiB.",
                MAX_CNIS_PDF_BYTES / (1024 * 1024)
            )
        );
    }

    #[tokio::test]
    async fn cadastro_novo_respeita_limite_por_ip() {
        let banco = test_db().await;
        let base = spawn_server(test_state_with_db(
            config::Config::for_tests(),
            banco.pool().clone(),
        ))
        .await;
        let client = reqwest::Client::new();
        for attempt in 0..CADASTRO_IP_WINDOW.max {
            let email = format!("novo-{}@teste.local", Uuid::new_v4());
            let response = client
                .get(format!("{base}/api/me"))
                .header("x-dev-user", email)
                .send()
                .await
                .expect("resposta do servidor");
            assert_eq!(response.status(), 200, "tentativa {attempt}");
        }
        let response = client
            .get(format!("{base}/api/me"))
            .header("x-dev-user", format!("novo-{}@teste.local", Uuid::new_v4()))
            .send()
            .await
            .expect("resposta do servidor");
        assert_eq!(response.status(), 429);
        let body: Value = response.json().await.expect("erro em json");
        assert_eq!(error_message(&body), CADASTRO_IP_WINDOW.message);

        banco.descartar().await;
    }

    #[tokio::test]
    async fn admin_com_permissao_lista_usuarios() {
        let banco = test_db().await;
        let base = spawn_server(test_state_with_db(
            config::Config::for_tests(),
            banco.pool().clone(),
        ))
        .await;
        let response = reqwest::Client::new()
            .get(format!("{base}/api/admin/users"))
            .header("x-dev-user", "admin@teste.local")
            .send()
            .await
            .expect("resposta do servidor");
        assert_eq!(response.status(), 200);
        let body: Value = response.json().await.expect("lista em json");
        assert!(body.get("users").is_some_and(Value::is_array));

        banco.descartar().await;
    }

    #[tokio::test]
    async fn headers_de_seguranca_em_api_e_arquivo_estatico() {
        let dist = tempfile::tempdir().expect("diretorio temporario para arquivos estaticos");
        std::fs::write(
            dist.path().join("index.html"),
            "<!doctype html><html><body>pagina estatica</body></html>",
        )
        .expect("index.html de teste");
        let mut config = config::Config::for_tests();
        config.web_dist = Some(dist.path().to_string_lossy().into_owned());
        let base = spawn_server(test_state(config)).await;
        let client = reqwest::Client::new();

        let response = client
            .get(format!("{base}/api/health"))
            .send()
            .await
            .expect("resposta do servidor");
        assert_eq!(response.status(), 200);
        let headers = response.headers();
        assert_header(
            headers,
            "content-security-policy",
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; frame-src blob:",
        );
        assert_header(
            headers,
            "strict-transport-security",
            "max-age=31536000; includeSubDomains",
        );
        assert_header(headers, "cache-control", "no-store");
        assert_header(headers, "x-content-type-options", "nosniff");
        assert_header(
            headers,
            "referrer-policy",
            "strict-origin-when-cross-origin",
        );
        assert_header(headers, "x-frame-options", "DENY");

        let response = client
            .get(format!("{base}/index.html"))
            .send()
            .await
            .expect("resposta do servidor");
        assert_eq!(response.status(), 200);
        let headers = response.headers().clone();
        let body = response.text().await.expect("corpo do arquivo estatico");
        assert!(body.contains("pagina estatica"), "corpo: {body}");
        assert_header(
            &headers,
            "content-security-policy",
            "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; frame-src blob:",
        );
        assert_header(
            &headers,
            "strict-transport-security",
            "max-age=31536000; includeSubDomains",
        );
        assert_header(&headers, "x-content-type-options", "nosniff");
        assert_header(
            &headers,
            "referrer-policy",
            "strict-origin-when-cross-origin",
        );
        assert_header(&headers, "x-frame-options", "DENY");
        let cache_control = headers
            .get(header::CACHE_CONTROL)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        assert_ne!(
            cache_control, "no-store",
            "arquivo estatico nao deve receber Cache-Control: no-store"
        );
    }
}
