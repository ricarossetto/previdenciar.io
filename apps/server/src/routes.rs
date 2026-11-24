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
        .route("/api/auth/atrium-login", post(auth_atrium_login))
        .route(
            "/api/atrium/contacts",
            get(atrium_list_contacts).post(atrium_send_contact),
        )
        .route(
            "/api/atrium/processes",
            get(atrium_list_processes).post(atrium_send_process),
        )
        .route("/api/bootstrap", get(bootstrap))
        .route("/api/me", get(me))
        .route("/api/terms/accept", post(accept_terms))
        .route("/api/admin/users", get(admin_users))
        .route("/api/admin/entitlement", post(admin_entitlement))
        .route("/api/admin/audit", get(admin_audit))
        .route("/api/reports/save", post(save_report))
        .route("/api/reports/saved", get(saved_reports))
        .route("/api/reports/saved/{id}/file", get(saved_report_file))
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
        .route("/api/reports/export", post(export_report_text))
        .route("/api/prepare-from-cnis", post(prepare_from_cnis))
        .route("/api/ai/status", get(ai_status))
        .route("/api/ai/chat", post(ai_chat))
        .route("/api/ai/data-status", get(ai_data_status))
        .route("/api/ai/knowledge/search", post(ai_knowledge_search))
        .route("/api/ai/knowledge/read", post(ai_knowledge_read))
        .route("/api/fabrica/status", get(fabrica_status))
        .route("/api/fabrica/rmi", post(fabrica_rmi))
        .route("/api/fabrica/tempo", post(fabrica_tempo))
        .route("/api/fabrica/tempo/por-grupos", post(fabrica_tempo_grupos))
        .route("/api/fabrica/atrasados", post(fabrica_atrasados))
        .route("/api/fabrica/documentos/{tipo}", post(fabrica_documento))
        .route("/api/fabrica/indices/{nome}", get(fabrica_indice))
        .route("/api/fabrica/fundamentos", get(fabrica_fundamentos))
        .route("/api/fabrica/marcos-temporais", get(fabrica_marcos_temporais))
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

#[derive(Deserialize)]
struct AtriumLoginRequest {
    username: String,
    password: String,
    #[serde(default)]
    code: String,
}

async fn auth_atrium_login(
    State(state): State<AppState>,
    ip: ClientIp,
    Json(body): Json<AtriumLoginRequest>,
) -> Result<Json<Value>, ApiError> {
    enforce(
        &state,
        vec![(format!("login:ip:{}", ip.0), LOGIN_WINDOW)],
    )?;
    if !state.config.atrium_sso_enabled() {
        return Err(ApiError::unavailable(
            "O login integrado com o Atrium ainda não foi habilitado neste servidor.".into(),
        ));
    }
    let identifier = body.username.trim();
    if identifier.is_empty() || body.password.is_empty() {
        return Err(ApiError::bad_request(
            "Informe seu usuário ou e-mail e a senha do Atrium.".into(),
        ));
    }
    let login_url = format!("{}/api/auth/login", state.config.atrium_auth_base_url);
    let response = state
        .http
        .post(&login_url)
        .json(&json!({
            "username": identifier,
            "password": body.password,
            "code": body.code.trim(),
        }))
        .send()
        .await
        .map_err(|_| {
            ApiError::unavailable(
                "Não foi possível conectar ao servidor do Atrium no momento.".into(),
            )
        })?;
    let status = response.status();
    let headers = response.headers().clone();
    let text = response.text().await.map_err(ApiError::internal)?;
    let payload: Value = serde_json::from_str(&text).unwrap_or_else(|_| json!({}));
    if !status.is_success() {
        let message = payload
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("Usuário, senha ou código de verificação do Atrium inválido.");
        return Err(ApiError::unauthorized(message.to_owned()));
    }

    let user_obj = payload.get("user").unwrap_or(&Value::Null);
    let username = user_obj
        .get("username")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(identifier)
        .to_lowercase();
    let workspace_id = user_obj
        .get("workspaceId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("default")
        .to_owned();
    let email = user_obj
        .get("email")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_lowercase());

    // Consulta a situação da assinatura do escritório no Atrium quando o cookie de sessão está presente.
    // Sem confirmação da assinatura, o acesso fica bloqueado.
    let mut subscription_checked = false;
    let mut subscription_active = false;
    let mut plan: Option<String> = None;
    let session_cookie = headers
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|val| val.to_str().ok())
        .filter_map(|raw| raw.split(';').next())
        .collect::<Vec<_>>()
        .join("; ");
    if !session_cookie.is_empty() {
        let billing_url = format!("{}/api/billing/status", state.config.atrium_auth_base_url);
        if let Ok(billing_res) = state
            .http
            .get(&billing_url)
            .header(header::COOKIE, session_cookie)
            .send()
            .await
        {
            if billing_res.status().is_success() {
                if let Ok(billing_json) = billing_res.json::<Value>().await {
                    let access_mode = billing_json
                        .get("access")
                        .and_then(|acc| acc.get("access"))
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let plan_limited = billing_json
                        .get("access")
                        .and_then(|acc| acc.get("planLimited"))
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    let raw_plan = billing_json
                        .get("plan")
                        .and_then(Value::as_str)
                        .unwrap_or("pro");
                    subscription_checked = true;
                    subscription_active = access_mode == "full";
                    plan = Some(if plan_limited {
                        "basic".to_owned()
                    } else {
                        raw_plan.to_owned()
                    });
                }
            }
        }
    }

    if !subscription_checked {
        return Err(ApiError::forbidden(
            "Não foi possível confirmar sua assinatura no Atrium. Tente de novo em instantes.".into(),
        ));
    }
    if !subscription_active {
        return Err(ApiError::forbidden(
            "Sua assinatura do Atrium está com pendência. Regularize no Atrium para acessar o módulo Previdenciar.".into(),
        ));
    }
    if plan.as_deref() == Some("basic") {
        return Err(ApiError::forbidden(
            "O módulo Previdenciar faz parte dos planos Atrium Pro e Pro IA. Atualize seu plano no Atrium.".into(),
        ));
    }

    let sub = format!("atrium:{workspace_id}:{username}");
    let id_token = auth::mint_atrium_hs256(
        &state.config.atrium_sso_secret,
        sub,
        email,
        Some(username),
        subscription_active,
        plan,
    )
    .map_err(ApiError::internal)?;

    Ok(Json(json!({
        "id_token": id_token,
        "token_type": "Bearer",
        "expires_in": auth::ATRIUM_SSO_TTL_SECS
    })))
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

fn constrained(value: &str, numeric: bool) -> String {
    if value
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '-' | '/') || (numeric && matches!(c, '.' | ',' | '+')))
    {
        value.to_owned()
    } else {
        String::new()
    }
}

fn private_case(calculation: &previdenciar_workspace::Calculation) -> previdenciar_workspace::Calculation {
    let mut case = calculation.clone();
    case.name.clear();
    case.notes.clear();
    case.atrium_id = None;
    case.scenario.clear();
    case.result = None;
    case.fabrica = None;
    case.dib = constrained(&case.dib, false);
    case.identity = Default::default();
    case.sex = None;
    case.disability = None;
    case.options = Default::default();
    for link in &mut case.manual_links {
        link.employer.clear();
        link.indicators.clear();
        link.regime.clear();
        link.start_date = constrained(&link.start_date, false);
        link.end_date = constrained(&link.end_date, false);
    }
    for salary in &mut case.manual_salaries {
        salary.indicators.clear();
        salary.amount = constrained(&salary.amount, true);
        salary.competence = constrained(&salary.competence, false);
    }
    if let Some(cnis) = &mut case.cnis {
        cnis.identity = Default::default();
        cnis.metadata.extractor.clear();
        cnis.metadata.source_sha256 = None;
        for link in &mut cnis.links {
            link.raw_employer = None;
            link.affiliation = None;
            link.indicators.clear();
            link.start_date = link.start_date.as_deref().map(|v| constrained(v, false));
            link.end_date = link.end_date.as_deref().map(|v| constrained(v, false));
            link.reported_last_salary_month = link
                .reported_last_salary_month
                .as_deref()
                .map(|v| constrained(v, false));
            if let Some(end) = &mut link.inferred_last_salary_end {
                end.competence = constrained(&end.competence, false);
                end.date_candidate = constrained(&end.date_candidate, false);
                end.provenance.clear();
            }
        }
        for salary in &mut cnis.salary_entries {
            salary.indicators.clear();
            salary.competence = constrained(&salary.competence, false);
        }
        for finding in cnis.findings.iter_mut().chain(cnis.warnings.iter_mut()) {
            finding.code = "source_finding".into();
            finding.message.clear();
        }
    }
    case
}

