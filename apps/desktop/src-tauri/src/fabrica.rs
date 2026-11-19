//! HTTP boundary for the optional Fábrica de Cálculos/TRF3 conference.
//! The key is used only in the x-api-key header and is never logged or echoed.

use previdenciar_ai::SecretVault;
use previdenciar_rgps::fabrica::{self, FabricaInput};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Read;

const MAX_RESPONSE: usize = 24 * 1024 * 1024;
const MAX_UPLOAD: usize = 32 * 1024 * 1024;
const KEY_NAME: &str = "trf3-fabrica";

fn client_with_timeout(secs: u64) -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(secs))
        .connect_timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .http1_only()
        .user_agent("previdenciar.io/fabrica-rmi")
        .build()
        .map_err(|_| "Cliente da Fábrica de Cálculos indisponível.".to_owned())
}

fn client() -> Result<reqwest::blocking::Client, String> {
    client_with_timeout(120)
}

pub fn valid_key(key: &str) -> bool {
    let key = key.trim();
    (8..=512).contains(&key.len()) && key.chars().all(|c| c.is_ascii_graphic())
}

pub fn resolve_key(explicit: Option<&str>, vault: Option<&SecretVault>) -> Option<String> {
    if let Some(raw) = explicit {
        let trimmed = raw.trim();
        if valid_key(trimmed) {
            return Some(trimmed.to_owned());
        }
    }
    if let Some(v) = vault {
        if let Ok(saved) = v.get_key(KEY_NAME) {
            let trimmed = saved.trim().to_owned();
            if valid_key(&trimmed) {
                return Some(trimmed);
            }
        }
    }
    if let Ok(env_key) = std::env::var("FABRICA_API_KEY") {
        let trimmed = env_key.trim().to_owned();
        if valid_key(&trimmed) {
            return Some(trimmed);
        }
    }
    None
}

pub fn key_status(vault: Option<&SecretVault>) -> Value {
    let vault_configured = vault
        .and_then(|v| v.has_key(KEY_NAME).ok())
        .unwrap_or(false);
    let env_configured = std::env::var("FABRICA_API_KEY")
        .ok()
        .is_some_and(|v| valid_key(&v));
    json!({
        "configured": vault_configured || env_configured,
        "vaultConfigured": vault_configured,
        "serverKeyAvailable": env_configured,
        "upstream": fabrica::BASE_URL,
    })
}

pub fn save_key(vault: &SecretVault, api_key: &str) -> Result<(), String> {
    let key = api_key.trim();
    if !valid_key(key) {
        return Err("Informe uma chave válida da API da Fábrica TRF3 (sem espaços).".into());
    }
    vault
        .set_key(KEY_NAME, key)
        .map_err(|e| format!("Falha ao salvar chave no cofre seguro: {e}"))
}

pub fn clear_key(vault: &SecretVault) -> Result<(), String> {
    vault
        .delete_key(KEY_NAME)
        .map_err(|e| format!("Falha ao remover chave do cofre seguro: {e}"))
}

