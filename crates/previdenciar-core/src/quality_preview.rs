use crate::{CalculationOutcome, CivilDate, DomainError, Warning};
use serde::{Deserialize, Serialize};


/// Inputs are deliberately limited to what the operator can declare. This
/// preview does not read CNIS, infer contribution counts or decide entitlement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualityPreviewInput {
    pub last_contribution_date: CivilDate,
    pub fact_date: CivilDate,
    /// More than 120 contributions without an interruption. The caller must
    /// have checked the contribution history.
    pub more_than_120: bool,
    /// Documented unemployment. The caller must have evidence.
    pub documented_unemployment: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualityPreviewOutput {
    pub last_contribution_date: CivilDate,
    pub base_grace_until: CivilDate,
    pub extended_grace_until: CivilDate,
    pub extension_months: u32,
    pub quality_maintained_at_fact: bool,
    pub fact_date: CivilDate,
    pub legal_basis: String,
}

pub fn quality_preview(
    input: &QualityPreviewInput,
) -> Result<CalculationOutcome<QualityPreviewOutput>, DomainError> {
    if input.last_contribution_date > input.fact_date {
        return Ok(CalculationOutcome::unsupported(
            "quality_preview_v1",
            vec![Warning::new(
                "last_contribution_after_fact",
                "A última contribuição é posterior à data do fato; corrija as datas e revise a origem.",
            )],
        ));
    }
    let extension_months = if input.more_than_120 { 12 } else { 0 }
        + if input.documented_unemployment { 12 } else { 0 };
    let total_months = 12u32
        .checked_add(extension_months)
        .ok_or(DomainError::Overflow)?;
    let grace_month = input
        .last_contribution_date
        .year_month()
        .checked_add_months(total_months)?;
    let grace_until = grace_month.on_day(15)?;
    let base_month = input
        .last_contribution_date
        .year_month()
        .checked_add_months(12)?;
    let base_grace_until = base_month.on_day(15)?;
    let mut warnings = vec![
        Warning::new(
            "declared_extensions",
            "As prorrogações por mais de 120 contribuições e desemprego comprovado foram declaradas pelo operador; nenhuma foi inferida.",
        ),
        Warning::new(
            "grace_day_convention",
            "O término usa o dia 15 do mês, a convenção do relatório de qualidade; não há ajuste de feriado ou fim de semana.",
        ),
        Warning::new(
            "legal_review_required",
            "A base legal das prorrogações e a contagem das contribuições exigem conferência jurídica e documental.",
        ),
    ];
    if extension_months == 24 {
        warnings.push(Warning::new(
            "extension_combination",
            "A combinação das duas prorrogações não foi confirmada pelo jurídico; o resultado é apenas uma hipótese declarada.",
        ));
    }
    let mut outcome = CalculationOutcome::experimental(
        QualityPreviewOutput {
            last_contribution_date: input.last_contribution_date,
            base_grace_until,
            extended_grace_until: grace_until,
            extension_months,
            quality_maintained_at_fact: input.fact_date <= grace_until,
            fact_date: input.fact_date,
            legal_basis: "Lei 8.213/1991, art. 15, II e § 1º, com prorrogações declaradas; revisão jurídica pendente.".into(),
        },
        "quality_preview_v1",
        &[],
        "Prévia declarada de período de graça; não lê CNIS e não decide direito a benefício.",
    );
    outcome.warnings.extend(warnings);
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn date(value: &str) -> CivilDate {
        CivilDate::from_str(value).unwrap()
    }

    #[test]
    fn base_and_declared_extensions_are_calculated_without_inference() {
        let out = quality_preview(&QualityPreviewInput {
            last_contribution_date: date("2024-01-10"),
            fact_date: date("2024-12-20"),
            more_than_120: false,
            documented_unemployment: false,
        })
        .unwrap();
        let data = out.data.unwrap();
        assert_eq!(data.base_grace_until.to_string(), "2025-01-15");
        assert!(data.quality_maintained_at_fact);
        assert_eq!(data.extension_months, 0);
    }

    #[test]
    fn both_declared_extensions_are_summed_but_flagged() {
        let out = quality_preview(&QualityPreviewInput {
            last_contribution_date: date("2024-01-10"),
            fact_date: date("2026-02-01"),
            more_than_120: true,
            documented_unemployment: true,
        })
        .unwrap();
        let data = out.data.unwrap();
        assert_eq!(data.extension_months, 24);
        assert_eq!(data.extended_grace_until.to_string(), "2027-01-15");
        assert!(out.warnings.iter().any(|warning| warning.code == "extension_combination"));
    }

    #[test]
    fn contribution_after_fact_is_refused() {
        let out = quality_preview(&QualityPreviewInput {
            last_contribution_date: date("2025-01-10"),
            fact_date: date("2024-01-10"),
            more_than_120: false,
            documented_unemployment: false,
        })
        .unwrap();
        assert!(out.data.is_none());
        assert_eq!(out.status, crate::OutcomeStatus::Unsupported);
    }
}


