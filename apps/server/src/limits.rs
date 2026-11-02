use crate::AppState;
use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::{request::Parts, HeaderMap};
use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

const MAX_WINDOW: Duration = Duration::from_secs(24 * 3600);
const DEFAULT_MAX_KEYS: usize = 100_000;

#[derive(Clone, Copy, Debug)]
pub struct RateWindow {
    pub max: usize,
    pub window: Duration,
    pub message: &'static str,
}

impl RateWindow {
    pub const fn new(max: usize, window: Duration, message: &'static str) -> Self {
        Self {
            max,
            window,
            message,
        }
    }
}

#[derive(Debug)]
pub struct RateLimitError {
    pub message: String,
    pub retry_after: Duration,
}

#[derive(Debug)]
struct Bucket {
    hits: VecDeque<Instant>,
    last: Instant,
}

pub struct RateLimiter {
    buckets: Mutex<HashMap<String, Bucket>>,
    max_keys: usize,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self::with_max_keys(DEFAULT_MAX_KEYS)
    }

    pub fn with_max_keys(max_keys: usize) -> Self {
        Self {
            buckets: Mutex::new(HashMap::new()),
            max_keys: max_keys.max(1),
        }
    }

    pub fn check(&self, entries: &[(String, RateWindow)]) -> Result<(), RateLimitError> {
        let now = Instant::now();
        let mut buckets = self.buckets.lock().unwrap_or_else(|error| error.into_inner());
        for (key, window) in entries {
            if let Some(bucket) = buckets.get_mut(key.as_str()) {
                while bucket
                    .hits
                    .front()
                    .is_some_and(|hit| now.duration_since(*hit) >= window.window)
                {
                    bucket.hits.pop_front();
                }
            }
        }
        for (key, window) in entries {
            let Some(bucket) = buckets.get(key.as_str()) else {
                continue;
            };
            if bucket.hits.len() >= window.max {
                let oldest = bucket.hits.front().copied().unwrap_or(now);
                let elapsed = now.duration_since(oldest);
                let retry_after = window
                    .window
                    .saturating_sub(elapsed)
                    .max(Duration::from_secs(1));
                return Err(RateLimitError {
                    message: window.message.to_owned(),
                    retry_after,
                });
            }
        }
        for (key, _window) in entries {
            if !buckets.contains_key(key.as_str()) && buckets.len() >= self.max_keys {
                self.evict(&mut buckets, now);
            }
            let bucket = buckets.entry(key.clone()).or_insert_with(|| Bucket {
                hits: VecDeque::new(),
                last: now,
            });
            bucket.hits.push_back(now);
            bucket.last = now;
        }
        Ok(())
    }

    pub fn tracked_keys(&self) -> usize {
        self.buckets
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .len()
    }

    fn evict(&self, buckets: &mut HashMap<String, Bucket>, now: Instant) {
        buckets.retain(|_, bucket| now.duration_since(bucket.last) < MAX_WINDOW);
        while buckets.len() >= self.max_keys {
            let Some(oldest) = buckets
                .iter()
                .min_by_key(|(_, bucket)| bucket.last)
                .map(|(key, _)| key.clone())
            else {
                break;
            };
            buckets.remove(&oldest);
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct ClientIp(pub String);

impl FromRequestParts<AppState> for ClientIp {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let peer = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|info| info.0);
        Ok(Self(resolve_client_ip(
            &parts.headers,
            peer,
            state.config.trust_proxy,
        )))
    }
}

pub fn resolve_client_ip(headers: &HeaderMap, peer: Option<SocketAddr>, trust_proxy: bool) -> String {
    if trust_proxy {
        if let Some(value) = forwarded_ip(headers) {
            return value;
        }
    }
    if let Some(addr) = peer {
        return addr.ip().to_string();
    }
    if let Some(value) = forwarded_ip(headers) {
        return value;
    }
    "desconhecido".to_owned()
}

