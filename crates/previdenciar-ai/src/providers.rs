use crate::error::{AiError, Result};
use reqwest::blocking::{Client, RequestBuilder, Response};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::io::Read;
use std::time::{Duration, Instant};

const USER_AGENT: &str = "previdenciar.io/desktop";
const ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    #[default]
    Openai,
    Anthropic,
}

impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Openai => "openai",
            Self::Anthropic => "anthropic",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderPreset {
    pub id: String,
    pub name: String,
    pub kind: ProviderKind,
    pub base_url: String,
    pub default_model: String,
    pub models: Vec<String>,
    pub key_required: bool,
    pub note: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    pub ok: bool,
    pub model: String,
    pub latency_ms: u128,
    pub sample: String,
}

fn preset(
    id: &str,
    name: &str,
    kind: ProviderKind,
    base_url: &str,
    default_model: &str,
    models: &[&str],
    key_required: bool,
    note: &str,
) -> ProviderPreset {
    ProviderPreset {
        id: id.into(),
        name: name.into(),
        kind,
        base_url: base_url.into(),
        default_model: default_model.into(),
        models: models.iter().map(|value| (*value).to_owned()).collect(),
        key_required,
        note: note.into(),
    }
}

/// Catalog of wire-compatible presets. Model identifiers are editable in the UI
/// because catalogs change faster than desktop releases.
pub fn catalog() -> Vec<ProviderPreset> {
    vec![
        preset(
            "opencode-go",
            "OpenCode Go - DeepSeek V4.1 Flash",
            ProviderKind::Openai,
            "https://opencode.ai/inference/go/openai/v1",
            "deepseek-v4.1-flash",
            &["deepseek-v4.1-flash"],
            true,
            "Preset observado no app operacional de referência. Confirme com o provedor a autorizacao de uso juridico e os limites do plano.",
        ),
        preset(
            "deepseek",
            "DeepSeek",
            ProviderKind::Openai,
            "https://api.deepseek.com",
            "deepseek-chat",
            &["deepseek-chat", "deepseek-reasoner"],
            true,
            "Endpoint oficial OpenAI-compatible. Modelos e disponibilidade podem mudar.",
        ),
        preset(
            "openai",
            "OpenAI",
            ProviderKind::Openai,
            "https://api.openai.com/v1",
            "gpt-4.1",
            &["gpt-4.1", "gpt-4.1-mini", "gpt-4o", "gpt-4o-mini"],
            true,
            "Use o identificador de modelo que sua conta realmente oferece.",
        ),
        preset(
            "anthropic",
            "Anthropic (Claude)",
            ProviderKind::Anthropic,
            "https://api.anthropic.com",
            "claude-sonnet-4-5",
            &["claude-sonnet-4-5", "claude-opus-4-1", "claude-3-7-sonnet-latest"],
            true,
            "Adaptador nativo da Messages API, com ferramentas e streaming.",
        ),
        preset(
            "gemini",
            "Google Gemini",
            ProviderKind::Openai,
            "https://generativelanguage.googleapis.com/v1beta/openai",
            "gemini-2.5-flash",
            &["gemini-2.5-flash", "gemini-2.5-pro"],
            true,
            "Usa a camada OpenAI-compatible do Gemini. Confira o modelo disponivel na sua conta.",
        ),
        preset(
            "qwen",
            "Qwen / DashScope",
            ProviderKind::Openai,
            "https://dashscope.aliyuncs.com/compatible-mode/v1",
            "qwen-plus",
            &["qwen-plus", "qwen-max", "qwen-turbo"],
            true,
            "Camada OpenAI-compatible do DashScope.",
        ),
        preset(
            "moonshot",
            "Moonshot / Kimi",
            ProviderKind::Openai,
            "https://api.moonshot.cn/v1",
            "moonshot-v1-8k",
            &["moonshot-v1-8k", "moonshot-v1-32k", "kimi-k2"],
            true,
            "Modelos e limites conforme a conta Moonshot.",
        ),
        preset(
            "zhipu",
            "Zhipu / GLM",
            ProviderKind::Openai,
            "https://open.bigmodel.cn/api/paas/v4",
            "glm-4-plus",
            &["glm-4-plus", "glm-4-air", "glm-4-flash"],
            true,
            "Camada OpenAI-compatible do Zhipu.",
        ),
        preset(
            "minimax",
            "MiniMax",
            ProviderKind::Openai,
            "https://api.minimax.chat/v1",
            "abab6.5s-chat",
            &["abab6.5s-chat", "MiniMax-Text-01"],
            true,
            "Confirme o identificador de modelo e a regiao do endpoint.",
        ),
        preset(
            "baidu",
            "Baidu Qianfan",
            ProviderKind::Openai,
            "https://qianfan.baidubce.com/v2",
            "ernie-4.5-turbo-128k",
            &["ernie-4.5-turbo-128k", "ernie-4.0-8k"],
            true,
            "A camada compativel depende do plano Qianfan contratado.",
        ),
        preset(
            "openrouter",
            "OpenRouter",
            ProviderKind::Openai,
            "https://openrouter.ai/api/v1",
            "deepseek/deepseek-chat",
            &[
                "deepseek/deepseek-chat",
                "anthropic/claude-sonnet-4",
                "google/gemini-2.5-flash",
            ],
            true,
            "Agregador OpenAI-compatible. O modelo deve incluir o prefixo do provedor.",
        ),
        preset(
            "ollama",
            "Ollama local",
            ProviderKind::Openai,
            "http://127.0.0.1:11434/v1",
            "llama3.2",
            &["llama3.2", "qwen2.5", "mistral"],
            false,
            "Execucao local. A chave e opcional; nenhum dado sai da maquina.",
        ),
        preset(
            "custom",
            "Personalizado / gateway OpenAI",
            ProviderKind::Openai,
            "",
            "",
            &[],
            false,
            "Informe a URL raiz e o modelo. Use esta opção para gateways e provedores BYOK compatíveis com OpenAI. CommandCode e Cline são agentes BYOK, não gateways universais; este aplicativo não é um provedor de modelo.",
        ),
        preset(
            "custom-anthropic",
            "Personalizado / gateway Anthropic",
            ProviderKind::Anthropic,
            "",
            "",
            &[],
            true,
            "Informe a URL raiz e o modelo de um gateway compativel com a Messages API da Anthropic. Chave obrigatoria.",
        ),
    ]
}

