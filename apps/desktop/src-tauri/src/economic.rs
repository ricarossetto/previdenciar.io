//! Public economic data only. A CNIS, identity or API key can never enter this request.
use previus_rgps::pcd::{Limits, Rate, Snapshot};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::io::Read;
const BASE: &str = "https://fabrica-de-calculos.app.trf3.jus.br/indices/api/v1/";
fn decimal(v: &Value) -> Result<String, String> {
    match v {
        Value::Number(n) => previus_rgps::pcd::canonical_snapshot_decimal(&n.to_string()),
        Value::String(s) if !s.trim().is_empty() => {
            previus_rgps::pcd::canonical_snapshot_decimal(&s.trim().replace(',', "."))
        }
        _ => Err("Índice/limite ausente no provedor; nenhum zero foi presumido.".into()),
    }
}
fn optional(v: &Value) -> Result<Option<String>, String> {
    if v.is_null() || v.as_str() == Some("") {
        Ok(None)
    } else {
        decimal(v).map(Some)
    }
}
fn get(client: &reqwest::blocking::Client, url: &str) -> Result<Vec<u8>, String> {
    // Explicit negotiation avoids the public gateway hanging on bare native requests.
    let response=client.get(url)
        .header(reqwest::header::ACCEPT, "application/json")
        .header(reqwest::header::ACCEPT_LANGUAGE, "pt-BR,pt;q=0.9")
        .header(reqwest::header::ACCEPT_ENCODING, "identity")
        .header(reqwest::header::CONNECTION, "close")
        .send().map_err(|_|"Não foi possível consultar os dados públicos; tente novamente, sem alterar o snapshot anterior.".to_owned())?;
    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err("Limite temporário do provedor; aguarde antes de tentar novamente.".into());
    }
    if !response.status().is_success() {
        return Err(format!(
            "O provedor público recusou a consulta (HTTP {}).",
            response.status().as_u16()
        ));
    }
    if response
        .content_length()
        .is_some_and(|n| n > 2 * 1024 * 1024)
    {
        return Err("Resposta pública excede o limite de tamanho.".into());
    }
    let mut bytes = vec![];
    response
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Falha ao ler o snapshot público.")?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("Resposta pública excede o limite de tamanho.".into());
    }
    Ok(bytes)
}
pub fn load(through: &str) -> Result<Snapshot, String> {
    let date = chrono::NaiveDate::parse_from_str(&(through.to_owned() + "-01"), "%Y-%m-%d")
        .map_err(|_| "Informe uma competência AAAA-MM válida.")?;
    if through.len() != 7
        || !through.is_ascii()
        || date < chrono::NaiveDate::from_ymd_opt(1994, 7, 1).unwrap()
        || date > chrono::Utc::now().date_naive()
    {
        return Err("Consulta de índices deve ter corte entre 1994-07 e o mês atual; dados futuros não são buscados nem inventados.".into());
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(25))
        .connect_timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .http1_only()
        .user_agent("previdenciar.io/public-economic-snapshot")
        .build()
        .map_err(|_| "Cliente seguro de índices indisponível.")?;
    let indices = get(
        &client,
        &format!("{BASE}tabela?competenciaInicial=1994-07&competenciaFinal={through}"),
    )?;
    let limits = get(
        &client,
        &format!("{BASE}limites?competenciaInicial=1994-07&competenciaFinal={through}"),
    )?;
    decode(through, &indices, &limits)
}
fn decode(through: &str, indices: &[u8], limits: &[u8]) -> Result<Snapshot, String> {
    let mut hash = Sha256::new();
    hash.update(&indices);
    hash.update(b"\n");
    hash.update(&limits);
    let digest = format!("{:x}", hash.finalize());
    let j: Value = serde_json::from_slice(&indices).map_err(|_| "Resposta de índices inválida.")?;
    let l: Value = serde_json::from_slice(&limits).map_err(|_| "Resposta de limites inválida.")?;
    if j["status"] != "ok" || l["status"] != "ok" {
        return Err("O provedor não confirmou os dados públicos.".into());
    }
    let mut rates = vec![];
    for (m, v) in j["result"].as_object().ok_or("Tabela pública inválida.")? {
        rates.push(Rate {
            month: m.clone(),
            inpc: decimal(&v["inpc"])?,
            igpdi: optional(&v["igpdi"])?,
            ipcr: optional(&v["ipcr"])?,
            inpc_inss: optional(&v["inpcInss"])?,
        });
    }
    let mut bound = vec![];
    for v in l["result"]
        .as_array()
        .ok_or("Tabela de limites inválida.")?
    {
        bound.push(Limits {
            month: v["competencia"]
                .as_str()
                .ok_or("Limite sem competência.")?
                .into(),
            minimum: decimal(&v["salarioMinimo"])?,
            ceiling: decimal(&v["tetoContribuicao"])?,
            minimum_source: v["fontePiso"].as_str().ok_or("Piso sem fonte.")?.into(),
            ceiling_source: v["fonteTeto"].as_str().ok_or("Teto sem fonte.")?.into(),
        });
    }
    let snapshot = Snapshot {
        source_url: format!("{BASE}tabela"),
        retrieved_at: chrono::Utc::now().to_rfc3339(),
        response_sha256: digest,
        through: through.into(),
        rates,
        limits: bound,
    };
    snapshot.validate()?;
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn indices() -> Vec<u8> {
        br#"{"status":"ok","result":{"2020-01":{"inpc":5e-5,"igpdi":null,"ipcr":null,"inpcInss":null}}}"#.to_vec()
    }
    fn limits() -> Vec<u8> {
        br#"{"status":"ok","result":[{"competencia":"2020-01","salarioMinimo":1000.00,"tetoContribuicao":"10000.00","fontePiso":"synthetic minimum","fonteTeto":"synthetic ceiling"}]}"#.to_vec()
    }
    #[test]
    fn public_request_negotiation_is_explicit_and_never_has_private_headers() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let worker = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut bytes = Vec::new();
            while !bytes.ends_with(b"\r\n\r\n") {
                let mut b = [0];
                stream.read_exact(&mut b).unwrap();
                bytes.push(b[0]);
                assert!(bytes.len() < 8192);
            }
            let headers = String::from_utf8(bytes).unwrap().to_lowercase();
            for expected in [
                "accept: application/json",
                "accept-language: pt-br,pt;q=0.9",
                "accept-encoding: identity",
                "connection: close",
            ] {
                assert!(headers.contains(expected));
            }
            for forbidden in ["authorization:", "x-api-key:", "cookie:", "cnis"] {
                assert!(!headers.contains(forbidden));
            }
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}")
                .unwrap();
        });
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap();
        assert_eq!(
            get(&client, &format!("http://{address}/synthetic-catalog")).unwrap(),
            b"{}"
        );
        worker.join().unwrap();
    }
    #[test]
    fn exact_scientific_rates_and_public_provenance_are_preserved() {
        let s = decode("2020-01", &indices(), &limits()).unwrap();
        assert_eq!(s.rates[0].inpc, "0.00005");
        assert_eq!(s.limits[0].minimum, "1000");
        assert_eq!(s.response_sha256.len(), 64);
        assert!(s.rates[0].igpdi.is_none());
    }
    #[test]
    fn missing_or_nonconfirmed_data_is_never_replaced_with_zero() {
        let bad = br#"{"status":"ok","result":{"2020-01":{}}}"#;
        assert!(decode("2020-01", bad, &limits()).is_err());
        assert!(decode("2020-01", br#"{"status":"error"}"#, &limits()).is_err());
    }
    #[test]
    fn provider_data_after_the_requested_cutoff_is_rejected() {
        assert!(decode("2019-12", &indices(), &limits()).is_err());
    }
}
