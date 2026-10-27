//! HTTP boundary for the optional Fábrica de Cálculos/TRF3 conference.
//! The key is used only in the x-api-key header and is never logged or echoed.

use previus_rgps::fabrica::{self, FabricaInput};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::Read;

const MAX_RESPONSE: usize = 8 * 1024 * 1024;

fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(90))
        .connect_timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .http1_only()
        .user_agent("previdenciar.io/fabrica-rmi")
        .build()
        .map_err(|_| "Cliente da Fábrica de Cálculos indisponível.".to_owned())
}

fn valid_key(key: &str) -> bool {
    let key = key.trim();
    (8..=512).contains(&key.len()) && key.chars().all(|c| c.is_ascii_graphic())
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

pub fn conferir(input: FabricaInput, api_key: String) -> Result<Value, String> {
    let key = api_key.trim();
    if !valid_key(key) {
        return Err("Informe a chave da API da Fábrica em Configurações.".into());
    }
    let body = fabrica::build_body(&input)?;
    let body_bytes = serde_json::to_vec(&body)
        .map_err(|_| "Não foi possível preparar o pedido da conferência.".to_string())?;
    let client = client()?;
    let response_bytes = post_to(&client, fabrica::ENDPOINT, key, &body_bytes)?;
    let provider = fabrica::parse_summary(&response_bytes)?;
    Ok(json!({
        "provider": provider,
        "source_url": fabrica::ENDPOINT,
        "retrieved_at": chrono::Utc::now().to_rfc3339(),
        "request_sha256": digest(&body_bytes),
        "response_sha256": digest(&response_bytes),
        "bytes": response_bytes.len(),
        "api": "Fábrica de Cálculos/TRF3",
        "version": 1,
    }))
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

