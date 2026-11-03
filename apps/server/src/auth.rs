use crate::AppState;
use axum::http::HeaderMap;
use jsonwebtoken::{decode, decode_header, jwk::JwkSet, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use serde_json::Value;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct AuthUser {
    pub subject: String,
    pub email: String,
    /// Identidade confiável para decisão administrativa. Em produção só é
    /// preenchida por `email` + `email_verified=true`; em development/test,
    /// pelo header `x-dev-user` do ambiente explicitamente inseguro de teste.
    pub admin_email: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Claims {
    sub: String,
    email: Option<String>,
    username: Option<String>,
    token_use: Option<String>,
    email_verified: Option<Value>,
}

pub async fn authenticate(headers: &HeaderMap, state: &AppState) -> Result<AuthUser, String> {
    if state.config.auth_disabled {
        let email = headers
            .get("x-dev-user")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("dev@local");
        return Ok(AuthUser {
            subject: format!("dev:{email}"),
            email: email.to_owned(),
            // Header confiável somente porque `auth_disabled` só é aceito
            // em APP_ENV=development/test. Não é prova de Cognito verificado.
            admin_email: Some(email.to_owned()),
        });
    }

    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or_else(|| "Sessão ausente.".to_owned())?;

    let header = decode_header(token).map_err(|_| "Token inválido.".to_owned())?;
    let kid = header
        .kid
        .ok_or_else(|| "Token sem identificador de chave.".to_owned())?;
    let jwks = jwks(state).await?;
    let jwk = jwks
        .keys
        .iter()
        .find(|key| key.common.key_id.as_deref() == Some(kid.as_str()))
        .ok_or_else(|| "Chave de assinatura desconhecida.".to_owned())?;
    let key =
        DecodingKey::from_jwk(jwk).map_err(|_| "Chave de assinatura inválida.".to_owned())?;
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[state.config.cognito_client_id.as_str()]);
    validation.set_issuer(&[state.config.cognito_issuer().as_str()]);
    let claims = decode::<Claims>(token, &key, &validation)
        .map_err(|_| "Sessão inválida ou expirada.".to_owned())?
        .claims;
    if claims.token_use.as_deref() == Some("access") && claims.email.is_none() {
        // Access tokens are accepted, but the email is taken from the subject
        // until the profile endpoint or a first ID token supplies it.
    }
    let verified = email_verified(&claims);
    if claims.email.is_some() && claims.email_verified.is_some() && !verified {
        return Err("Confirme o e-mail antes de entrar.".to_owned());
    }
    let admin_email = verified_admin_email(&claims);
    let email = claims
        .email
        .clone()
        .or(claims.username.clone())
        .unwrap_or_else(|| claims.sub.clone());
    Ok(AuthUser {
        subject: claims.sub,
        email,
        admin_email,
    })
}

fn email_verified(claims: &Claims) -> bool {
    match claims.email_verified.as_ref() {
        Some(Value::Bool(value)) => *value,
        Some(Value::String(value)) => value == "true",
        _ => false,
    }
}

/// Só um e-mail explícito e verificado pode representar autorização
/// administrativa. `username` e `sub` continuam válidos para exibição/login
/// geral, mas nunca promovem alguém a admin.
fn verified_admin_email(claims: &Claims) -> Option<String> {
    if !email_verified(claims) {
        return None;
    }
    claims
        .email
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

async fn jwks(state: &AppState) -> Result<JwkSet, String> {
    if let Some((at, jwks)) = state.jwks.read().await.clone() {
        if at.elapsed() < Duration::from_secs(3600) {
            return Ok(jwks);
        }
    }
    let response = state
        .http
        .get(state.config.jwks_url())
        .send()
        .await
        .map_err(|_| "Não foi possível consultar as chaves do Cognito.".to_owned())?
        .error_for_status()
        .map_err(|_| "O Cognito recusou a consulta de chaves.".to_owned())?;
    let jwks = response
        .json::<JwkSet>()
        .await
        .map_err(|_| "Resposta de chaves do Cognito inválida.".to_owned())?;
    *state.jwks.write().await = Some((Instant::now(), jwks.clone()));
    Ok(jwks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn claims(
        email: Option<&str>,
        username: Option<&str>,
        email_verified: Option<Value>,
    ) -> Claims {
        Claims {
            sub: "sub-teste".to_owned(),
            email: email.map(str::to_owned),
            username: username.map(str::to_owned),
            token_use: Some("id".to_owned()),
            email_verified,
        }
    }

    #[test]
    fn admin_exige_email_explicito_e_verificado() {
        assert_eq!(
            verified_admin_email(&claims(
                Some("admin@teste.local"),
                None,
                Some(json!(true))
            )),
            Some("admin@teste.local".to_owned())
        );
        assert_eq!(
            verified_admin_email(&claims(
                Some("admin@teste.local"),
                None,
                Some(json!("true"))
            )),
            Some("admin@teste.local".to_owned())
        );
        assert_eq!(
            verified_admin_email(&claims(
                Some("admin@teste.local"),
                None,
                Some(json!(false))
            )),
            None
        );
        assert_eq!(
            verified_admin_email(&claims(Some("admin@teste.local"), None, None)),
            None
        );
    }

    #[test]
    fn username_ou_sub_nao_promovem_admin() {
        assert_eq!(
            verified_admin_email(&claims(
                None,
                Some("admin@teste.local"),
                Some(json!(true))
            )),
            None
        );
        assert_eq!(
            verified_admin_email(&claims(
                Some("   "),
                Some("admin@teste.local"),
                Some(json!(true))
            )),
            None
        );
    }

    #[test]
    fn lista_admin_preserva_normalizacao_existente() {
        let config = crate::config::Config::for_tests();
        assert!(config.is_admin("ADMIN@TESTE.LOCAL"));
        assert!(config.is_admin("admin@teste.local"));
        assert!(!config.is_admin("outro@teste.local"));
    }
}
