use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub web_dist: Option<String>,
    pub allowed_origins: Vec<String>,
    pub auth_disabled: bool,
    pub cognito_region: String,
    pub cognito_user_pool_id: String,
    pub cognito_client_id: String,
    pub cognito_domain: String,
    pub ai_provider_id: String,
    pub ai_base_url: String,
    pub ai_model: String,
    pub ai_api_key: String,
    pub ai_data_root: String,
    pub s3_bucket: String,
    pub s3_region: String,
    pub knowledge_key: String,
    pub admin_emails: Vec<String>,
    pub terms_version: String,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| "DATABASE_URL nÃ£o configurada.".to_owned())?;
        let auth_disabled = env::var("AUTH_DISABLED")
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes"))
            .unwrap_or(false);
        let cognito_region = env::var("COGNITO_REGION").unwrap_or_else(|_| "sa-east-1".into());
        let cognito_user_pool_id = env::var("COGNITO_USER_POOL_ID").unwrap_or_default();
        let cognito_client_id = env::var("COGNITO_CLIENT_ID").unwrap_or_default();
        let cognito_domain = env::var("COGNITO_DOMAIN").unwrap_or_default();
        if !auth_disabled && (cognito_user_pool_id.is_empty() || cognito_client_id.is_empty()) {
            return Err("Configure COGNITO_USER_POOL_ID e COGNITO_CLIENT_ID ou use AUTH_DISABLED=1 no desenvolvimento.".into());
        }
        Ok(Self {
            database_url,
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8787".into()),
            web_dist: env::var("WEB_DIST").ok().filter(|value| !value.trim().is_empty()),
            allowed_origins: env::var("ALLOWED_ORIGINS")
                .unwrap_or_default()
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect(),
            auth_disabled,
            cognito_region,
            cognito_user_pool_id,
            cognito_client_id,
            cognito_domain,
            ai_provider_id: env::var("AI_PROVIDER_ID").unwrap_or_else(|_| "opencode-go".into()),
            ai_base_url: env::var("AI_BASE_URL")
                .unwrap_or_else(|_| "https://opencode.ai/inference/go/openai/v1".into()),
            ai_model: env::var("AI_MODEL").unwrap_or_else(|_| "deepseek-v4.1-flash".into()),
            ai_api_key: env::var("AI_API_KEY").unwrap_or_default(),
            ai_data_root: env::var("AI_DATA_ROOT")
                .unwrap_or_else(|_| "./data/conhecimento".into()),
            s3_bucket: env::var("S3_BUCKET").unwrap_or_default(),
            s3_region: env::var("S3_REGION").unwrap_or_else(|_| "sa-east-1".into()),
            knowledge_key: env::var("KNOWLEDGE_KEY").unwrap_or_default(),
            admin_emails: env::var("ADMIN_EMAILS")
                .unwrap_or_default()
                .split(',')
                .map(|value| value.trim().to_lowercase())
                .filter(|value| !value.is_empty())
                .collect(),
            terms_version: env::var("TERMS_VERSION").unwrap_or_else(|_| "2026-10-31".into()),
        })
    }

    pub fn ai_enabled(&self) -> bool {
        !self.ai_api_key.trim().is_empty()
    }

    pub fn s3_enabled(&self) -> bool {
        !self.s3_bucket.trim().is_empty()
    }

    pub fn is_admin(&self, email: &str) -> bool {
        let email = email.to_lowercase();
        self.admin_emails.iter().any(|candidate| candidate == &email)
    }

    pub fn cognito_issuer(&self) -> String {
        format!(
            "https://cognito-idp.{}.amazonaws.com/{}",
            self.cognito_region, self.cognito_user_pool_id
        )
    }

    pub fn jwks_url(&self) -> String {
        format!("{}/.well-known/jwks.json", self.cognito_issuer())
    }
}