fn html_escape_value(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn recomputed_rgps_report(case: &previdenciar_workspace::Calculation) -> String {
    let outcome = match previdenciar_rgps::calculate_case(case) {
        Ok(v) => v,
        Err(_) => {
            return "<h2>Simulação técnica</h2><p>Não foi possível recalcular a hipótese. Nenhuma RMI armazenada foi aceita como resultado verificado.</p>".into();
        }
    };
    if outcome["status"] != "experimental" {
        return "<h2>Simulação técnica bloqueada</h2><p>Parâmetros ou origem não revisados. Nenhum valor financeiro foi gerado neste relatório.</p>".into();
    }
    let Some(summary) = outcome.pointer("/data/native_result/resumo") else {
        return "<p>O motor não retornou um resumo de cálculo completo.</p>".into();
    };
    let mut html = String::from(
        "<h2>Simulação técnica recalculada em Rust</h2><p class=\"warning\">EXPERIMENTAL: resultado do motor próprio com parâmetros explícitos. Não certifica legislação, enquadramento ou concessão. Conferência profissional obrigatória.</p><table><tr><th>Grandeza</th><th>Valor do motor</th></tr>",
    );
    for (key, label) in [
        ("rmi", "RMI da hipótese"),
        ("rmi_bruta", "RMI antes de limites"),
        ("salario_beneficio", "Salário de benefício"),
        ("coeficiente", "Coeficiente informado"),
        ("competencias", "Competências"),
        ("carencia_meses", "Carência calculada em meses"),
    ] {
        if let Some(v) = summary.get(key) {
            let value = v.as_str().map(str::to_owned).unwrap_or_else(|| v.to_string());
            html.push_str(&format!(
                "<tr><td>{label}</td><td>{}</td></tr>",
                html_escape_value(&value)
            ));
        }
    }
    html.push_str("</table><h3>Memória e avisos</h3><ul>");
    if let Some(warnings) = outcome["warnings"].as_array() {
        for v in warnings {
            if let Some(s) = v.as_str() {
                html.push_str(&format!("<li>{}</li>", html_escape_value(s)));
            }
        }
    }
    html.push_str("</ul>");
    html
}

fn reviewed_report_outcome(calculation: &previdenciar_workspace::Calculation) -> Value {
    match previdenciar_rgps::calculate_case(calculation) {
        Ok(value) => value,
        Err(_) => json!({
            "status": "blocked",
            "data": null,
            "legal_validation_performed": false,
            "warnings": ["Não foi possível recalcular a hipótese; nenhum resultado armazenado foi aceito."],
            "blockers": [{
                "code": "report_input_unresolved",
                "message": "Dados ou política ainda não permitem executar o motor."
            }]
        }),
    }
}

fn render_report_for_request(body: &ReportRequest) -> Result<(Vec<u8>, &'static str), ApiError> {
    let kind_name = body.format.strip_prefix("pdf_").unwrap_or(&body.format);
    let kind = previdenciar_reports::ReportKind::parse(kind_name).map_err(ApiError::bad_request)?;
    let outcome = reviewed_report_outcome(&body.calculation);
    let is_pcd_lc142 = outcome
        .pointer("/data/native_result/engine_id")
        .and_then(Value::as_str)
        == Some("lc142-constant-grade-v1");
    let bytes = if kind.has_typst_template() && !is_pcd_lc142 {
        previdenciar_reports::render_report(
            &body.calculation,
            kind.slug(),
            Some(&outcome),
            &body.workspace,
            body.include_personal,
        )
        .map_err(ApiError::bad_request)?
    } else {
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
            .map_err(ApiError::bad_request)?
    };
    Ok((bytes, kind.slug()))
}

fn render_report_bytes(body: &ReportRequest) -> Result<Vec<u8>, ApiError> {
    render_report_for_request(body).map(|(bytes, _)| bytes)
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

async fn saved_report_file(
    State(state): State<AppState>,
    ip: ClientIp,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
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
    let bytes = store.get_bytes(&key).await.map_err(ApiError::internal)?;
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/pdf")
        .header(header::CACHE_CONTROL, "private, no-store")
        .header(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"relatorio-{id}.pdf\""),
        )
        .body(Body::from(bytes))
        .map_err(ApiError::internal)
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
    if query.through.len() != 7
        || !query.through.is_ascii()
        || chrono::NaiveDate::parse_from_str(&(query.through.clone() + "-01"), "%Y-%m-%d").is_err()
    {
        return Err(ApiError::bad_request(
            "Informe uma competência AAAA-MM válida.".into(),
        ));
    }
    if let Ok(live) = fetch_live_pcd_snapshot(&state, &query.through).await {
        return Ok(Json(live));
    }
    let snapshot = state
        .blocking
        .run(move || previdenciar_rgps::pcd::local_snapshot(&query.through))
        .await
        .map_err(ApiError::internal)?
        .map_err(ApiError::bad_request)?;
    Ok(Json(snapshot))
}

async fn fetch_live_pcd_snapshot(
    state: &AppState,
    through: &str,
) -> Result<previdenciar_rgps::pcd::Snapshot, String> {
    use previdenciar_rgps::pcd::{canonical_snapshot_decimal, Limits, Rate, Snapshot};
    let base = state.config.fabrica_base_url.trim_end_matches('/');
    let tabela_url = format!("{base}/indices/api/v1/tabela?competenciaInicial=1994-07&competenciaFinal={through}");
    let limites_url = format!("{base}/indices/api/v1/limites?competenciaInicial=1994-07&competenciaFinal={through}");
    let req_builder = |url: &str| {
        let mut r = state
            .http
            .get(url)
            .timeout(Duration::from_secs(6))
            .header(header::ACCEPT, "application/json")
            .header(header::ACCEPT_LANGUAGE, "pt-BR,pt;q=0.9");
        if state.config.fabrica_enabled() {
            r = r.header("x-api-key", state.config.fabrica_api_key.as_str());
        }
        r
    };
    let idx_bytes = req_builder(&tabela_url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    let lim_bytes = req_builder(&limites_url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    hash.update(&idx_bytes);
    hash.update(b"\n");
    hash.update(&lim_bytes);
    let digest = format!("{:x}", hash.finalize());
    let j: Value = serde_json::from_slice(&idx_bytes).map_err(|e| e.to_string())?;
    let l: Value = serde_json::from_slice(&lim_bytes).map_err(|e| e.to_string())?;
    if j["status"] != "ok" || l["status"] != "ok" {
        return Err("Provedor público não confirmou índices.".into());
    }
    let dec_val = |v: &Value| -> Result<String, String> {
        match v {
            Value::Number(n) => canonical_snapshot_decimal(&n.to_string()),
            Value::String(s) if !s.trim().is_empty() => canonical_snapshot_decimal(&s.trim().replace(',', ".")),
            _ => Err("Valor decimal ausente.".into()),
        }
    };
    let opt_val = |v: &Value| -> Result<Option<String>, String> {
        if v.is_null() || v.as_str() == Some("") { Ok(None) } else { dec_val(v).map(Some) }
    };
    let mut rates = Vec::new();
    for (m, v) in j["result"].as_object().ok_or("Tabela de índices inválida.")? {
        rates.push(Rate {
            month: m.clone(),
            inpc: dec_val(&v["inpc"])?,
            igpdi: opt_val(&v["igpdi"])?,
            ipcr: opt_val(&v["ipcr"])?,
            inpc_inss: opt_val(&v["inpcInss"])?,
        });
    }
    let mut limits = Vec::new();
    for v in l["result"].as_array().ok_or("Tabela de limites inválida.")? {
        limits.push(Limits {
            month: v["competencia"].as_str().ok_or("Competência ausente.")?.into(),
            minimum: dec_val(&v["salarioMinimo"])?,
            ceiling: dec_val(&v["tetoContribuicao"])?,
            minimum_source: v["fontePiso"].as_str().ok_or("Fonte piso ausente.")?.into(),
            ceiling_source: v["fonteTeto"].as_str().ok_or("Fonte teto ausente.")?.into(),
        });
    }
    let snapshot = Snapshot {
        source_url: tabela_url,
        retrieved_at: chrono::Utc::now().to_rfc3339(),
        response_sha256: digest,
        through: through.to_string(),
        rates,
        limits,
    };
    snapshot.validate()?;
    Ok(snapshot)
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
        Err(previdenciar_cnis::ImportError::NoExtractableText)
        | Err(previdenciar_cnis::ImportError::NotCnis) => {
            let extracted = previdenciar_ai::extract_case_pdf(file.path())
                .map_err(|error| ApiError::bad_request(error.to_string()))?;
            let mut imported = previdenciar_cnis::import_text(&extracted.text)
                .map_err(|error| ApiError::bad_request(error.to_string()))?;
            if extracted.used_ocr {
                imported.warnings.push(previdenciar_cnis::Finding {
                    code: "cnis_ocr_fallback".into(),
                    severity: previdenciar_cnis::Severity::Warning,
                    message: "Extrato CNIS lido via OCR automático; confira os números antes de protocolar.".into(),
                    count: 1,
                    source: None,
                });
            }
            Ok(imported)
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
    render_report_for_request(&body)
}

#[derive(Deserialize)]
struct ExportTextRequest {
    calculation: previdenciar_workspace::Calculation,
    workspace: previdenciar_workspace::Workspace,
    format: String,
    include_personal: bool,
}

fn summary_json(calculation: &previdenciar_workspace::Calculation) -> Value {
    let counts = calculation.cnis.as_ref().map(|cnis| {
        json!({
            "salary_entries": cnis.salary_entries.len(),
            "salary_months": cnis.salary_months.len(),
            "links": cnis.links.len(),
            "prpps_links": cnis.links.iter().filter(|v| v.prpps).count(),
            "missing_end_dates": cnis.links.iter().filter(|v| v.end_date.is_none()).count(),
            "benefits": cnis.benefits.len(),
            "warnings": cnis.warnings.len(),
            "pages": cnis.metadata.page_count,
        })
    });
    json!({
        "schema": "previdenciar-next-summary/v1",
        "legal_validation_performed": false,
        "currency_policy": "currency_units_unverified",
        "cnis_counts": counts,
        "manual_link_count": calculation.manual_links.len(),
        "manual_salary_count": calculation.manual_salaries.len(),
        "warnings": [
            "Resumo técnico de importação, não concessão de benefício nem validação jurídica.",
            "Indicadores, moedas históricas e regimes exigem revisão independente.",
        ],
    })
}

async fn export_report_text(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ExportTextRequest>,
) -> Result<Json<Value>, ApiError> {
    let _user = auth::authenticate(&headers, &state)
        .await
        .map_err(ApiError::unauthorized)?;
    let out = state
        .blocking
        .run(move || -> Result<Value, ApiError> {
            match body.format.as_str() {
                "html" => {
                    let redacted;
                    let case = if body.include_personal {
                        &body.calculation
                    } else {
                        redacted = private_case(&body.calculation);
                        &redacted
                    };
                    let mut html = previdenciar_workspace::report_html(
                        case,
                        &body.workspace,
                        body.include_personal,
                    );
                    if body.include_personal {
                        let extra = recomputed_rgps_report(&body.calculation);
                        html = html.replace("</body>", &format!("{extra}</body>"));
                    }
                    Ok(json!({ "content": html, "mime": "text/html; charset=utf-8", "extension": "html" }))
                }
                "csv" => {
                    let csv = if body.include_personal {
                        previdenciar_workspace::export_salary_csv(&body.calculation)
                    } else {
                        let redacted = private_case(&body.calculation);
                        previdenciar_workspace::export_salary_csv(&redacted)
                    };
                    Ok(json!({ "content": csv, "mime": "text/csv; charset=utf-8", "extension": "csv" }))
                }
                "json" => {
                    let payload = if body.include_personal {
                        let outcome = previdenciar_rgps::calculate_case(&body.calculation)
                            .unwrap_or_else(|e| json!({ "status": "error", "message": e }));
                        json!({
                            "schema": "previdenciar-web-export/v1",
                            "calculation": {
                                "id": body.calculation.id,
                                "kind": body.calculation.kind,
                                "dib": body.calculation.dib,
                                "especie": body.calculation.especie,
                            },
                            "outcome": outcome,
                        })
                    } else {
                        summary_json(&body.calculation)
                    };
                    let pretty = serde_json::to_string_pretty(&payload).map_err(ApiError::internal)?;
                    Ok(json!({ "content": pretty, "mime": "application/json; charset=utf-8", "extension": "json" }))
                }
                _ => Err(ApiError::bad_request(
                    "Formato de exportação inválido (use html, csv ou json).".into(),
                )),
            }
        })
        .await
        .map_err(ApiError::internal)??;
    Ok(Json(out))
}

async fn ai_data_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let _user = auth::authenticate(&headers, &state)
        .await
        .map_err(ApiError::unauthorized)?;
    let root = state.config.ai_data_root.clone();
    let status = state
        .blocking
        .run(move || previdenciar_ai::DataStore::new(root.into()).status())
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(serde_json::to_value(status).map_err(ApiError::internal)?))
}

#[derive(Deserialize)]
struct KnowledgeSearchRequest {
    block: previdenciar_ai::KnowledgeBlock,
    query: String,
    limit: Option<u32>,
}

async fn ai_knowledge_search(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<KnowledgeSearchRequest>,
) -> Result<Json<Value>, ApiError> {
    let _user = auth::authenticate(&headers, &state)
        .await
        .map_err(ApiError::unauthorized)?;
    let root = state.config.ai_data_root.clone();
    let limit = body.limit.unwrap_or(8).clamp(1, 8) as usize;
    let outcome = state
        .blocking
        .run(move || {
            previdenciar_ai::DataStore::new(root.into())
                .search_block(body.block, &body.query, limit)
                .map_err(|e| ApiError::bad_request(e.to_string()))
        })
        .await
        .map_err(ApiError::internal)??;
    Ok(Json(serde_json::to_value(outcome).map_err(ApiError::internal)?))
}

#[derive(Deserialize)]
struct KnowledgeReadRequest {
    block: previdenciar_ai::KnowledgeBlock,
    id: String,
}

async fn ai_knowledge_read(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<KnowledgeReadRequest>,
) -> Result<Json<Value>, ApiError> {
    let _user = auth::authenticate(&headers, &state)
        .await
        .map_err(ApiError::unauthorized)?;
    let root = state.config.ai_data_root.clone();
    let outcome = state
        .blocking
        .run(move || {
            previdenciar_ai::DataStore::new(root.into())
                .read_block(body.block, &body.id)
                .map_err(|e| ApiError::bad_request(e.to_string()))
        })
        .await
        .map_err(ApiError::internal)??;
    Ok(Json(serde_json::to_value(outcome).map_err(ApiError::internal)?))
}

#[derive(Deserialize, Default)]
struct FabricaQuery {
    demonstrativo: Option<bool>,
    permalink: Option<bool>,
    fundamento: Option<bool>,
}

fn valid_fabrica_key(key: &str) -> bool {
    let trimmed = key.trim();
    (8..=512).contains(&trimmed.len()) && trimmed.chars().all(|c| c.is_ascii_graphic())
}

fn resolve_fabrica_key(headers: &HeaderMap, state: &AppState) -> Result<Option<String>, ApiError> {
    if let Some(raw) = headers.get("x-fabrica-key").and_then(|v| v.to_str().ok()) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            if !valid_fabrica_key(trimmed) {
                return Err(ApiError::bad_request(
                    "Chave da API da Fábrica TRF3 inválida (sem espaços).".into(),
                ));
            }
            return Ok(Some(trimmed.to_owned()));
        }
    }
    let server_key = state.config.fabrica_api_key.trim();
    if valid_fabrica_key(server_key) {
        Ok(Some(server_key.to_owned()))
    } else {
        Ok(None)
    }
}