fn messages(bytes: &[u8]) -> String {
    let Ok(value) = serde_json::from_slice::<Value>(bytes) else {
        return String::new();
    };
    value
        .get("messages")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .take(3)
                .filter_map(Value::as_str)
                .map(|text| text.chars().take(200).collect::<String>())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

fn read_limited(response: reqwest::blocking::Response, limit: usize) -> Result<Vec<u8>, String> {
    if response
        .content_length()
        .is_some_and(|size| size > limit as u64)
    {
        return Err("A resposta da Fábrica excede o limite de tamanho.".into());
    }
    let mut bytes = Vec::new();
    response
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Falha ao ler a resposta da Fábrica.".to_string())?;
    if bytes.len() > limit {
        return Err("A resposta da Fábrica excede o limite de tamanho.".into());
    }
    Ok(bytes)
}

fn request(
    client: &reqwest::blocking::Client,
    url: &str,
    key: &str,
    body: &[u8],
) -> reqwest::blocking::RequestBuilder {
    client
        .post(url)
        .header("x-api-key", key)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(reqwest::header::ACCEPT, "application/json")
        .header(reqwest::header::ACCEPT_LANGUAGE, "pt-BR,pt;q=0.9")
        .header(reqwest::header::ACCEPT_ENCODING, "identity")
        .header(reqwest::header::CONNECTION, "close")
        .body(body.to_vec())
}

fn post_to(
    client: &reqwest::blocking::Client,
    url: &str,
    key: &str,
    body: &[u8],
) -> Result<Vec<u8>, String> {
    let response = request(client, url, key, body)
        .send()
        .map_err(|_| {
            "Não foi possível falar com a Fábrica de Cálculos; a conferência não alterou nenhum valor."
                .to_string()
        })?;
    match response.status().as_u16() {
        200..=299 => read_limited(response, MAX_RESPONSE),
        401 => Err("A Fábrica recusou a chave (HTTP 401). Confira a chave em Configurações.".into()),
        403 => Err("A chave não possui o escopo rgps exigido pela Fábrica (HTTP 403).".into()),
        429 => Err("Limite temporário de uso da Fábrica; aguarde antes de tentar novamente.".into()),
        status => {
            let bytes = read_limited(response, 64 * 1024).unwrap_or_default();
            let detail = messages(&bytes);
            Err(if detail.is_empty() {
                format!("A Fábrica recusou a conferência (HTTP {status}).")
            } else {
                format!("A Fábrica recusou a conferência (HTTP {status}): {detail}")
            })
        }
    }
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn get_from(client: &reqwest::blocking::Client, url: &str, key: Option<&str>) -> Result<Vec<u8>, String> {
    let mut req = client
        .get(url)
        .header(reqwest::header::ACCEPT, "application/json")
        .header(reqwest::header::ACCEPT_LANGUAGE, "pt-BR,pt;q=0.9")
        .header(reqwest::header::ACCEPT_ENCODING, "identity")
        .header(reqwest::header::CONNECTION, "close");
    if let Some(k) = key.filter(|s| valid_key(s)) {
        req = req.header("x-api-key", k.trim());
    }
    let response = req
        .send()
        .map_err(|_| "Não foi possível consultar os dados da Fábrica TRF3.".to_string())?;
    match response.status().as_u16() {
        200..=299 => read_limited(response, MAX_RESPONSE),
        401 => Err("A Fábrica recusou a chave (HTTP 401). Confira a chave em Configurações.".into()),
        403 => Err("A chave não possui permissão para este recurso na Fábrica (HTTP 403).".into()),
        429 => Err("Limite temporário de uso da Fábrica; aguarde antes de tentar novamente.".into()),
        status => {
            let bytes = read_limited(response, 64 * 1024).unwrap_or_default();
            let detail = messages(&bytes);
            Err(if detail.is_empty() {
                format!("A Fábrica retornou HTTP {status}.")
            } else {
                format!("A Fábrica retornou HTTP {status}: {detail}")
            })
        }
    }
}

pub fn conferir(input: FabricaInput, api_key: String) -> Result<Value, String> {
    conferir_with_options(input, api_key, None, None, None)
}

pub fn conferir_with_options(
    input: FabricaInput,
    api_key: String,
    demonstrativo: Option<bool>,
    permalink: Option<bool>,
    fundamento: Option<bool>,
) -> Result<Value, String> {
    let key = api_key.trim();
    if !valid_key(key) {
        return Err("Informe a chave da API da Fábrica em Configurações.".into());
    }
    let demo = demonstrativo.unwrap_or(input.demonstrativo);
    let perm = permalink.unwrap_or(input.permalink);
    let use_fund = fundamento.unwrap_or(false)
        && input
            .fundamento
            .as_deref()
            .is_some_and(|s| !s.trim().is_empty());
    let body = fabrica::build_body(&input)?;
    let body_bytes = serde_json::to_vec(&body)
        .map_err(|_| "Não foi possível preparar o pedido da conferência.".to_string())?;
    let base = if use_fund {
        fabrica::ENDPOINT_RMI_POR_FUNDAMENTO
    } else {
        fabrica::ENDPOINT_RMI
    };
    let url = format!("{base}{}", fabrica::query_suffix(demo, perm));
    let client = client()?;
    let response_bytes = post_to(&client, &url, key, &body_bytes)?;
    let provider = fabrica::parse_summary(&response_bytes)?;
    Ok(json!({
        "provider": provider,
        "source_url": url,
        "retrieved_at": chrono::Utc::now().to_rfc3339(),
        "request_sha256": digest(&body_bytes),
        "response_sha256": digest(&response_bytes),
        "bytes": response_bytes.len(),
        "api": "Fábrica de Cálculos/TRF3",
        "version": 1,
    }))
}

pub fn calcular_tempo(
    input_or_body: Value,
    api_key: String,
    por_grupos: bool,
    demonstrativo: Option<bool>,
    permalink: Option<bool>,
) -> Result<Value, String> {
    let key = api_key.trim();
    if !valid_key(key) {
        return Err("Informe a chave da API da Fábrica em Configurações.".into());
    }
    let (body, inp_demo, inp_perm) =
        if input_or_body.get("periodos").is_some() && input_or_body.get("periodosContribuicao").is_none() {
            let input: FabricaInput =
                serde_json::from_value(input_or_body).map_err(|e| format!("Entrada inválida para Tempo TRF3: {e}"))?;
            let demo = input.demonstrativo;
            let perm = input.permalink;
            (fabrica::build_tempo_body(&input)?, demo, perm)
        } else {
            (input_or_body, false, false)
        };
    let body_bytes = serde_json::to_vec(&body)
        .map_err(|_| "Não foi possível preparar o cálculo de tempo.".to_string())?;
    let base = if por_grupos {
        fabrica::ENDPOINT_TEMPO_GRUPOS
    } else {
        fabrica::ENDPOINT_TEMPO
    };
    let suffix = fabrica::query_suffix(demonstrativo.unwrap_or(inp_demo), permalink.unwrap_or(inp_perm));
    let url = format!("{base}{suffix}");
    let client = client()?;
    let response_bytes = post_to(&client, &url, key, &body_bytes)?;
    fabrica::parse_tempo_summary(&response_bytes)
}

pub fn calcular_atrasados(
    body: Value,
    api_key: String,
    demonstrativo: bool,
    permalink: bool,
) -> Result<Value, String> {
    let key = api_key.trim();
    if !valid_key(key) {
        return Err("Informe a chave da API da Fábrica em Configurações.".into());
    }
    let body_bytes = serde_json::to_vec(&body)
        .map_err(|_| "Não foi possível preparar o pedido de atrasados.".to_string())?;
    let url = format!(
        "{}{}",
        fabrica::ENDPOINT_ATRASADOS,
        fabrica::query_suffix(demonstrativo, permalink)
    );
    let client = client()?;
    let response_bytes = post_to(&client, &url, key, &body_bytes)?;
    fabrica::parse_atrasados_summary(&response_bytes)
}

pub fn ler_documento(tipo: &str, file_name: &str, bytes: &[u8], api_key: String) -> Result<Value, String> {
    if !fabrica::DOCUMENT_TYPES.contains(&tipo) {
        return Err(format!(
            "Tipo documental inválido: {tipo}. Aceitos: cnis, rdctc, carta, pap, prevjud."
        ));
    }
    let key = api_key.trim();
    if !valid_key(key) {
        return Err("Informe a chave da API da Fábrica em Configurações.".into());
    }
    if bytes.is_empty() || bytes.len() > MAX_UPLOAD {
        return Err("Arquivo vazio ou maior que 32 MiB.".into());
    }
    let clean_name: String = file_name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .take(64)
        .collect();
    let clean_name = if clean_name.is_empty() {
        "documento.pdf".to_string()
    } else {
        clean_name
    };
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(1);
    let boundary = format!("----PrevidenciarBoundary{nanos}");
    let mut payload = Vec::with_capacity(bytes.len() + 256);
    payload.extend_from_slice(
        format!(
            "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{clean_name}\"\r\nContent-Type: application/pdf\r\n\r\n"
        )
        .as_bytes(),
    );
    payload.extend_from_slice(bytes);
    payload.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let url = format!("{}/rgps/api/v1/documentos/{tipo}/ler", fabrica::BASE_URL);
    let client = client_with_timeout(180)?;
    let response = client
        .post(&url)
        .header("x-api-key", key)
        .header(
            reqwest::header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .header(reqwest::header::ACCEPT, "application/json")
        .header(reqwest::header::ACCEPT_LANGUAGE, "pt-BR,pt;q=0.9")
        .header(reqwest::header::ACCEPT_ENCODING, "identity")
        .header(reqwest::header::CONNECTION, "close")
        .body(payload)
        .send()
        .map_err(|_| "Falha ao contatar o leitor documental da Fábrica TRF3.".to_string())?;
    let status = response.status().as_u16();
    if !(200..=299).contains(&status) {
        let err_bytes = read_limited(response, 64 * 1024).unwrap_or_default();
        let detail = messages(&err_bytes);
        return Err(if detail.is_empty() {
            format!("Leitor documental TRF3 retornou HTTP {status}.")
        } else {
            format!("Leitor documental TRF3 retornou HTTP {status}: {detail}")
        });
    }
    let resp_bytes = read_limited(response, MAX_RESPONSE)?;
    fabrica::parse_documento_response(&resp_bytes)
}

pub fn consultar_indices(
    endpoint: &str,
    nome: Option<&str>,
    query: Option<BTreeMap<String, String>>,
    api_key: Option<&str>,
) -> Result<Value, String> {
    let path = match endpoint {
        "fundamentos" => "/rgps/api/v1/fundamentos".to_string(),
        "marcos-temporais" => "/rgps/api/v1/marcos-temporais".to_string(),
        "indice" => {
            let target = nome.unwrap_or_default().trim();
            if !fabrica::INDEX_TYPES.contains(&target) {
                return Err("Índice econômico não suportado pela Fábrica TRF3.".into());
            }
            let pairs: Vec<String> = query
                .unwrap_or_default()
                .into_iter()
                .filter(|(k, v)| {
                    matches!(k.as_str(), "competenciaInicial" | "competenciaFinal" | "indice")
                        && !v.trim().is_empty()
                        && v.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
                })
                .map(|(k, v)| format!("{k}={}", v.trim()))
                .collect();
            if pairs.is_empty() {
                format!("/indices/api/v1/{target}")
            } else {
                format!("/indices/api/v1/{target}?{}", pairs.join("&"))
            }
        }
        _ => return Err("Consulta à Fábrica TRF3 não reconhecida.".into()),
    };
    let url = format!("{}{}", fabrica::BASE_URL, path);
    let client = client_with_timeout(30)?;
    let resp_bytes = get_from(&client, &url, api_key)?;
    serde_json::from_slice(&resp_bytes)
        .map_err(|_| "Resposta inválida ao consultar tabela da Fábrica TRF3.".into())
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_shape_is_strict_and_never_echoed() {
        assert!(valid_key(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        ));
        assert!(!valid_key(""));
        assert!(!valid_key("short"));
        assert!(!valid_key("chave com espaço"));
        let error = conferir(
            serde_json::from_value(json!({
                "dataApuracao":"2026-01-10","dataNascimento":"1956-01-10","sexo":"M","especie":41,
                "periodos":[{"dataInicial":"1994-07-01","dataFinal":"2019-06-30","salarios":[{"competencia":"1994-07","valor":"2000.00"}]}]
            }))
            .unwrap(),
            "".into(),
        )
        .unwrap_err();
        assert!(error.contains("Configurações"));
        assert!(!error.contains("0123456789"));
    }

    #[test]
    fn request_carries_the_key_only_in_the_header_and_no_identity() {
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .build()
            .unwrap();
        let body: &[u8] = br#"{"dataApuracao":"2026-01-10"}"#;
        let built = request(&client, "http://127.0.0.1/synthetic-rmi", "test-key", body)
            .build()
            .unwrap();
        assert_eq!(
            built.headers().get("x-api-key").and_then(|value| value.to_str().ok()),
            Some("test-key")
        );
        assert_eq!(
            built.headers().get(reqwest::header::CONTENT_TYPE).and_then(|value| value.to_str().ok()),
            Some("application/json")
        );
        assert!(!built.headers().contains_key(reqwest::header::AUTHORIZATION));
        assert_eq!(built.body().and_then(|body| body.as_bytes()), Some(body));
    }

    #[test]
    fn provider_statuses_map_to_safe_messages() {
        use std::io::{Read, Write};
        for (status, expected) in [(401, "401"), (403, "escopo rgps"), (429, "Limite")] {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let worker = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                let mut buffer = [0; 4096];
                let _ = stream.read(&mut buffer);
                let response = format!(
                    "HTTP/1.1 {status} Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                );
                stream.write_all(response.as_bytes()).unwrap();
            });
            let client = reqwest::blocking::Client::builder()
                .no_proxy()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap();
            let error = post_to(
                &client,
                &format!("http://{address}/synthetic-rmi"),
                "test-key",
                b"{}",
            )
            .unwrap_err();
            assert!(error.contains(expected), "{status}: {error}");
            worker.join().unwrap();
        }
    }
}

