//! New independent domain implementation, not recovered WLanguage source.
//! No database, file IO, clock, process execution or network dependencies.
//! All behavioral calculators remain experimental; RMI/entitlement are unsupported.
//!
//! Synthetic reference sources (SHA-256, captured 2026-10-10):
//! reports/08-especificacao-reconstrucao-rust.md:
//! A2D8188B60281E1E2B5C67B47B900299CD43D164A70F4231DA61889D80E8FFCB
//! schemas/reconstruction-contracts.json:
//! 68EA94DAF6C949F0D98DE1FEDCA7B4283549ECAD72F29B37BBE528197C74CC16
//! evidence/runtime/behavioral-test-vectors.json:
//! 1A1814E85B86A1CB2D2E21D81C392AFDF532066FBCE35A193A1D3EFB3C9A570C
//! Paths above are relative to previus-reverse-engineering, not source imports.

#![forbid(unsafe_code)]

pub mod dates;
pub mod domain;
pub mod error;
pub mod facade;
pub mod finance;
pub mod insured_quality;
pub mod money;
pub mod outcome;
pub mod pension;
pub mod quality;

pub use dates::{CivilDate, YearMonth};
pub use error::DomainError;
pub use facade::run_tool;
pub use insured_quality::{insured_quality, QualityInput, QualityPeriod, QualitySummary};
pub use money::{Money, RoundingMode};
pub use outcome::{CalculationOutcome, OutcomeStatus, RuleReviewStatus, Warning};