async fn fabrica_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let _user = auth::authenticate(&headers, &state)
        .await
        .map_err(ApiError::unauthorized)?;
    let session_key = headers
        .get("x-fabrica-key")
        .and_then(|v| v.to_str().ok())
        .is_some_and(valid_fabrica_key);
    let server_key = state.config.fabrica_enabled();
    Ok(Json(json!({
        "configured": session_key || server_key,
        "serverKeyAvailable": server_key,
        "sessionKeyConfigured": session_key,
        "upstream": state.config.fabrica_base_url,
    })))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn fab_error_messages(bytes: &[u8]) -> String {
    let Ok(val) = serde_json::from_slice::<Value>(bytes) else {
        return String::new();
    };
    val.get("messages")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .take(3)
                .filter_map(Value::as_str)
                .map(|t| t.chars().take(200).collect::<String>())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

async fn post_fabrica_json(
    state: &AppState,
    url: &str,
    api_key: &str,
    body_bytes: &[u8],
) -> Result<Vec<u8>, ApiError> {
    let resp = state
        .http
        .post(url)
        .timeout(Duration::from_secs(120))
        .header("x-api-key", api_key)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ACCEPT, "application/json")
        .header(header::ACCEPT_LANGUAGE, "pt-BR,pt;q=0.9")
        .body(body_bytes.to_vec())
        .send()
        .await
        .map_err(|_| {
            ApiError::unavailable(
                "Não foi possível conectar à Fábrica de Cálculos do TRF3.".into(),
            )
        })?;
    let status = resp.status();
    let bytes = resp.bytes().await.map_err(ApiError::internal)?;
    if status == StatusCode::UNAUTHORIZED {
        return Err(ApiError::bad_request(
            "A Fábrica TRF3 recusou a chave (HTTP 401).".into(),
        ));
    }
    if status == StatusCode::FORBIDDEN {
        return Err(ApiError::bad_request(
            "A chave não possui permissão para este recurso na Fábrica TRF3 (HTTP 403).".into(),
        ));
    }
    if status == StatusCode::TOO_MANY_REQUESTS {
        return Err(ApiError::unavailable(
            "Limite temporário de uso da Fábrica TRF3; tente novamente em instantes.".into(),
        ));
    }
    if !status.is_success() {
        let detail = fab_error_messages(&bytes);
        let msg = if detail.is_empty() {
            format!("A Fábrica TRF3 retornou HTTP {}.", status.as_u16())
        } else {
            format!("A Fábrica TRF3 retornou HTTP {}: {detail}", status.as_u16())
        };
        return Err(ApiError::bad_request(msg));
    }
    if bytes.len() > 24 * 1024 * 1024 {
        return Err(ApiError::payload_too_large(
            "Resposta da Fábrica de Cálculos excede o limite de 24 MiB.".into(),
        ));
    }
    Ok(bytes.to_vec())
}

