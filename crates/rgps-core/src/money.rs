use rust_decimal::prelude::*;
use rust_decimal::{Decimal, RoundingStrategy};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{0}")]
pub struct RgpsError(pub String);

pub fn erro<T>(msg: impl Into<String>) -> Result<T> {
    Err(RgpsError(msg.into()))
}

pub type Result<T> = std::result::Result<T, RgpsError>;

/// Converte string de dinheiro ("1234.56") em Decimal com validação.
pub fn parse_money(valor: &str, label: &str) -> Result<Decimal> {
    let limpo = valor.trim();
    if limpo.is_empty() {
        return erro(format!("{label} inválido: valor vazio."));
    }
    Decimal::from_str(limpo).map_err(|_| RgpsError(format!("{label} inválido: {valor}.")))
}

/// Arredonda para 2 casas com half-up monetário (mesma semântica do toCents do ATRIUM).
pub fn to_cents(valor: Decimal) -> Decimal {
    valor.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
}

/// String monetária com exatamente 2 casas e ponto decimal (contrato do ATRIUM).
pub fn money(valor: Decimal) -> String {
    let centavos = (valor * Decimal::from(100))
        .round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero)
        .mantissa(); // escala 0 → mantissa = centavos inteiros
    let negativo = centavos < 0;
    let abs = centavos.unsigned_abs();
    format!(
        "{}{}.{:02}",
        if negativo { "-" } else { "" },
        abs / 100,
        abs % 100
    )
}

/// Formata um Decimal com exatamente `dp` casas (índice com 6 casas etc.).
pub fn fixed(valor: Decimal, dp: u32) -> String {
    let scaled = valor.round_dp(dp);
    let texto = scaled.to_string();
    let (inteiro, frac) = match texto.split_once('.') {
        Some((i, f)) => (i.to_string(), f.to_string()),
        None => (texto, String::new()),
    };
    if dp == 0 {
        return inteiro;
    }
    format!("{}.{}", inteiro, format!("{:0<width$}", frac, width = dp as usize))
}

pub fn zero() -> Decimal {
    Decimal::ZERO
}

pub fn from_int(n: i64) -> Decimal {
    Decimal::from(n)
}

pub fn add(a: Decimal, b: Decimal) -> Decimal {
    a + b
}

pub fn mul(a: Decimal, b: Decimal) -> Decimal {
    a * b
}

pub fn div(a: Decimal, b: Decimal) -> Result<Decimal> {
    if b.is_zero() {
        return erro("Divisão por zero no cálculo.");
    }
    // Alta precisão intermediária (o ATRIUM usa 18 casas).
    Ok(a / b)
}

pub fn min_dec(a: Decimal, b: Decimal) -> Decimal {
    if a < b {
        a
    } else {
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::prelude::FromStr;

    #[test]
    fn arredonda_half_up() {
        assert_eq!(money(Decimal::from_str("462.9625").unwrap()), "462.96");
        assert_eq!(money(Decimal::from_str("1.005").unwrap()), "1.01");
        assert_eq!(money(Decimal::from_str("1600").unwrap()), "1600.00");
    }

    #[test]
    fn rejeita_valor_vazio() {
        assert!(parse_money("", "Salário").is_err());
        assert!(parse_money("abc", "Salário").is_err());
    }
}
