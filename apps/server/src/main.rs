mod auth;
mod blocking;
mod config;
mod db;
mod limits;
mod routes;
mod storage;

#[cfg(test)]
mod integration_db;

use axum::extract::Request;
use axum::http::{header, HeaderName, HeaderValue, Method};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::Router;
use config::Config;
use jsonwebtoken::jwk::JwkSet;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: sqlx::PgPool,
    pub http: reqwest::Client,
    pub jwks: Arc<RwLock<Option<(Instant, JwkSet)>>>,
    pub storage: Option<Arc<storage::S3Store>>,
    pub limits: Arc<limits::RateLimiter>,
    pub blocking: Arc<blocking::BlockingGate>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let _ = dotenvy::from_path(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".env"));
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,previdenciar_web_server=debug".into()),
        )
        .init();

    let mut config = Config::from_env()?;
    let storage = storage::S3Store::new(config.s3_bucket.clone(), config.s3_region.clone()).await.map(Arc::new);
    if let Some(store) = &storage {
        if !config.knowledge_key.trim().is_empty() {
            let target = std::env::temp_dir().join("previdenciar-conhecimento");
            let _ = std::fs::remove_dir_all(&target);
            if let Err(error) = store.download_knowledge(&config.knowledge_key, &target).await {
                tracing::error!(error = %error, "knowledge sync failed; starting without imported base");
            } else {
                config.ai_data_root = target.to_string_lossy().to_string();
            }
        }
    }
    let mut connect_options: sqlx::postgres::PgConnectOptions = config.database_url.parse()?;
    if let Some((user, password)) = &config.database_credentials {
        connect_options = connect_options.username(user).password(password);
    }
    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect_with(connect_options)
        .await?;
    sqlx::migrate!("./migrations").run(&db).await?;

    let state = AppState {
        config: Arc::new(config.clone()),
        db,
        http: reqwest::Client::new(),
        jwks: Arc::new(RwLock::new(None)),
        storage,
        limits: Arc::new(limits::RateLimiter::new()),
        blocking: Arc::new(blocking::BlockingGate::new()),
    };

    let mut router = routes::router(state);
    let mut allowed_headers = vec![
        axum::http::header::AUTHORIZATION,
        axum::http::header::CONTENT_TYPE,
        HeaderName::from_static("x-fabrica-key"),
    ];
    if config.dev_user_header_allowed() {
        allowed_headers.push(HeaderName::from_static("x-dev-user"));
    }
    let cors = if config.allowed_origins.is_empty() {
        CorsLayer::new()
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
            .allow_headers(allowed_headers.clone())
    } else {
        let origins = config
            .allowed_origins
            .iter()
            .filter_map(|origin| origin.parse::<HeaderValue>().ok())
            .collect::<Vec<_>>();
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
            .allow_headers(allowed_headers.clone())
    };
    router = security_headers(router)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    tracing::info!(addr = %config.bind_addr, "previdenciar web server listening");
    if !config.trust_proxy {
        tracing::info!("IP do cliente vem do endereco de conexao; havendo proxy ou CDN, defina TRUST_PROXY=1");
    }
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

fn security_headers(router: Router) -> Router {
    router
        .layer(middleware::from_fn(cache_control_no_store_in_api))
        .layer(SetResponseHeaderLayer::overriding(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(
                "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; frame-src blob:",
            ),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::REFERRER_POLICY,
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static(
                "camera=(), microphone=(), geolocation=(), payment=(), usb=(), interest-cohort=()",
            ),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            HeaderName::from_static("cross-origin-opener-policy"),
            HeaderValue::from_static("same-origin"),
        ))
        .layer(middleware::from_fn(redirect_plain_http))
}

/// O HTTPS termina no Cloudflare, que avisa o esquema original em `CF-Visitor`.
/// Visita feita em `http://` recebe 301 para o mesmo endereço em `https://`.
/// Verificações internas (sem `CF-Visitor`) e chamadas à API não são redirecionadas.
async fn redirect_plain_http(request: Request, next: Next) -> Response {
    let plain_http = request
        .headers()
        .get("cf-visitor")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.replace(' ', "").contains("\"scheme\":\"http\""));
    if plain_http && !request.uri().path().starts_with("/api/") {
        if let Some(host) = request
            .headers()
            .get(header::HOST)
            .and_then(|value| value.to_str().ok())
            .filter(|host| {
                !host.is_empty()
                    && host
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':'))
            })
        {
            let path = request
                .uri()
                .path_and_query()
                .map(|value| value.as_str())
                .unwrap_or("/");
            if let Ok(location) = HeaderValue::from_str(&format!("https://{host}{path}")) {
                let mut response = Response::new(axum::body::Body::empty());
                *response.status_mut() = axum::http::StatusCode::MOVED_PERMANENTLY;
                response.headers_mut().insert(header::LOCATION, location);
                return response;
            }
        }
    }
    next.run(request).await
}

async fn cache_control_no_store_in_api(request: Request, next: Next) -> Response {
    let is_api = request.uri().path().starts_with("/api/");
    let mut response = next.run(request).await;
    if is_api {
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
    response
}