fn prepare_fabrica_payload(raw: Value, is_tempo: bool) -> Result<(Value, bool, bool, Option<String>), ApiError> {
    if raw.get("periodos").is_some() && raw.get("periodosContribuicao").is_none() {
        let input: previdenciar_rgps::fabrica::FabricaInput =
            serde_json::from_value(raw).map_err(|e| ApiError::bad_request(e.to_string()))?;
        let demo = input.demonstrativo;
        let perm = input.permalink;
        let fund = input.fundamento.clone();
        let body = if is_tempo {
            previdenciar_rgps::fabrica::build_tempo_body(&input)
        } else {
            previdenciar_rgps::fabrica::build_body(&input)
        }
        .map_err(ApiError::bad_request)?;
        Ok((body, demo, perm, fund))
    } else {
        Ok((raw, false, false, None))
    }
}

async fn fabrica_rmi(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<FabricaQuery>,
    Json(raw): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let user = auth::authenticate(&headers, &state)
        .await
        .map_err(ApiError::unauthorized)?;
    enforce(
        &state,
        vec![(format!("calc:{}", user.email.to_lowercase()), CALC_WINDOW)],
    )?;
    let Some(key) = resolve_fabrica_key(&headers, &state)? else {
        return Err(ApiError::unavailable(
            "Chave da Fábrica TRF3 não configurada (informe x-fabrica-key ou configure FABRICA_API_KEY no servidor).".into(),
        ));
    };
    let (body, inp_demo, inp_perm, fund) = prepare_fabrica_payload(raw, false)?;
    let demo = q.demonstrativo.unwrap_or(inp_demo);
    let perm = q.permalink.unwrap_or(inp_perm);
    let suffix = previdenciar_rgps::fabrica::query_suffix(demo, perm);
    let use_fund = q.fundamento.unwrap_or(false)
        && fund.as_deref().is_some_and(|s| !s.trim().is_empty());
    let path = if use_fund {
        "/rgps/api/v1/calculos-rmi/por-fundamento"
    } else {
        "/rgps/api/v1/calculos-rmi"
    };
    let url = format!("{}{}{}", state.config.fabrica_base_url, path, suffix);
    let body_bytes = serde_json::to_vec(&body).map_err(ApiError::internal)?;
    let bytes = post_fabrica_json(&state, &url, &key, &body_bytes).await?;
    let parsed = previdenciar_rgps::fabrica::parse_summary(&bytes).map_err(ApiError::bad_request)?;
    Ok(Json(json!({
        "provider": parsed,
        "source_url": url,
        "retrieved_at": chrono::Utc::now().to_rfc3339(),
        "request_sha256": sha256_hex(&body_bytes),
        "response_sha256": sha256_hex(&bytes),
        "bytes": bytes.len(),
        "api": "Fábrica de Cálculos/TRF3",
        "version": 1,
    })))
}

async fn fabrica_tempo(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<FabricaQuery>,
    Json(raw): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    fabrica_tempo_inner(state, headers, q, raw, "/rgps/api/v1/calculos-tempo").await
}

async fn fabrica_tempo_grupos(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<FabricaQuery>,
    Json(raw): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    fabrica_tempo_inner(state, headers, q, raw, "/rgps/api/v1/calculos-tempo/por-grupos").await
}

async fn fabrica_tempo_inner(
    state: AppState,
    headers: HeaderMap,
    q: FabricaQuery,
    raw: Value,
    endpoint_path: &str,
) -> Result<Json<Value>, ApiError> {
    let user = auth::authenticate(&headers, &state)
        .await
        .map_err(ApiError::unauthorized)?;
    enforce(
        &state,
        vec![(format!("calc:{}", user.email.to_lowercase()), CALC_WINDOW)],
    )?;
    let Some(key) = resolve_fabrica_key(&headers, &state)? else {
        return Err(ApiError::unavailable(
            "Chave da Fábrica TRF3 não configurada.".into(),
        ));
    };
    let (body, inp_demo, inp_perm, _) = prepare_fabrica_payload(raw, true)?;
    let suffix = previdenciar_rgps::fabrica::query_suffix(
        q.demonstrativo.unwrap_or(inp_demo),
        q.permalink.unwrap_or(inp_perm),
    );
    let url = format!("{}{}{}", state.config.fabrica_base_url, endpoint_path, suffix);
    let body_bytes = serde_json::to_vec(&body).map_err(ApiError::internal)?;
    let bytes = post_fabrica_json(&state, &url, &key, &body_bytes).await?;
    let parsed = previdenciar_rgps::fabrica::parse_tempo_summary(&bytes).map_err(ApiError::bad_request)?;
    Ok(Json(parsed))
}

async fn fabrica_atrasados(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<FabricaQuery>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let user = auth::authenticate(&headers, &state)
        .await
        .map_err(ApiError::unauthorized)?;
    enforce(
        &state,
        vec![(format!("calc:{}", user.email.to_lowercase()), CALC_WINDOW)],
    )?;
    let Some(key) = resolve_fabrica_key(&headers, &state)? else {
        return Err(ApiError::unavailable(
            "Chave da Fábrica TRF3 não configurada.".into(),
        ));
    };
    let suffix = previdenciar_rgps::fabrica::query_suffix(
        q.demonstrativo.unwrap_or(false),
        q.permalink.unwrap_or(false),
    );
    let url = format!("{}/rgps/api/v1/calculos-atrasados{}", state.config.fabrica_base_url, suffix);
    let body_bytes = serde_json::to_vec(&body).map_err(ApiError::internal)?;
    let bytes = post_fabrica_json(&state, &url, &key, &body_bytes).await?;
    let parsed = previdenciar_rgps::fabrica::parse_atrasados_summary(&bytes).map_err(ApiError::bad_request)?;
    Ok(Json(parsed))
}

