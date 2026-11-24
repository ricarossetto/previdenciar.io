//! Formatação brasileira para os relatórios: moeda, números, datas e períodos.
use rust_decimal::Decimal;
use std::str::FromStr;

/// Converte "1234.56" ou 1234.56 em "1.234,56".
pub fn br_number(value: &str) -> String {
    let raw = value.trim();
    let negative = raw.starts_with('-');
    let digits: String = raw.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect();
    let (int_part, frac_part) = match digits.split_once('.') {
        Some((i, f)) => (i.to_string(), f.chars().take(2).collect::<String>()),
        None => (digits.clone(), String::new()),
    };
    let mut int_out = String::new();
    for (i, c) in int_part.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            int_out.push('.');
        }
        int_out.push(c);
    }
    let int_out: String = int_out.chars().rev().collect();
    let int_out = if int_out.is_empty() { "0".to_string() } else { int_out };
    let frac = match frac_part.len() {
        0 => "00".to_string(),
        1 => format!("{frac_part}0"),
        _ => frac_part,
    };
    let sign = if negative { "-" } else { "" };
    format!("{sign}{int_out},{frac}")
}

/// "1234.56" -> "R$ 1.234,56".
pub fn brl(value: &str) -> String {
    format!("R$ {}", br_number(value))
}

/// Decimal -> "R$ 1.234,56".
pub fn brl_decimal(value: Decimal) -> String {
    brl(&value.round_dp(2).to_string())
}

/// "2026-05-22" -> "22/05/2026". Datas inválidas passam inalteradas.
pub fn br_date(iso: &str) -> String {
    let parts: Vec<&str> = iso.split('-').collect();
    if parts.len() == 3 && parts[0].len() == 4 {
        format!("{}/{}/{}", parts[2], parts[1], parts[0])
    } else {
        iso.to_string()
    }
}

/// "2026-05" -> "05/2026".
pub fn br_month(iso: &str) -> String {
    let parts: Vec<&str> = iso.split('-').collect();
    if parts.len() >= 2 {
        format!("{}/{}", parts[1], parts[0])
    } else {
        iso.to_string()
    }
}

/// "22 anos, 1 mês e 4 dias" a partir de dias 30/360.
pub fn period_from_days_360(days: i64) -> String {
    let years = days / 360;
    let months = (days % 360) / 30;
    let rem = days % 30;
    period(years, months, rem)
}

/// "22 anos, 1 mês e 4 dias" a partir de meses (idade).
pub fn period_from_months(months: i64) -> String {
    period(months / 12, months % 12, 0)
}

fn plural(n: i64, singular: &str, plural: &str) -> String {
    if n == 1 {
        format!("{n} {singular}")
    } else {
        format!("{n} {plural}")
    }
}

/// Junta anos/meses/dias omitindo zeros e usando " e " antes do último item.
pub fn period(years: i64, months: i64, days: i64) -> String {
    let mut parts: Vec<String> = Vec::new();
    if years > 0 {
        parts.push(plural(years, "ano", "anos"));
    }
    if months > 0 {
        parts.push(plural(months, "mês", "meses"));
    }
    if days > 0 || parts.is_empty() {
        parts.push(plural(days, "dia", "dias"));
    }
    match parts.len() {
        1 => parts[0].clone(),
        _ => {
            let last = parts.pop().unwrap();
            format!("{} e {}", parts.join(", "), last)
        }
    }
}

/// Lê um decimal de string JSON ou número, tolerante.
pub fn decimal(value: &serde_json::Value) -> Option<Decimal> {
    match value {
        serde_json::Value::String(s) => Decimal::from_str(s.trim()).ok(),
        serde_json::Value::Number(n) => Decimal::from_str(&n.to_string()).ok(),
        _ => None,
    }
}
