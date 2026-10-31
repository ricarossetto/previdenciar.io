mod auth;
mod config;
mod db;
mod routes;
mod storage;

use axum::http::{HeaderName, HeaderValue, Method};
use config::Config;
use jsonwebtoken::jwk::JwkSet;
use sqlx::postgres::PgPoolOptions;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: sqlx::PgPool,
    pub http: reqwest::Client,
    pub jwks: Arc<RwLock<Option<(Instant, JwkSet)>>>,
    pub storage: Option<Arc<storage::S3Store>>,
    pub limits: Arc<tokio::sync::Mutex<HashMap<String, Vec<Instant>>>>,
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
    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&db).await?;

    let state = AppState {
        config: Arc::new(config.clone()),
        db,
        http: reqwest::Client::new(),
        jwks: Arc::new(RwLock::new(None)),
        storage,
        limits: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
    };

    let mut router = routes::router(state);
    let cors = if config.allowed_origins.is_empty() {
        CorsLayer::new()
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
            .allow_headers([
                axum::http::header::AUTHORIZATION,
                axum::http::header::CONTENT_TYPE,
                HeaderName::from_static("x-dev-user"),
            ])
    } else {
        let origins = config
            .allowed_origins
            .iter()
            .filter_map(|origin| origin.parse::<HeaderValue>().ok())
            .collect::<Vec<_>>();
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
            .allow_headers([
                axum::http::header::AUTHORIZATION,
                axum::http::header::CONTENT_TYPE,
                HeaderName::from_static("x-dev-user"),
            ])
    };
    router = router.layer(cors).layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    tracing::info!(addr = %config.bind_addr, "previdenciar web server listening");
    axum::serve(listener, router).await?;
    Ok(())
}