async fn fabrica_documento(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(tipo): Path<String>,
    mut multipart: Multipart,
) -> Result<Json<Value>, ApiError> {
    let user = auth::authenticate(&headers, &state)
        .await
        .map_err(ApiError::unauthorized)?;
    enforce(
        &state,
        vec![(format!("ocr:{}", user.email.to_lowercase()), OCR_WINDOW)],
    )?;
    if !previdenciar_rgps::fabrica::DOCUMENT_TYPES.contains(&tipo.as_str()) {
        return Err(ApiError::bad_request(format!(
            "Tipo documental inválido: {tipo}. Aceitos: cnis, rdctc, carta, pap, prevjud."
        )));
    }
    let Some(key) = resolve_fabrica_key(&headers, &state)? else {
        return Err(ApiError::unavailable(
            "Chave da Fábrica TRF3 não configurada.".into(),
        ));
    };
    while let Some(field) = multipart.next_field().await.map_err(multipart_error)? {
        if field.name() != Some("file") {
            continue;
        }
        let file_name = field.file_name().unwrap_or("documento.pdf").to_owned();
        let bytes = field.bytes().await.map_err(multipart_error)?;
        if bytes.is_empty() || bytes.len() > MAX_CNIS_PDF_BYTES {
            return Err(ApiError::bad_request(
                "Arquivo vazio ou maior que 32 MiB.".into(),
            ));
        }
        let boundary = format!("----PrevidenciarBoundary{}", Uuid::new_v4().simple());
        let mut payload = Vec::with_capacity(bytes.len() + 256);
        let safe_name: String = file_name
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
            .take(64)
            .collect();
        let clean_name = if safe_name.is_empty() { "documento.pdf".to_string() } else { safe_name };
        payload.extend_from_slice(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{clean_name}\"\r\nContent-Type: application/pdf\r\n\r\n").as_bytes());
        payload.extend_from_slice(&bytes);
        payload.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

        let url = format!(
            "{}/rgps/api/v1/documentos/{tipo}/ler",
            state.config.fabrica_base_url
        );
        let resp = state
            .http
            .post(&url)
            .timeout(Duration::from_secs(180))
            .header("x-api-key", key)
            .header(header::ACCEPT, "application/json")
            .header(header::ACCEPT_LANGUAGE, "pt-BR,pt;q=0.9")
            .header(
                header::CONTENT_TYPE,
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(payload)
            .send()
            .await
            .map_err(|_| ApiError::unavailable("Falha ao contatar leitor documental da Fábrica TRF3.".into()))?;
        let status = resp.status();
        let resp_bytes = resp.bytes().await.map_err(ApiError::internal)?;
        if status == StatusCode::UNAUTHORIZED {
            return Err(ApiError::bad_request("A Fábrica TRF3 recusou a chave (HTTP 401).".into()));
        }
        if status == StatusCode::FORBIDDEN {
            return Err(ApiError::bad_request("A chave não possui permissão para o leitor documental da Fábrica TRF3 (HTTP 403).".into()));
        }
        if status == StatusCode::TOO_MANY_REQUESTS {
            return Err(ApiError::unavailable("Limite temporário de uso da Fábrica TRF3; tente novamente em instantes.".into()));
        }
        if !status.is_success() {
            let detail = fab_error_messages(&resp_bytes);
            return Err(ApiError::bad_request(if detail.is_empty() {
                format!("Leitor documental TRF3 retornou HTTP {}.", status.as_u16())
            } else {
                format!("Leitor documental TRF3 retornou HTTP {}: {detail}", status.as_u16())
            }));
        }
        if resp_bytes.len() > 24 * 1024 * 1024 {
            return Err(ApiError::payload_too_large("Resposta da Fábrica de Cálculos excede o limite de 24 MiB.".into()));
        }
        let parsed = previdenciar_rgps::fabrica::parse_documento_response(&resp_bytes)
            .map_err(ApiError::bad_request)?;
        return Ok(Json(parsed));
    }
    Err(ApiError::bad_request("Envie o arquivo no campo file.".into()))
}

async fn fabrica_get_proxy(
    state: &AppState,
    headers: &HeaderMap,
    upstream_path: &str,
) -> Result<Json<Value>, ApiError> {
    let _user = auth::authenticate(headers, state)
        .await
        .map_err(ApiError::unauthorized)?;
    let url = format!("{}{}", state.config.fabrica_base_url, upstream_path);
    let mut req = state
        .http
        .get(&url)
        .timeout(Duration::from_secs(30))
        .header(header::ACCEPT, "application/json")
        .header(header::ACCEPT_LANGUAGE, "pt-BR,pt;q=0.9");
    if let Some(key) = resolve_fabrica_key(headers, state)? {
        req = req.header("x-api-key", key);
    }
    let resp = req
        .send()
        .await
        .map_err(|_| ApiError::unavailable("Fábrica TRF3 indisponível no momento.".into()))?;
    let status = resp.status();
    let bytes = resp.bytes().await.map_err(ApiError::internal)?;
    if status == StatusCode::UNAUTHORIZED {
        return Err(ApiError::bad_request("A Fábrica TRF3 recusou a chave (HTTP 401).".into()));
    }
    if status == StatusCode::FORBIDDEN {
        return Err(ApiError::bad_request("A chave não possui permissão para este recurso na Fábrica TRF3 (HTTP 403).".into()));
    }
    if status == StatusCode::TOO_MANY_REQUESTS {
        return Err(ApiError::unavailable("Limite temporário de uso da Fábrica TRF3; tente novamente em instantes.".into()));
    }
    if !status.is_success() {
        let code = status.as_u16();
        let detail = fab_error_messages(&bytes);
        return Err(ApiError::bad_request(if detail.is_empty() {
            format!("Fábrica TRF3 retornou HTTP {code}.")
        } else {
            format!("Fábrica TRF3 retornou HTTP {code}: {detail}")
        }));
    }
    if bytes.len() > 24 * 1024 * 1024 {
        return Err(ApiError::payload_too_large("Resposta da Fábrica de Cálculos excede o limite de 24 MiB.".into()));
    }
    let val: Value = serde_json::from_slice(&bytes)
        .map_err(|_| ApiError::bad_request("Resposta inválida ao consultar tabela da Fábrica TRF3.".into()))?;
    Ok(Json(val))
}

async fn fabrica_indice(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(nome): Path<String>,
    Query(params): Query<std::collections::BTreeMap<String, String>>,
) -> Result<Json<Value>, ApiError> {
    if !previdenciar_rgps::fabrica::INDEX_TYPES.contains(&nome.as_str()) {
        return Err(ApiError::bad_request("Índice não suportado.".into()));
    }
    let query_pairs: Vec<String> = params
        .into_iter()
        .filter_map(|(k, v)| {
            let trimmed = v.trim();
            if matches!(k.as_str(), "competenciaInicial" | "competenciaFinal" | "indice")
                && !trimmed.is_empty()
                && trimmed.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
            {
                Some(format!("{k}={trimmed}"))
            } else {
                None
            }
        })
        .collect();
    let suffix = if query_pairs.is_empty() {
        String::new()
    } else {
        format!("?{}", query_pairs.join("&"))
    };
    fabrica_get_proxy(&state, &headers, &format!("/indices/api/v1/{nome}{suffix}")).await
}

async fn fabrica_fundamentos(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    fabrica_get_proxy(&state, &headers, "/rgps/api/v1/fundamentos").await
}

