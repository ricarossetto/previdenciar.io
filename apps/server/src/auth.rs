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
        });
    }

    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or_else(|| "SessÃ£o ausente.".to_owned())?;

    let header = decode_header(token).map_err(|_| "Token invÃ¡lido.".to_owned())?;
    let kid = header.kid.ok_or_else(|| "Token sem identificador de chave.".to_owned())?;
    let jwks = jwks(state).await?;
    let jwk = jwks
        .keys
        .iter()
        .find(|key| key.common.key_id.as_deref() == Some(kid.as_str()))
        .ok_or_else(|| "Chave de assinatura desconhecida.".to_owned())?;
    let key = DecodingKey::from_jwk(jwk).map_err(|_| "Chave de assinatura invÃ¡lida.".to_owned())?;
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[state.config.cognito_client_id.as_str()]);
    validation.set_issuer(&[state.config.cognito_issuer().as_str()]);
    let claims = decode::<Claims>(token, &key, &validation)
        .map_err(|_| "SessÃ£o invÃ¡lida ou expirada.".to_owned())?
        .claims;
    if claims.token_use.as_deref() == Some("access") && claims.email.is_none() {
        // Access tokens are accepted, but the email is taken from the subject
        // until the profile endpoint or a first ID token supplies it.
    }
    let verified = match claims.email_verified.as_ref() {
        Some(Value::Bool(value)) => *value,
        Some(Value::String(value)) => value == "true",
        _ => false,
    };
    if claims.email.is_some() && !verified {
        return Err("Confirme o e-mail antes de entrar.".to_owned());
    }
    let email = claims
        .email
        .or(claims.username)
        .unwrap_or_else(|| claims.sub.clone());
    Ok(AuthUser {
        subject: claims.sub,
        email,
    })
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
        .map_err(|_| "NÃ£o foi possÃ­vel consultar as chaves do Cognito.".to_owned())?
        .error_for_status()
        .map_err(|_| "O Cognito recusou a consulta de chaves.".to_owned())?;
    let jwks = response
        .json::<JwkSet>()
        .await
        .map_err(|_| "Resposta de chaves do Cognito invÃ¡lida.".to_owned())?;
    *state.jwks.write().await = Some((Instant::now(), jwks.clone()));
    Ok(jwks)
}

