use crate::DomainError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// BRL in exact signed integer cents. No floats at API or arithmetic boundaries.
/// Intermediate products use i128; operations check overflow before returning i64.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Money(i64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoundingMode { HalfAwayFromZero, TowardZero }
impl Money {
    pub const ZERO: Self = Self(0);
    pub const fn from_cents(cents: i64) -> Self { Self(cents) }
    pub const fn cents(self) -> i64 { self.0 }
    pub fn checked_add(self, other: Self) -> Result<Self, DomainError> {
        self.0.checked_add(other.0).map(Self).ok_or(DomainError::Overflow)
    }
    pub fn checked_sub(self, other: Self) -> Result<Self, DomainError> {
        self.0.checked_sub(other.0).map(Self).ok_or(DomainError::Overflow)
    }
    pub fn ensure_nonnegative(self) -> Result<Self, DomainError> {
        if self.0 < 0 { Err(DomainError::NegativeAmount) } else { Ok(self) }
    }
    /// Canonical interchange: decimal point, no grouping/exponent, 0..2 decimals.
    pub fn parse_decimal(input: &str) -> Result<Self, DomainError> {
        parse_parts(input, '.')
    }
    /// Strict pt-BR: optional R$ prefix, comma decimals, verified '.' grouping.
    /// Ambiguous foreign format such as 1,234.56 is rejected, never guessed.
    pub fn parse_br(input: &str) -> Result<Self, DomainError> {
        let input = input.trim();
        let input = input.strip_prefix("R$").map(str::trim_start).unwrap_or(input);
        let (sign, unsigned) = split_sign(input)?;
        let mut parts = unsigned.split(',');
        let whole = parts.next().ok_or(DomainError::InvalidMoney)?;
        let fraction = parts.next();
        if parts.next().is_some() { return Err(DomainError::InvalidMoney); }
        // Bound grouping input before allocating helper vectors.
        if whole.len() > 26 { return Err(DomainError::Overflow); }
        let groups: Vec<&str> = whole.split('.').collect();
        if groups.len() > 1 && (groups[0].is_empty() || groups[0].len() > 3
            || groups[1..].iter().any(|g| g.len() != 3)) {
            return Err(DomainError::InvalidMoney);
        }
        if groups.iter().any(|g| g.is_empty() || !g.bytes().all(|b| b.is_ascii_digit())) {
            return Err(DomainError::InvalidMoney);
        }
        let mut canonical = String::new();
        if sign < 0 { canonical.push('-'); }
        canonical.push_str(&groups.concat());
        if let Some(f) = fraction { canonical.push('.'); canonical.push_str(f); }
        Self::parse_decimal(&canonical)
    }
    pub fn to_decimal(self) -> String {
        let absolute = i128::from(self.0).abs();
        format!("{}{}.{:02}", if self.0 < 0 { "-" } else { "" }, absolute / 100, absolute % 100)
    }
    pub fn to_br(self) -> String {
        let absolute = i128::from(self.0).abs();
        let whole = (absolute / 100).to_string();
        let mut grouped = String::new();
        for (i, c) in whole.chars().enumerate() {
            if i > 0 && (whole.len() - i) % 3 == 0 { grouped.push('.'); }
            grouped.push(c);
        }
        format!("{}{},{}", if self.0 < 0 { "-" } else { "" }, grouped, format_args!("{:02}", absolute % 100))
    }
    pub fn apply_percent(self, percent: u8, rounding: RoundingMode) -> Result<Self, DomainError> {
        let numerator = i128::from(self.0) * i128::from(percent);
        let rounded = round_ratio(numerator, 100, rounding);
        i64::try_from(rounded).map(Self).map_err(|_| DomainError::Overflow)
    }
}
fn split_sign(input: &str) -> Result<(i128, &str), DomainError> {
    let (sign, value) = if let Some(v) = input.strip_prefix('-') { (-1, v) } else { (1, input) };
    if value.is_empty() { return Err(DomainError::InvalidMoney); }
    Ok((sign, value))
}
fn parse_parts(input: &str, separator: char) -> Result<Money, DomainError> {
    let (sign, unsigned) = split_sign(input)?;
    let mut parts = unsigned.split(separator);
    let whole = parts.next().ok_or(DomainError::InvalidMoney)?;
    let fraction = parts.next();
    if whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) || parts.next().is_some() {
        return Err(DomainError::InvalidMoney);
    }
    if let Some(f) = fraction {
        if f.is_empty() || f.len() > 2 || !f.bytes().all(|b| b.is_ascii_digit()) {
            return Err(DomainError::InvalidMoney);
        }
    }
    if whole.len() > 17 { return Err(DomainError::Overflow); }
    let whole: i128 = whole.parse().map_err(|_| DomainError::Overflow)?;
    let minor: i128 = match fraction {
        Some(f) => f.parse::<i128>().map_err(|_| DomainError::InvalidMoney)? * if f.len() == 1 { 10 } else { 1 },
        None => 0,
    };
    let signed = (whole * 100 + minor) * sign;
    i64::try_from(signed).map(Money).map_err(|_| DomainError::Overflow)
}
/// Denominator must be positive. All call sites supply a checked positive value.
pub(crate) fn round_ratio(numerator: i128, denominator: i128, mode: RoundingMode) -> i128 {
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    if mode == RoundingMode::HalfAwayFromZero && remainder.abs() * 2 >= denominator {
        quotient + numerator.signum()
    } else { quotient }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MoneyWire { currency: String, amount: String }
impl Serialize for Money {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        MoneyWire { currency: "BRL".to_owned(), amount: self.to_decimal() }.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for Money {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = MoneyWire::deserialize(deserializer)?;
        if wire.currency != "BRL" { return Err(serde::de::Error::custom("Only explicit BRL is supported")); }
        Self::parse_decimal(&wire.amount).map_err(serde::de::Error::custom)
    }
}