pub fn preset_by_id(id: &str) -> Option<ProviderPreset> {
    catalog().into_iter().find(|item| item.id == id)
}

pub fn key_required(id: &str) -> bool {
    preset_by_id(id)
        .map(|item| item.key_required)
        .unwrap_or(false)
}

pub fn client() -> Result<Client> {
    Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(240))
        .user_agent(USER_AGENT)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| AiError::Network("Não foi possível preparar a conexão HTTPS.".into()))
}

fn trim_root(base_url: &str) -> Result<&str> {
    let value = base_url.trim().trim_end_matches('/');
    if value.is_empty() {
        return Err(AiError::Invalid("Informe a URL base do provedor.".into()));
    }
    let parsed = reqwest::Url::parse(value)
        .map_err(|_| AiError::Invalid("A URL base do provedor e invalida.".into()))?;
    let local = matches!(parsed.host_str(), Some("127.0.0.1") | Some("localhost"));
    if parsed.scheme() != "https" && !(parsed.scheme() == "http" && local) {
        return Err(AiError::Invalid(
            "A URL deve usar HTTPS. HTTP so e aceito para servidor local em 127.0.0.1 ou localhost.".into(),
        ));
    }
    if !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.fragment().is_some()
        || parsed.query().is_some()
    {
        return Err(AiError::Invalid(
            "A URL base não pode conter credenciais, query ou fragmento.".into(),
        ));
    }
    Ok(value)
}

pub fn chat_endpoint(base_url: &str, kind: ProviderKind) -> Result<String> {
    let root = trim_root(base_url)?;
    match kind {
        ProviderKind::Openai => {
            if root.ends_with("/chat/completions") {
                Ok(root.to_owned())
            } else {
                Ok(format!("{root}/chat/completions"))
            }
        }
        ProviderKind::Anthropic => {
            if root.ends_with("/v1/messages") {
                Ok(root.to_owned())
            } else if root.ends_with("/v1") {
                Ok(format!("{root}/messages"))
            } else if root.ends_with("/messages") {
                Ok(root.to_owned())
            } else {
                Ok(format!("{root}/v1/messages"))
            }
        }
    }
}

pub fn models_endpoint(base_url: &str, kind: ProviderKind) -> Result<String> {
    let root = trim_root(base_url)?;
    if root == "https://opencode.ai/inference/go/openai/v1" {
        // The Go catalog is published outside the OpenAI-compatible route.
        return Ok("https://opencode.ai/inference/go/v1/models".into());
    }
    match kind {
        ProviderKind::Openai => {
            if root.ends_with("/models") {
                Ok(root.to_owned())
            } else if root.ends_with("/chat/completions") {
                Ok(format!(
                    "{}/models",
                    root.trim_end_matches("/chat/completions")
                ))
            } else {
                Ok(format!("{root}/models"))
            }
        }
        ProviderKind::Anthropic => {
            if root.ends_with("/v1") {
                Ok(format!("{root}/models"))
            } else if root.ends_with("/v1/messages") {
                Ok(format!("{}/models", root.trim_end_matches("/messages")))
            } else if root.ends_with("/models") {
                Ok(root.to_owned())
            } else {
                Ok(format!("{root}/v1/models"))
            }
        }
    }
}