async fn fabrica_marcos_temporais(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    fabrica_get_proxy(&state, &headers, "/rgps/api/v1/marcos-temporais").await
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

// ---------------------------------------------------------------------------
// Troca de clientes e casos com o Atrium.
//
// O navegador nunca fala com o Atrium: estas rotas assinam um token HS256 de curta
// duração com o segredo da integração, com o `sub` da identidade Atrium ligada à conta,
// e chamam `/api/integrations/previdenciar/*` no servidor do Atrium. O Atrium decide o
// escritório pelo `sub` assinado e só devolve ou altera o que esse usuário já vê lá.
// ---------------------------------------------------------------------------

const ATRIUM_SYNC_WINDOW: RateWindow = RateWindow::new(
    600,
    Duration::from_secs(60 * 60),
    "Limite de trocas com o Atrium atingido. Aguarde uma hora e tente de novo.",
);

struct AtriumLink {
    owner: Uuid,
    subject: String,
    workspace_id: String,
    username: String,
    email: String,
}

fn parse_atrium_subject(subject: &str) -> Option<(String, String)> {
    let rest = subject.strip_prefix("atrium:")?;
    let (workspace, username) = rest.split_once(':')?;
    if workspace.is_empty() || username.is_empty() {
        return None;
    }
    Some((workspace.to_owned(), username.to_owned()))
}

async fn atrium_link(
    state: &AppState,
    ip: &ClientIp,
    headers: &HeaderMap,
) -> Result<AtriumLink, ApiError> {
    let user = auth::authenticate(headers, state)
        .await
        .map_err(ApiError::unauthorized)?;
    let owner = ensure_user_guarded(state, ip, &user).await?;
    if !state.config.atrium_sso_enabled() {
        return Err(ApiError::unavailable(
            "A troca de dados com o Atrium ainda não foi habilitada neste servidor.".into(),
        ));
    }
    enforce(
        state,
        vec![(format!("atrium-sync:{owner}"), ATRIUM_SYNC_WINDOW)],
    )?;
    let subject = if user.subject.starts_with("atrium:") {
        user.subject.clone()
    } else {
        sqlx::query_scalar::<_, String>(
            "SELECT cognito_sub FROM users WHERE id = $1 AND cognito_sub LIKE 'atrium:%' \
             UNION ALL \
             SELECT subject FROM user_identities WHERE user_id = $1 AND subject LIKE 'atrium:%' \
             LIMIT 1",
        )
        .bind(owner)
        .fetch_optional(&state.db)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| {
            ApiError::forbidden(
                "Para trocar dados com o Atrium, entre uma vez com a sua conta do Atrium.".into(),
            )
        })?
    };
    let (workspace_id, username) = parse_atrium_subject(&subject).ok_or_else(|| {
        ApiError::forbidden("A sessão do Atrium não identifica o escritório.".into())
    })?;
    Ok(AtriumLink {
        owner,
        subject,
        workspace_id,
        username,
        email: user.email,
    })
}

async fn atrium_call(
    state: &AppState,
    link: &AtriumLink,
    method: reqwest::Method,
    path: &str,
    query: &[(&str, String)],
    body: Option<Value>,
) -> Result<(StatusCode, Value), ApiError> {
    let token = auth::mint_atrium_hs256(
        &state.config.atrium_sso_secret,
        link.subject.clone(),
        (!link.email.is_empty()).then(|| link.email.clone()),
        Some(link.username.clone()),
        true,
        None,
    )
    .map_err(ApiError::internal)?;
    let base = format!(
        "{}/api/integrations/previdenciar/{path}",
        state.config.atrium_auth_base_url.trim_end_matches('/')
    );
    let url = reqwest::Url::parse_with_params(&base, query).map_err(ApiError::internal)?;
    let mut request = state
        .http
        .request(method, url)
        .timeout(Duration::from_secs(20))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Atrium-Workspace", &link.workspace_id)
        .header("X-Atrium-User", &link.username);
    if let Some(body) = body {
        request = request.json(&body);
    }
    let response = request.send().await.map_err(|_| {
        ApiError::unavailable(
            "Não foi possível conectar ao Atrium agora. Tente de novo em instantes.".into(),
        )
    })?;
    let status = response.status();
    let text = response.text().await.map_err(ApiError::internal)?;
    let payload: Option<Value> = serde_json::from_str(&text).ok();
    if status.is_success() {
        return Ok((
            StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK),
            payload.unwrap_or_else(|| json!({})),
        ));
    }
    let upstream_message = payload
        .as_ref()
        .and_then(|value| value.get("message"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    let (code, message) = match status.as_u16() {
        401 => (
            StatusCode::BAD_GATEWAY,
            "O Atrium recusou a credencial da integração. Os segredos dos dois servidores não conferem; avise o suporte.".to_owned(),
        ),
        404 if upstream_message.is_none() => (
            StatusCode::SERVICE_UNAVAILABLE,
            "O Atrium ainda não publicou essa função da integração.".to_owned(),
        ),
        code @ (400 | 402 | 403 | 404 | 409) => (
            StatusCode::from_u16(code).unwrap_or(StatusCode::BAD_GATEWAY),
            upstream_message.unwrap_or_else(|| "O Atrium recusou o pedido.".to_owned()),
        ),
        _ => (
            StatusCode::SERVICE_UNAVAILABLE,
            upstream_message.unwrap_or_else(|| {
                "O Atrium não respondeu como esperado. Tente de novo em instantes.".to_owned()
            }),
        ),
    };
    Err(ApiError {
        status: code,
        message,
        retry_after: None,
    })
}

fn clip_text(value: Option<&Value>, max: usize) -> String {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("")
        .chars()
        .take(max)
        .collect()
}

#[derive(Deserialize, Default)]
struct AtriumListQuery {
    q: Option<String>,
    contact_ids: Option<String>,
}

fn atrium_list_params(query: &AtriumListQuery) -> Vec<(&'static str, String)> {
    let mut params = Vec::new();
    if let Some(q) = query.q.as_deref().map(str::trim).filter(|q| !q.is_empty()) {
        params.push(("q", q.chars().take(120).collect()));
    }
    if let Some(ids) = query
        .contact_ids
        .as_deref()
        .map(str::trim)
        .filter(|ids| !ids.is_empty())
    {
        params.push(("contact_ids", ids.chars().take(8000).collect()));
    }
    params
}

async fn atrium_list_contacts(
    State(state): State<AppState>,
    ip: ClientIp,
    headers: HeaderMap,
    Query(query): Query<AtriumListQuery>,
) -> Result<Json<Value>, ApiError> {
    let link = atrium_link(&state, &ip, &headers).await?;
    let (_, payload) = atrium_call(
        &state,
        &link,
        reqwest::Method::GET,
        "contacts",
        &atrium_list_params(&query),
        None,
    )
    .await?;
    Ok(Json(json!({
        "contacts": payload.get("contacts").cloned().unwrap_or_else(|| json!([]))
    })))
}

async fn atrium_list_processes(
    State(state): State<AppState>,
    ip: ClientIp,
    headers: HeaderMap,
    Query(query): Query<AtriumListQuery>,
) -> Result<Json<Value>, ApiError> {
    let link = atrium_link(&state, &ip, &headers).await?;
    let (_, payload) = atrium_call(
        &state,
        &link,
        reqwest::Method::GET,
        "processes",
        &atrium_list_params(&query),
        None,
    )
    .await?;
    Ok(Json(json!({
        "processes": payload.get("processes").cloned().unwrap_or_else(|| json!([]))
    })))
}

fn atrium_write_result(payload: &Value) -> Result<Json<Value>, ApiError> {
    let id = payload
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| {
            ApiError::unavailable("O Atrium não confirmou a gravação. Tente de novo em instantes.".into())
        })?;
    Ok(Json(json!({
        "id": id,
        "created": payload.get("created").and_then(Value::as_bool).unwrap_or(false)
    })))
}

async fn atrium_send_contact(
    State(state): State<AppState>,
    ip: ClientIp,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let link = atrium_link(&state, &ip, &headers).await?;
    let name = clip_text(body.get("name"), 200);
    if name.is_empty() {
        return Err(ApiError::bad_request("Informe o nome do cliente.".into()));
    }
    // Só os campos do cadastro seguem para o Atrium; nada do cálculo ou do CNIS.
    let outgoing = json!({
        "name": name,
        "cpf": clip_text(body.get("cpf"), 20),
        "email": clip_text(body.get("email"), 200),
        "phone": clip_text(body.get("phone"), 40),
        "birth_date": clip_text(body.get("birth_date"), 20),
        "sex": clip_text(body.get("sex"), 20),
        "notes": clip_text(body.get("notes"), 4000),
    });
    let (_, payload) = atrium_call(
        &state,
        &link,
        reqwest::Method::POST,
        "contacts",
        &[],
        Some(outgoing),
    )
    .await?;
    atrium_write_result(&payload)
}

async fn atrium_send_process(
    State(state): State<AppState>,
    ip: ClientIp,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    let link = atrium_link(&state, &ip, &headers).await?;
    let contact_id = clip_text(body.get("contact_id"), 120);
    let title = clip_text(body.get("title"), 200);
    let calculation_id = body.get("calculation_id").and_then(Value::as_u64);
    let Some(calculation_id) = calculation_id.filter(|_| !contact_id.is_empty() && !title.is_empty()) else {
        return Err(ApiError::bad_request(
            "Informe o caso, o nome do caso e o contato do Atrium.".into(),
        ));
    };
    let outgoing = json!({
        "contact_id": contact_id,
        "prev_ref": format!("{}:{calculation_id}", link.owner),
        "atrium_id": clip_text(body.get("atrium_id"), 120),
        "title": title,
        "number": clip_text(body.get("number"), 60),
        "nb": clip_text(body.get("nb"), 40),
        "notes": clip_text(body.get("notes"), 4000),
    });
    let (_, payload) = atrium_call(
        &state,
        &link,
        reqwest::Method::POST,
        "processes",
        &[],
        Some(outgoing),
    )
    .await?;
    atrium_write_result(&payload)
}

