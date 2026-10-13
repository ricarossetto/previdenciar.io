use crate::{CalculationOutcome, DomainError};
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};

/// Validated civil Gregorian date, no system clock/timezone dependencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CivilDate(NaiveDate);
impl CivilDate {
    pub fn new(year: i32, month: u32, day: u32) -> Result<Self, DomainError> {
        if !(1..=9999).contains(&year) { return Err(DomainError::InvalidDate); }
        NaiveDate::from_ymd_opt(year, month, day).map(Self).ok_or(DomainError::InvalidDate)
    }
    pub fn parse_br(value: &str) -> Result<Self, DomainError> {
        if value.len() != 10 || value.as_bytes()[2] != b'/' || value.as_bytes()[5] != b'/'
            || !value.bytes().enumerate().all(|(i,b)| i == 2 || i == 5 || b.is_ascii_digit()) {
            return Err(DomainError::InvalidDate);
        }
        Self::new(value[6..10].parse().map_err(|_| DomainError::InvalidDate)?,
            value[3..5].parse().map_err(|_| DomainError::InvalidDate)?,
            value[0..2].parse().map_err(|_| DomainError::InvalidDate)?)
    }
    pub fn year(self) -> i32 { self.0.year() }
    pub fn month(self) -> u32 { self.0.month() }
    pub fn day(self) -> u32 { self.0.day() }
    pub fn year_month(self) -> YearMonth { YearMonth { year: self.year(), month: self.month() } }
    pub fn to_br(self) -> String { format!("{:02}/{:02}/{:04}", self.day(), self.month(), self.year()) }
    pub fn next_day(self) -> Result<Self, DomainError> {
        let next = self.0.succ_opt().ok_or(DomainError::Overflow)?;
        Self::new(next.year(), next.month(), next.day()).map_err(|_| DomainError::Overflow)
    }
}
impl FromStr for CivilDate {
    type Err = DomainError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != 10 || value.as_bytes()[4] != b'-' || value.as_bytes()[7] != b'-'
            || !value.bytes().enumerate().all(|(i,b)| i == 4 || i == 7 || b.is_ascii_digit()) {
            return Err(DomainError::InvalidDate);
        }
        Self::new(value[0..4].parse().map_err(|_| DomainError::InvalidDate)?,
            value[5..7].parse().map_err(|_| DomainError::InvalidDate)?,
            value[8..10].parse().map_err(|_| DomainError::InvalidDate)?)
    }
}
impl fmt::Display for CivilDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{:04}-{:02}-{:02}", self.year(), self.month(), self.day()) }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct YearMonth { year: i32, month: u32 }
impl YearMonth {
    pub fn new(year: i32, month: u32) -> Result<Self, DomainError> {
        if !(1..=9999).contains(&year) || !(1..=12).contains(&month) { return Err(DomainError::InvalidYearMonth); }
        Ok(Self { year, month })
    }
    pub fn year(self) -> i32 { self.year }
    pub fn month(self) -> u32 { self.month }
    /// Shifts MONTH only: never guesses what to do with Feb 29/day 31.
    pub fn checked_add_months(self, months: u32) -> Result<Self, DomainError> {
        let ordinal = i64::from(self.year - 1) * 12 + i64::from(self.month - 1) + i64::from(months);
        let year = i32::try_from(ordinal / 12 + 1).map_err(|_| DomainError::Overflow)?;
        Self::new(year, (ordinal % 12 + 1) as u32).map_err(|_| DomainError::Overflow)
    }
    pub fn on_day(self, day: u32) -> Result<CivilDate, DomainError> { CivilDate::new(self.year, self.month, day) }
}
impl FromStr for YearMonth {
    type Err = DomainError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != 7 || value.as_bytes()[4] != b'-'
            || !value.bytes().enumerate().all(|(i,b)| i == 4 || b.is_ascii_digit()) {
            return Err(DomainError::InvalidYearMonth);
        }
        Self::new(value[0..4].parse().map_err(|_| DomainError::InvalidYearMonth)?,
            value[5..7].parse().map_err(|_| DomainError::InvalidYearMonth)?)
    }
}
impl fmt::Display for YearMonth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{:04}-{:02}", self.year, self.month) }
}
macro_rules! string_serde {
    ($type:ty) => {
        impl Serialize for $type {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> { s.serialize_str(&self.to_string()) }
        }
        impl<'de> Deserialize<'de> for $type {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let value = String::deserialize(d)?;
                value.parse().map_err(serde::de::Error::custom)
            }
        }
    };
}
string_serde!(CivilDate);
string_serde!(YearMonth);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InclusiveIntervalInput { pub start: CivilDate, pub end: CivilDate }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InclusiveIntervalOutput {
    pub calendar_days: u64,
    pub counting_policy: String,
    pub duration_text: String,
    pub total_days_text: String,
}
/// EXPLICIT inclusive operation, NOT a global rule for contributory/benefit periods.
/// Observed oracle: TEST-04/UI-047 same-day => 1. Other inputs are design extrapolation.
pub fn count_inclusive_calendar_days(input: &InclusiveIntervalInput) -> Result<CalculationOutcome<InclusiveIntervalOutput>, DomainError> {
    if input.end < input.start { return Err(DomainError::ReversedInterval); }
    let days = (input.end.0 - input.start.0).num_days() as u64 + 1;
    Ok(CalculationOutcome::experimental(InclusiveIntervalOutput {
        calendar_days: days, counting_policy: "both_endpoints_included".to_owned(),
        duration_text: format!("{} {}", days, if days == 1 { "dia" } else { "dias" }),
        total_days_text: format!("Total de dias corridos: {}", days),
    }, "inclusive_calendar_v1", &["TEST-04", "UI-047"],
    "Somente o caso de datas iguais foi observado. Inclusividade explicitamente escolhida nesta operação; não define tempo de contribuição."))
}