fn forwarded_ip(headers: &HeaderMap) -> Option<String> {
    for name in ["cf-connecting-ip", "x-real-ip"] {
        if let Some(value) = headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(value.to_owned());
        }
    }
    let chain = headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())?;
    let first = chain.split(',').next().map(str::trim)?;
    if first.is_empty() {
        None
    } else {
        Some(first.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(max: usize, window: Duration, message: &'static str) -> RateWindow {
        RateWindow::new(max, window, message)
    }

    #[test]
    fn janela_deslizante_conta_e_explica_o_limite() {
        let limiter = RateLimiter::new();
        let entries = || vec![("login:127.0.0.1".to_owned(), window(3, Duration::from_secs(60), "Limite de login."))];
        for _ in 0..3 {
            limiter.check(&entries()).expect("dentro do limite");
        }
        let error = limiter.check(&entries()).expect_err("deve estourar o limite");
        assert_eq!(error.message, "Limite de login.");
        assert!(error.retry_after > Duration::ZERO);
        assert!(error.retry_after <= Duration::from_secs(60));
    }

    #[test]
    fn varias_janelas_sao_avaliadas_juntas_sem_registrar_quando_uma_falha() {
        let limiter = RateLimiter::new();
        let ip = window(2, Duration::from_secs(3600), "Limite por IP.");
        let email = window(5, Duration::from_secs(86_400), "Limite por e-mail.");
        let entries = || {
            vec![
                ("cadastro:ip:10.0.0.1".to_owned(), ip),
                ("cadastro:email:x@y".to_owned(), email),
            ]
        };
        limiter.check(&entries()).expect("1o cadastro");
        limiter.check(&entries()).expect("2o cadastro");
        let error = limiter.check(&entries()).expect_err("3o cadastro estoura o IP");
        assert_eq!(error.message, "Limite por IP.");
        limiter
            .check(&[("cadastro:email:x@y".to_owned(), email)])
            .expect("janela de e-mail ainda tem saldo");
    }

    #[test]
    fn chaves_diferentes_nao_interferem() {
        let limiter = RateLimiter::new();
        let rule = window(1, Duration::from_secs(60), "Limite.");
        limiter
            .check(&[("ai:a@b".to_owned(), rule)])
            .expect("primeira chamada");
        let error = limiter
            .check(&[("ai:a@b".to_owned(), rule)])
            .expect_err("segunda chamada estoura");
        assert_eq!(error.message, "Limite.");
        limiter
            .check(&[("ai:outro@b".to_owned(), rule)])
            .expect("outra chave continua livre");
    }

    #[test]
    fn mapa_de_chaves_fica_limitado() {
        let limiter = RateLimiter::with_max_keys(64);
        let rule = window(5, Duration::from_secs(3600), "Limite.");
        for index in 0..500 {
            limiter
                .check(&[(format!("ferramenta:{index}"), rule)])
                .expect("sem limite por chave");
        }
        assert!(limiter.tracked_keys() <= 64);
    }

    #[test]
    fn limite_por_e_mail_usa_janela_do_dia() {
        let limiter = RateLimiter::new();
        let rule = window(2, Duration::from_secs(86_400), "Limite diario.");
        for _ in 0..2 {
            limiter.check(&[("cadastro:email:z@z".to_owned(), rule)]).unwrap();
        }
        let error = limiter
            .check(&[("cadastro:email:z@z".to_owned(), rule)])
            .expect_err("terceiro cadastro no dia");
        assert_eq!(error.message, "Limite diario.");
        assert!(error.retry_after > Duration::from_secs(23 * 3600));
    }

    #[test]
    fn resolve_ip_usa_peer_quando_nao_confia_em_proxy() {
        let headers = HeaderMap::new();
        let peer = Some("203.0.113.9:44321".parse::<SocketAddr>().unwrap());
        assert_eq!(
            resolve_client_ip(&headers, peer, false),
            "203.0.113.9"
        );
    }

    #[test]
    fn resolve_ip_confia_em_proxy_quando_configurado() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "198.51.100.7, 10.0.0.1".parse().unwrap());
        let peer = Some("10.0.0.1:44321".parse::<SocketAddr>().unwrap());
        assert_eq!(resolve_client_ip(&headers, peer, true), "198.51.100.7");
        assert_eq!(resolve_client_ip(&headers, peer, false), "10.0.0.1");
    }
}