fn is_opencode_go(base_url: &str) -> bool {
    base_url
        .trim()
        .to_ascii_lowercase()
        .contains("opencode.ai/inference/go/")
}

fn opencode_session() -> &'static str {
    static SESSION: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    SESSION.get_or_init(|| {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|value| value.as_nanos())
            .unwrap_or(0);
        format!("previdenciar-{}-{nanos:x}", std::process::id())
    })
}

pub(crate) fn apply_auth(
    request: RequestBuilder,
    kind: ProviderKind,
    key: &str,
    base_url: &str,
) -> RequestBuilder {
    let request = match kind {
        ProviderKind::Openai if !key.is_empty() => request.bearer_auth(key),
        ProviderKind::Anthropic => request
            .header("x-api-key", key)
            .header("anthropic-version", ANTHROPIC_VERSION),
        _ => request,
    };
    if is_opencode_go(base_url) {
        request.header("x-opencode-session", opencode_session())
    } else {
        request
    }
}

pub(crate) fn send_with_retry<F>(build: F) -> Result<Response>
where
    F: Fn() -> RequestBuilder,
{
    let mut last: Option<AiError> = None;
    for attempt in 0..3u32 {
        let response = match build().send() {
            Ok(response) => response,
            Err(_) if attempt < 2 => {
                last = Some(AiError::Network("Falha de rede ao consultar o provedor.".into()));
                std::thread::sleep(Duration::from_millis(500 * u64::from(attempt + 1)));
                continue;
            }
            Err(_) => {
                return Err(AiError::Network(
                    "Falha de rede ao consultar o provedor.".into(),
                ))
            }
        };
        if response.status().as_u16() == 429 || response.status().is_server_error() {
            if attempt < 2 {
                last = Some(provider_error(response));
                std::thread::sleep(Duration::from_millis(700 * u64::from(attempt + 1)));
                continue;
            }
            return Err(provider_error(response));
        }
        return Ok(response);
    }
    Err(last.unwrap_or_else(|| AiError::Network("Falha de rede ao consultar o provedor.".into())))
}
pub(crate) fn provider_error(response: Response) -> AiError {
    let status = response.status();
    let mut bytes = Vec::new();
    let _ = response.take(16 * 1024).read_to_end(&mut bytes);
    let body = String::from_utf8_lossy(&bytes);
    let message = serde_json::from_str::<Value>(&body)
        .ok()
        .and_then(|value| {
            value
                .pointer("/error/message")
                .or_else(|| value.get("message"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("Resposta HTTP {}.", status.as_u16()));
    let message = message.trim().replace(['\r', '\n'], " ");
    let message: String = message.chars().take(600).collect();
    AiError::Provider(format!(
        "Provedor recusou a requisição ({status}). {message}"
    ))
}

fn test_body(kind: ProviderKind, model: &str) -> Value {
    let prompt = "Responda apenas: ok";
    match kind {
        ProviderKind::Openai => json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": 128,
            "stream": false
        }),
        ProviderKind::Anthropic => json!({
            "model": model,
            "max_tokens": 128,
            "messages": [{"role": "user", "content": prompt}],
            "stream": false
        }),
    }
}

fn completion_text(kind: ProviderKind, value: &Value) -> String {
    match kind {
        ProviderKind::Openai => value
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .or_else(|| value.pointer("/choices/0/text").and_then(Value::as_str))
            .unwrap_or_default()
            .to_owned(),
        ProviderKind::Anthropic => value
            .get("content")
            .and_then(Value::as_array)
            .and_then(|items| {
                items
                    .iter()
                    .find_map(|item| item.get("text").and_then(Value::as_str))
            })
            .unwrap_or_default()
            .to_owned(),
    }
}

pub fn test_connection(
    kind: ProviderKind,
    base_url: &str,
    model: &str,
    key: &str,
) -> Result<TestResult> {
    let model = model.trim();
    if model.is_empty() {
        return Err(AiError::Invalid(
            "Informe o identificador do modelo.".into(),
        ));
    }
    if kind == ProviderKind::Anthropic && key.trim().is_empty() {
        return Err(AiError::MissingKey);
    }
    let started = Instant::now();
    let client = client()?;
    let endpoint = chat_endpoint(base_url, kind)?;
    let response = send_with_retry(|| {
        apply_auth(client.post(&endpoint), kind, key, base_url)
            .header("Accept", "application/json")
            .json(&test_body(kind, model))
    })?;
    if !response.status().is_success() {
        return Err(provider_error(response));
    }
    let value: Value = response
        .json()
        .map_err(|_| AiError::Protocol("O provedor respondeu em formato inesperado.".into()))?;
    let sample = completion_text(kind, &value);
    Ok(TestResult {
        ok: true,
        model: model.to_owned(),
        latency_ms: started.elapsed().as_millis(),
        sample: sample.chars().take(240).collect(),
    })
}

pub fn list_models(kind: ProviderKind, base_url: &str, key: &str) -> Result<Vec<String>> {
    let client = client()?;
    let endpoint = models_endpoint(base_url, kind)?;
    let response = send_with_retry(|| {
        apply_auth(client.get(&endpoint), kind, key, base_url).header("Accept", "application/json")
    })?;
    if !response.status().is_success() {
        return Err(provider_error(response));
    }
    let value: Value = response
        .json()
        .map_err(|_| AiError::Protocol("A lista de modelos veio em formato inesperado.".into()))?;
    let mut found = BTreeSet::new();
    for key_name in ["data", "models"] {
        if let Some(items) = value.get(key_name).and_then(Value::as_array) {
            for item in items {
                let id = item
                    .get("id")
                    .and_then(Value::as_str)
                    .or_else(|| item.get("name").and_then(Value::as_str))
                    .or_else(|| item.get("model").and_then(Value::as_str));
                if let Some(id) = id {
                    let clean = id.trim();
                    if !clean.is_empty() && clean.len() <= 200 {
                        found.insert(clean.to_owned());
                    }
                }
            }
        }
    }
    if found.is_empty() {
        return Err(AiError::Provider(
            "O provedor não retornou modelos. Informe o identificador manualmente.".into(),
        ));
    }
    Ok(found.into_iter().take(300).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_helper_retries_server_errors_before_returning_success() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            for attempt in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buffer = [0u8; 2048];
                let _ = stream.read(&mut buffer);
                let (status, body) = if attempt == 0 { ("500 Internal Server Error", "{\"error\":{\"message\":\"retry\"}}") } else { ("200 OK", "{}") };
                let response = format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                stream.write_all(response.as_bytes()).unwrap();
            }
        });
        let client = Client::builder().build().unwrap();
        let response = send_with_retry(|| client.get(format!("http://{address}/health"))).unwrap();
        assert!(response.status().is_success());
        server.join().unwrap();
    }
    #[test]
    fn catalog_contains_opencode_go_and_editable_presets() {
        let items = catalog();
        let opencode = items.iter().find(|item| item.id == "opencode-go").unwrap();
        assert_eq!(
            opencode.base_url,
            "https://opencode.ai/inference/go/openai/v1"
        );
        assert_eq!(opencode.default_model, "deepseek-v4.1-flash");
        let custom_anthropic = items.iter().find(|item| item.id == "custom-anthropic").unwrap();
        assert_eq!(custom_anthropic.kind, ProviderKind::Anthropic);
        assert!(custom_anthropic.key_required);
        assert!(items
            .iter()
            .any(|item| item.id == "custom" && !item.key_required));
    }

    #[test]
    fn endpoints_are_built_without_duplicate_suffixes() {
        assert_eq!(
            chat_endpoint("https://api.deepseek.com", ProviderKind::Openai).unwrap(),
            "https://api.deepseek.com/chat/completions"
        );
        assert_eq!(
            chat_endpoint(
                "https://example.test/v1/chat/completions",
                ProviderKind::Openai
            )
            .unwrap(),
            "https://example.test/v1/chat/completions"
        );
        assert_eq!(
            chat_endpoint("https://api.anthropic.com", ProviderKind::Anthropic).unwrap(),
            "https://api.anthropic.com/v1/messages"
        );
        assert_eq!(
            chat_endpoint("https://api.anthropic.com/v1", ProviderKind::Anthropic).unwrap(),
            "https://api.anthropic.com/v1/messages"
        );
        assert!(chat_endpoint("javascript:alert(1)", ProviderKind::Openai).is_err());
        assert!(chat_endpoint("http://localhost.evil.test/v1", ProviderKind::Openai).is_err());
        assert!(chat_endpoint("https://user:pass@example.test/v1", ProviderKind::Openai).is_err());
        assert!(chat_endpoint("http://127.0.0.1:11434/v1", ProviderKind::Openai).is_ok());
    }

    #[test]
    fn models_endpoint_handles_roots_and_routes() {
        assert_eq!(
            models_endpoint("https://api.openai.com/v1", ProviderKind::Openai).unwrap(),
            "https://api.openai.com/v1/models"
        );
        assert_eq!(
            models_endpoint("https://api.anthropic.com", ProviderKind::Anthropic).unwrap(),
            "https://api.anthropic.com/v1/models"
        );
        assert_eq!(
            models_endpoint(
                "https://opencode.ai/inference/go/openai/v1",
                ProviderKind::Openai
            )
            .unwrap(),
            "https://opencode.ai/inference/go/v1/models"
        );
    }
}


