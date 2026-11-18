use std::env;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Environment {
    Development,
    Test,
    Production,
}

impl Environment {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "development" | "dev" => Some(Self::Development),
            "test" => Some(Self::Test),
            "production" | "prod" => Some(Self::Production),
            _ => None,
        }
    }

    pub fn is_dev_or_test(self) -> bool {
        matches!(self, Self::Development | Self::Test)
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub environment: Environment,
    pub database_url: String,
    /// Usuário e senha vindos do segredo gerenciado pelo RDS (`DB_USERNAME`/`DB_PASSWORD`).
    /// Quando presentes, substituem os da `DATABASE_URL`, que sobrevive à rotação da senha.
    pub database_credentials: Option<(String, String)>,
    pub bind_addr: String,
    pub web_dist: Option<String>,
    pub allowed_origins: Vec<String>,
    pub auth_disabled: bool,
    pub trust_proxy: bool,
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
    pub fabrica_base_url: String,
    pub fabrica_api_key: String,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        Self::from_lookup(|key| env::var(key).ok())
    }

    fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, String> {
        let database_url = lookup("DATABASE_URL")
            .ok_or_else(|| "DATABASE_URL não configurada.".to_owned())?;
        let non_empty = |key: &str| lookup(key).filter(|value| !value.is_empty());
        let database_credentials = match (non_empty("DB_USERNAME"), non_empty("DB_PASSWORD")) {
            (Some(user), Some(password)) => Some((user, password)),
            (None, None) => None,
            _ => return Err("Configure DB_USERNAME e DB_PASSWORD juntos, ou nenhum dos dois.".into()),
        };
        let environment = match lookup("APP_ENV") {
            Some(raw) if !raw.trim().is_empty() => Environment::parse(&raw).unwrap_or_else(|| {
                tracing::warn!(
                    app_env = raw.trim(),
                    "APP_ENV desconhecido; usando production. Valores aceitos: development, test, production."
                );
                Environment::Production
            }),
            _ => Environment::Production,
        };
        let auth_disabled_requested = lookup("AUTH_DISABLED")
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes"))
            .unwrap_or(false);
        let auth_disabled = if !auth_disabled_requested {
            false
        } else if environment.is_dev_or_test() {
            true
        } else {
            tracing::warn!(
                "AUTH_DISABLED=1 ignorado: a autenticacao so pode ser desativada com APP_ENV=development ou APP_ENV=test."
            );
            false
        };
        let trust_proxy = lookup("TRUST_PROXY")
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes"))
            .unwrap_or(false);
        let cognito_region = lookup("COGNITO_REGION").unwrap_or_else(|| "sa-east-1".into());
        let cognito_user_pool_id = lookup("COGNITO_USER_POOL_ID").unwrap_or_default();
        let cognito_client_id = lookup("COGNITO_CLIENT_ID").unwrap_or_default();
        let cognito_domain = lookup("COGNITO_DOMAIN").unwrap_or_default();
        if !auth_disabled && (cognito_user_pool_id.is_empty() || cognito_client_id.is_empty()) {
            return Err("Configure COGNITO_USER_POOL_ID e COGNITO_CLIENT_ID ou use AUTH_DISABLED=1 com APP_ENV=development.".into());
        }
        Ok(Self {
            environment,
            database_url,
            database_credentials,
            bind_addr: lookup("BIND_ADDR").unwrap_or_else(|| "0.0.0.0:8787".into()),
            web_dist: lookup("WEB_DIST").filter(|value| !value.trim().is_empty()),
            allowed_origins: lookup("ALLOWED_ORIGINS")
                .unwrap_or_default()
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect(),
            auth_disabled,
            trust_proxy,
            cognito_region,
            cognito_user_pool_id,
            cognito_client_id,
            cognito_domain,
            ai_provider_id: lookup("AI_PROVIDER_ID").unwrap_or_else(|| "opencode-go".into()),
            ai_base_url: lookup("AI_BASE_URL")
                .unwrap_or_else(|| "https://opencode.ai/inference/go/openai/v1".into()),
            ai_model: lookup("AI_MODEL").unwrap_or_else(|| "deepseek-v4.1-flash".into()),
            ai_api_key: lookup("AI_API_KEY").unwrap_or_default(),
            ai_data_root: lookup("AI_DATA_ROOT")
                .unwrap_or_else(|| "./data/conhecimento".into()),
            s3_bucket: lookup("S3_BUCKET").unwrap_or_default(),
            s3_region: lookup("S3_REGION").unwrap_or_else(|| "sa-east-1".into()),
            knowledge_key: lookup("KNOWLEDGE_KEY").unwrap_or_default(),
            admin_emails: lookup("ADMIN_EMAILS")
                .unwrap_or_default()
                .split(',')
                .map(|value| value.trim().to_lowercase())
                .filter(|value| !value.is_empty())
                .collect(),
            terms_version: lookup("TERMS_VERSION").unwrap_or_else(|| "2026-10-31".into()),
            fabrica_base_url: lookup("FABRICA_BASE_URL")
                .unwrap_or_else(|| "https://fabrica-de-calculos.app.trf3.jus.br".into())
                .trim_end_matches('/')
                .to_owned(),
            fabrica_api_key: lookup("FABRICA_API_KEY").unwrap_or_default().trim().to_owned(),
        })
    }

    #[cfg(test)]
    pub fn for_tests() -> Self {
        Self {
            environment: Environment::Test,
            database_url: "postgres://teste:inexistente@127.0.0.1:9/sem_banco".to_owned(),
            database_credentials: None,
            bind_addr: "127.0.0.1:0".to_owned(),
            web_dist: None,
            allowed_origins: Vec::new(),
            auth_disabled: true,
            trust_proxy: false,
            cognito_region: "sa-east-1".to_owned(),
            cognito_user_pool_id: "sa-east-1_teste".to_owned(),
            cognito_client_id: "cliente-teste".to_owned(),
            cognito_domain: String::new(),
            ai_provider_id: "opencode-go".to_owned(),
            ai_base_url: "http://127.0.0.1:9".to_owned(),
            ai_model: "modelo-teste".to_owned(),
            ai_api_key: String::new(),
            ai_data_root: "./data/conhecimento".to_owned(),
            s3_bucket: String::new(),
            s3_region: "sa-east-1".to_owned(),
            knowledge_key: String::new(),
            admin_emails: vec!["admin@teste.local".to_owned()],
            terms_version: "2026-10-31".to_owned(),
            fabrica_base_url: "https://fabrica-de-calculos.app.trf3.jus.br".to_owned(),
            fabrica_api_key: String::new(),
        }
    }

    pub fn fabrica_enabled(&self) -> bool {
        !self.fabrica_api_key.trim().is_empty()
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

    pub fn dev_user_header_allowed(&self) -> bool {
        self.environment.is_dev_or_test()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env_lookup(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let vars: HashMap<String, String> = vars
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect();
        move |key| vars.get(key).cloned()
    }

    fn config_with(extra: &[(&str, &str)]) -> Result<Config, String> {
        let mut vars = vec![
            ("DATABASE_URL", "postgres://teste@127.0.0.1:9/sem_banco"),
            ("COGNITO_USER_POOL_ID", "sa-east-1_teste"),
            ("COGNITO_CLIENT_ID", "cliente-teste"),
        ];
        vars.extend_from_slice(extra);
        Config::from_lookup(env_lookup(&vars))
    }

    #[test]
    fn credenciais_do_banco_exigem_usuario_e_senha_juntos() {
        assert!(config_with(&[]).expect("config").database_credentials.is_none());
        let config = config_with(&[("DB_USERNAME", "u"), ("DB_PASSWORD", "p")]).expect("config");
        assert_eq!(config.database_credentials, Some(("u".into(), "p".into())));
        assert!(config_with(&[("DB_PASSWORD", "p")]).is_err());
    }

    #[test]
    fn auth_disabled_e_ignorado_em_producao() {
        let config =
            config_with(&[("APP_ENV", "production"), ("AUTH_DISABLED", "1")]).expect("config");
        assert_eq!(config.environment, Environment::Production);
        assert!(!config.auth_disabled);
        assert!(!config.dev_user_header_allowed());
    }

    #[test]
    fn auth_disabled_funciona_em_desenvolvimento() {
        let config =
            config_with(&[("APP_ENV", "development"), ("AUTH_DISABLED", "1")]).expect("config");
        assert_eq!(config.environment, Environment::Development);
        assert!(config.auth_disabled);
        assert!(config.dev_user_header_allowed());
    }

    #[test]
    fn auth_disabled_funciona_em_teste() {
        let config = config_with(&[("APP_ENV", "test"), ("AUTH_DISABLED", "1")]).expect("config");
        assert_eq!(config.environment, Environment::Test);
        assert!(config.auth_disabled);
        assert!(config.dev_user_header_allowed());
    }

    #[test]
    fn app_env_ausente_e_tratado_como_producao() {
        let config = config_with(&[("AUTH_DISABLED", "1")]).expect("config");
        assert_eq!(config.environment, Environment::Production);
        assert!(!config.auth_disabled);
        assert!(!config.dev_user_header_allowed());
    }

    #[test]
    fn app_env_desconhecido_e_tratado_como_producao() {
        let config =
            config_with(&[("APP_ENV", "staging"), ("AUTH_DISABLED", "1")]).expect("config");
        assert_eq!(config.environment, Environment::Production);
        assert!(!config.auth_disabled);
        assert!(!config.dev_user_header_allowed());
    }

    #[test]
    fn producao_exige_cognito_mesmo_com_auth_disabled() {
        let error = config_with(&[
            ("APP_ENV", "production"),
            ("AUTH_DISABLED", "1"),
            ("COGNITO_USER_POOL_ID", ""),
            ("COGNITO_CLIENT_ID", ""),
        ])
        .expect_err("producao sem cognito deve falhar");
        assert!(error.contains("COGNITO_USER_POOL_ID"));
    }

    #[test]
    fn desenvolvimento_sem_cognito_aceita_auth_disabled() {
        let config = config_with(&[
            ("APP_ENV", "development"),
            ("AUTH_DISABLED", "1"),
            ("COGNITO_USER_POOL_ID", ""),
            ("COGNITO_CLIENT_ID", ""),
        ])
        .expect("config");
        assert!(config.auth_disabled);
    }

    #[test]
    fn sem_auth_disabled_a_flag_fica_falsa_em_qualquer_ambiente() {
        for app_env in ["development", "test", "production"] {
            let config = config_with(&[("APP_ENV", app_env)]).expect("config");
            assert!(!config.auth_disabled);
        }
    }

    #[test]
    fn environment_parse_reconhece_aliases() {
        assert_eq!(Environment::parse("dev"), Some(Environment::Development));
        assert_eq!(
            Environment::parse("DEVELOPMENT"),
            Some(Environment::Development)
        );
        assert_eq!(Environment::parse("test"), Some(Environment::Test));
        assert_eq!(Environment::parse("prod"), Some(Environment::Production));
        assert_eq!(Environment::parse(" production "), Some(Environment::Production));
        assert_eq!(Environment::parse("staging"), None);
        assert_eq!(Environment::parse(""), None);
    }

    #[test]
    fn header_dev_user_somente_em_dev_ou_teste() {
        let dev = config_with(&[("APP_ENV", "development")]).expect("config");
        let test = config_with(&[("APP_ENV", "test")]).expect("config");
        let prod = config_with(&[("APP_ENV", "production")]).expect("config");
        assert!(dev.dev_user_header_allowed());
        assert!(test.dev_user_header_allowed());
        assert!(!prod.dev_user_header_allowed());
    }
}