async fn ensure_user_guarded(
    state: &AppState,
    ip: &ClientIp,
    user: &auth::AuthUser,
) -> Result<Uuid, ApiError> {
    let known = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM users WHERE cognito_sub = $1
         UNION ALL SELECT user_id FROM user_identities WHERE subject = $1
         LIMIT 1",
    )
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

    #[tokio::test]
    async fn fabrica_status_export_and_snapshot_endpoints_work_without_external_db() {
        let base = spawn_server(test_state(config::Config::for_tests())).await;
        let client = reqwest::Client::new();

        let s1 = client
            .get(format!("{base}/api/fabrica/status"))
            .header("x-dev-user", "adv@teste.local")
            .send()
            .await
            .expect("fabrica status");
        assert_eq!(s1.status(), 200);
        let j1: Value = s1.json().await.unwrap();
        assert_eq!(j1["configured"], false);
        assert_eq!(j1["sessionKeyConfigured"], false);

        let s2 = client
            .get(format!("{base}/api/fabrica/status"))
            .header("x-dev-user", "adv@teste.local")
            .header("x-fabrica-key", "chave-teste-trf3-12345")
            .send()
            .await
            .expect("fabrica status custom key");
        assert_eq!(s2.status(), 200);
        let j2: Value = s2.json().await.unwrap();
        assert_eq!(j2["configured"], true);
        assert_eq!(j2["sessionKeyConfigured"], true);

        let snap = client
            .get(format!("{base}/api/pcd/snapshot?through=2026-09"))
            .header("x-dev-user", "adv@teste.local")
            .send()
            .await
            .expect("pcd snapshot");
        assert_eq!(snap.status(), 200);
        let sj: Value = snap.json().await.unwrap();
        assert_eq!(sj["through"], "2026-09");
        assert!(!sj["rates"].as_array().unwrap().is_empty());

        let ai = client
            .get(format!("{base}/api/ai/data-status"))
            .header("x-dev-user", "adv@teste.local")
            .send()
            .await
            .expect("ai data status");
        assert_eq!(ai.status(), 200);

        let calc = previdenciar_workspace::Calculation::default();
        let ws = previdenciar_workspace::Workspace::default();
        for fmt in ["html", "csv", "json"] {
            let exp = client
                .post(format!("{base}/api/reports/export"))
                .header("x-dev-user", "adv@teste.local")
                .json(&json!({
                    "calculation": calc,
                    "workspace": ws,
                    "format": fmt,
                    "include_personal": false
                }))
                .send()
                .await
                .expect("export response");
            assert_eq!(exp.status(), 200, "format {fmt}");
            let ej: Value = exp.json().await.unwrap();
            assert_eq!(ej["extension"], fmt);
            assert!(!ej["content"].as_str().unwrap().is_empty());
        }
    }

    #[tokio::test]
    async fn fabrica_proxy_routes_validate_keys_and_forward_without_echoing_secret() {
        let upstream_router = Router::new()
            .route(
                "/rgps/api/v1/calculos-rmi",
                post(|headers: HeaderMap, Query(q): Query<std::collections::BTreeMap<String, String>>, Json(body): Json<Value>| async move {
                    let api_key = headers.get("x-api-key").and_then(|v| v.to_str().ok()).unwrap_or_default();
                    if api_key != "chave-trf3-valida-123" {
                        return (StatusCode::UNAUTHORIZED, Json(json!({"messages": ["unauthorized"]}))).into_response();
                    }
                    assert!(body.get("periodosContribuicao").is_some());
                    assert!(body.get("dadosSegurado").is_some());
                    assert_eq!(q.get("demonstrativo").map(String::as_str), Some("true"));
                    assert_eq!(q.get("permalink").map(String::as_str), Some("true"));
                    Json(json!({
                        "status": "ok",
                        "permalink": "https://exemplo.invalid/perm/1",
                        "demonstrativoPdf": "JVBERi0xLjQ=",
                        "result": [{
                            "maiorRmi": "1700.00",
                            "moeda": { "simbolo": "R$", "nome": "Real" },
                            "beneficiosMaisVantajosos": [{
                                "especie": 41,
                                "descricao": "Aposentadoria por Idade",
                                "chaveFundamento": "EC_103_ART_18",
                                "fundamento": "EC 103, art. 18",
                                "dadosRmi": { "rmi": "1700.00", "salarioBeneficio": "2000.00", "coeficiente": 0.7, "divisor": 300 },
                                "dadosApuracao": { "temDireito": true, "naApuracao": {} }
                            }],
                            "conclusao": { "completo": true },
                            "linhasDescartes": []
                        }]
                    })).into_response()
                }),
            )
            .route(
                "/rgps/api/v1/calculos-tempo/por-grupos",
                post(|| async {
                    Json(json!({ "status": "ok", "result": { "tempoTotalDias": 9125 }, "messages": [] }))
                }),
            )
            .route(
                "/rgps/api/v1/calculos-atrasados",
                post(|| async {
                    Json(json!({ "status": "ok", "result": { "totalAtualizado": "15000.00" }, "messages": [] }))
                }),
            )
            .route(
                "/rgps/api/v1/documentos/cnis/ler",
                post(|headers: HeaderMap| async move {
                    let api_key = headers.get("x-api-key").and_then(|v| v.to_str().ok()).unwrap_or_default();
                    if api_key == "chave-recusada-401" {
                        return (StatusCode::UNAUTHORIZED, Json(json!({ "messages": ["denied"] }))).into_response();
                    }
                    Json(json!({ "status": "ok", "result": { "vinculos": 1 }, "messages": [] })).into_response()
                }),
            )
            .route(
                "/rgps/api/v1/fundamentos",
                get(|| async { Json(json!([{ "chave": "EC_103_ART_18" }])) }),
            );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let upstream_addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let _ = axum::serve(listener, upstream_router).await;
        });

        let mut cfg = config::Config::for_tests();
        cfg.fabrica_base_url = format!("http://{upstream_addr}");
        let base = spawn_server(test_state(cfg)).await;
        let client = reqwest::Client::new();

        let payload = json!({
            "dataApuracao": "2026-01-10",
            "dataNascimento": "1956-01-10",
            "sexo": "M",
            "especie": 41,
            "isencaoCarencia": false,
            "tipo": "COMUM",
            "periodos": [{
                "dataInicial": "1994-07-01",
                "dataFinal": "2019-06-30",
                "salarios": [{ "competencia": "1994-07", "valor": "2000.00" }]
            }]
        });

        // Sem chave -> 503
        let no_key = client
            .post(format!("{base}/api/fabrica/rmi"))
            .header("x-dev-user", "adv@teste.local")
            .json(&payload)
            .send()
            .await
            .unwrap();
        assert_eq!(no_key.status(), 503);

        // Chave com espaço -> 400 e não ecoa o valor
        let bad_key = client
            .post(format!("{base}/api/fabrica/rmi"))
            .header("x-dev-user", "adv@teste.local")
            .header("x-fabrica-key", "SEGREDO COM ESPACO")
            .json(&payload)
            .send()
            .await
            .unwrap();
        assert_eq!(bad_key.status(), 400);
        let bad_txt = bad_key.text().await.unwrap();
        assert!(!bad_txt.contains("SEGREDO"));

        // RMI válido com ?demonstrativo=true&permalink=true
        let ok_rmi = client
            .post(format!("{base}/api/fabrica/rmi?demonstrativo=true&permalink=true"))
            .header("x-dev-user", "adv@teste.local")
            .header("x-fabrica-key", "chave-trf3-valida-123")
            .json(&payload)
            .send()
            .await
            .unwrap();
        assert_eq!(ok_rmi.status(), 200);
        let rmi_json: Value = ok_rmi.json().await.unwrap();
        assert_eq!(rmi_json["provider"]["maiorRmi"], "1700.00");
        assert_eq!(rmi_json["provider"]["demonstrativoPdf"], "JVBERi0xLjQ=");
        assert_eq!(rmi_json["provider"]["permalink"], "https://exemplo.invalid/perm/1");
        assert!(!rmi_json.to_string().contains("chave-trf3-valida-123"));

        // Tempo por grupos
        let ok_tempo = client
            .post(format!("{base}/api/fabrica/tempo/por-grupos"))
            .header("x-dev-user", "adv@teste.local")
            .header("x-fabrica-key", "chave-trf3-valida-123")
            .json(&payload)
            .send()
            .await
            .unwrap();
        assert_eq!(ok_tempo.status(), 200);
        let tempo_json: Value = ok_tempo.json().await.unwrap();
        assert_eq!(tempo_json["result"]["tempoTotalDias"], 9125);

        // Atrasados
        let ok_atr = client
            .post(format!("{base}/api/fabrica/atrasados"))
            .header("x-dev-user", "adv@teste.local")
            .header("x-fabrica-key", "chave-trf3-valida-123")
            .json(&json!({ "valor": "1000.00" }))
            .send()
            .await
            .unwrap();
        assert_eq!(ok_atr.status(), 200);
        let atr_json: Value = ok_atr.json().await.unwrap();
        assert_eq!(atr_json["result"]["totalAtualizado"], "15000.00");

        // Documento válido vs 401 upstream vs tipo inválido
        let ok_doc = client
            .post(format!("{base}/api/fabrica/documentos/cnis"))
            .header("x-dev-user", "adv@teste.local")
            .header("x-fabrica-key", "chave-trf3-valida-123")
            .header(header::CONTENT_TYPE, "multipart/form-data; boundary=fab-test")
            .body(multipart_body("fab-test", b"%PDF-1.4\nsynthetic"))
            .send()
            .await
            .unwrap();
        assert_eq!(ok_doc.status(), 200);

        let doc_401 = client
            .post(format!("{base}/api/fabrica/documentos/cnis"))
            .header("x-dev-user", "adv@teste.local")
            .header("x-fabrica-key", "chave-recusada-401")
            .header(header::CONTENT_TYPE, "multipart/form-data; boundary=fab-test")
            .body(multipart_body("fab-test", b"%PDF-1.4\nsynthetic"))
            .send()
            .await
            .unwrap();
        assert_eq!(doc_401.status(), 400);
        let doc_401_txt = doc_401.text().await.unwrap();
        assert!(doc_401_txt.contains("401"));
        assert!(!doc_401_txt.contains("chave-recusada-401"));

        let doc_bad = client
            .post(format!("{base}/api/fabrica/documentos/desconhecido"))
            .header("x-dev-user", "adv@teste.local")
            .header("x-fabrica-key", "chave-trf3-valida-123")
            .header(header::CONTENT_TYPE, "multipart/form-data; boundary=fab-test")
            .body(multipart_body("fab-test", b"%PDF-1.4"))
            .send()
            .await
            .unwrap();
        assert_eq!(doc_bad.status(), 400);

        // Fundamentos (lista no topo) e índice inválido
        let ok_fund = client
            .get(format!("{base}/api/fabrica/fundamentos"))
            .header("x-dev-user", "adv@teste.local")
            .send()
            .await
            .unwrap();
        assert_eq!(ok_fund.status(), 200);
        let fund_json: Value = ok_fund.json().await.unwrap();
        assert_eq!(fund_json, json!([{ "chave": "EC_103_ART_18" }]));

        let bad_idx = client
            .get(format!("{base}/api/fabrica/indices/inexistente"))
            .header("x-dev-user", "adv@teste.local")
            .send()
            .await
            .unwrap();
        assert_eq!(bad_idx.status(), 400);
    }

    #[tokio::test]
    async fn report_preview_and_text_export_enforce_privacy_fallback_and_recomputation() {
        let base = spawn_server(test_state(config::Config::for_tests())).await;
        let client = reqwest::Client::new();

        let mut calc: previdenciar_workspace::Calculation = serde_json::from_str(include_str!(
            "../../../crates/previdenciar-rgps/tests/fixtures/calculation-synthetic.json"
        ))
        .unwrap();
        calc.identity.full_name = Some("SEGURADO CONFIDENCIAL HTML".into());
        calc.identity.cpf = Some("111.222.333-44".into());
        calc.notes = "NOTA CONFIDENCIAL HTML".into();
        calc.options.report_title = "TITULO CONFIDENCIAL OPTS".into();
        for link in &mut calc.manual_links {
            link.employer = "EMPREGADOR CONFIDENCIAL HTML".into();
            link.indicators = "IND_LINK_SECRETO".into();
        }
        for sal in &mut calc.manual_salaries {
            sal.indicators = "IND_SAL_SECRETO".into();
        }
        calc.result = Some(json!({ "rmi": "999999.99" }));

        let mut ws = previdenciar_workspace::Workspace::default();
        ws.settings.office_name = "ESCRITORIO CONFIDENCIAL".into();
        ws.settings.report_footer = "RODAPE CONFIDENCIAL".into();

        // Modo privado (include_personal = false): redação total de campos arbitrários/identificação em HTML, CSV e JSON
        for fmt in ["html", "csv", "json"] {
            let resp = client
                .post(format!("{base}/api/reports/export"))
                .header("x-dev-user", "adv@teste.local")
                .json(&json!({
                    "calculation": calc,
                    "workspace": ws,
                    "format": fmt,
                    "include_personal": false
                }))
                .send()
                .await
                .unwrap();
            assert_eq!(resp.status(), 200, "private export {fmt}");
            let body: Value = resp.json().await.unwrap();
            let content = body["content"].as_str().unwrap();
            for secret in [
                "SEGURADO CONFIDENCIAL HTML",
                "111.222.333-44",
                "NOTA CONFIDENCIAL HTML",
                "TITULO CONFIDENCIAL OPTS",
                "EMPREGADOR CONFIDENCIAL HTML",
                "IND_LINK_SECRETO",
                "IND_SAL_SECRETO",
                "ESCRITORIO CONFIDENCIAL",
                "RODAPE CONFIDENCIAL",
                "999999.99",
            ] {
                assert!(!content.contains(secret), "{fmt} privado vazou {secret}");
            }
        }

        // Modo identificado (include_personal = true): HTML injeta simulação recalculada e JSON entrega outcome recalculado
        let json_pub = client
            .post(format!("{base}/api/reports/export"))
            .header("x-dev-user", "adv@teste.local")
            .json(&json!({
                "calculation": calc,
                "workspace": ws,
                "format": "json",
                "include_personal": true
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(json_pub.status(), 200);
        let pub_json_body: Value = json_pub.json().await.unwrap();
        let pub_json_str = pub_json_body["content"].as_str().unwrap();
        assert!(pub_json_str.contains("1600.00"), "{pub_json_str}");
        assert!(!pub_json_str.contains("999999.99"), "{pub_json_str}");

        let html_pub = client
            .post(format!("{base}/api/reports/export"))
            .header("x-dev-user", "adv@teste.local")
            .json(&json!({
                "calculation": calc,
                "workspace": ws,
                "format": "html",
                "include_personal": true
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(html_pub.status(), 200);
        let pub_json: Value = html_pub.json().await.unwrap();
        let pub_html = pub_json["content"].as_str().unwrap();
        assert!(pub_html.contains("Simulação técnica recalculada em Rust"), "{pub_html}");
        assert!(pub_html.contains("1600.00"), "{pub_html}");
        assert!(!pub_html.contains("999999.99"), "{pub_html}");

        // Preview PDF em Calculation::default() gera PDF de conferência (%PDF-) em vez de HTTP 400
        let empty_calc = previdenciar_workspace::Calculation::default();
        for fmt in ["pdf_summary", "pdf_regras", "pdf_detailed"] {
            let preview = client
                .post(format!("{base}/api/reports/preview"))
                .header("x-dev-user", "adv@teste.local")
                .json(&json!({
                    "calculation": empty_calc,
                    "workspace": ws,
                    "format": fmt,
                    "include_personal": false
                }))
                .send()
                .await
                .unwrap();
            assert_eq!(preview.status(), 200, "preview conference {fmt}");
            let pdf_bytes = preview.bytes().await.unwrap();
            assert!(pdf_bytes.starts_with(b"%PDF-"), "expected PDF header for {fmt}");
        }
    }
}
