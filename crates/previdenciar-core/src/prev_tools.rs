use crate::{CalculationOutcome, CivilDate, DomainError, Warning, YearMonth};
use serde::{Deserialize, Serialize};

fn outcome<T>(data: T, policy: &str, warning: &str) -> CalculationOutcome<T> {
    CalculationOutcome::experimental(
        data,
        policy,
        &[],
        warning,
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArrearsInput {
    pub monthly_benefit_centavos: i64,
    pub start_month: YearMonth,
    pub end_month: YearMonth,
    pub correction_percent: u32,
    pub interest_percent: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArrearsOutput {
    pub months: u32,
    pub nominal_centavos: i64,
    pub correction_centavos: i64,
    pub interest_centavos: i64,
    pub total_centavos: i64,
}

pub fn arrears(input: &ArrearsInput) -> Result<CalculationOutcome<ArrearsOutput>, DomainError> {
    if input.monthly_benefit_centavos <= 0 {
        return Err(DomainError::InvalidMoney);
    }
    if input.start_month > input.end_month {
        return Err(DomainError::ReversedInterval);
    }
    let start = i64::from(input.start_month.year() - 1) * 12 + i64::from(input.start_month.month() - 1);
    let end = i64::from(input.end_month.year() - 1) * 12 + i64::from(input.end_month.month() - 1);
    let months = u32::try_from(end - start + 1).map_err(|_| DomainError::Overflow)?;
    let nominal = i128::from(input.monthly_benefit_centavos) * i128::from(months);
    let correction = nominal * i128::from(input.correction_percent) / 100;
    let interest = nominal * i128::from(input.interest_percent) / 100;
    let total = nominal
        .checked_add(correction)
        .and_then(|value| value.checked_add(interest))
        .ok_or(DomainError::Overflow)?;
    let to_i64 = |value: i128| i64::try_from(value).map_err(|_| DomainError::Overflow);
    let mut outcome = outcome(
        ArrearsOutput {
            months,
            nominal_centavos: to_i64(nominal)?,
            correction_centavos: to_i64(correction)?,
            interest_centavos: to_i64(interest)?,
            total_centavos: to_i64(total)?,
        },
        "arrears_arithmetic_v1",
        "Soma declarada de parcelas, correção e juros; índices, termos iniciais, prescrição e consectários não foram decididos.",
    );
    outcome.warnings.extend([
        Warning::new(
            "declared_rates",
            "Correção e juros foram declarados pelo operador em percentual único; não são índices oficiais nem cálculo mês a mês.",
        ),
        Warning::new(
            "prescription_not_applied",
            "Prescrição, 13º, honorários e critério legal de atualização não foram aplicados.",
        ),
    ]);
    Ok(outcome)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PensionSplitInput {
    pub base_centavos: i64,
    pub dependents: u32,
    pub spouse_quota_percent: u32,
    pub duration_months: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PensionSplitOutput {
    pub per_dependent_centavos: i64,
    pub spouse_quota_centavos: i64,
    pub duration_months: u32,
}

pub fn pension_split(
    input: &PensionSplitInput,
) -> Result<CalculationOutcome<PensionSplitOutput>, DomainError> {
    if input.base_centavos <= 0 || input.dependents == 0 || input.dependents > 100 {
        return Err(DomainError::InvalidInput);
    }
    if input.spouse_quota_percent > 100 {
        return Err(DomainError::InvalidInput);
    }
    let per_dependent = i128::from(input.base_centavos) / i128::from(input.dependents);
    let spouse = i128::from(input.base_centavos) * i128::from(input.spouse_quota_percent) / 100;
    let mut outcome = outcome(
        PensionSplitOutput {
            per_dependent_centavos: i64::try_from(per_dependent).map_err(|_| DomainError::Overflow)?,
            spouse_quota_centavos: i64::try_from(spouse).map_err(|_| DomainError::Overflow)?,
            duration_months: input.duration_months,
        },
        "pension_split_v1",
        "Rateio aritmético por quantidade de dependentes e cota declarada; composição, duração e acumulação não foram decididas.",
    );
    outcome.warnings.push(Warning::new(
        "legal_pension_rules_required",
        "Duração por idade/contribuições, rateio legal e redutor de acumulação exigem regra jurídica confirmada.",
    ));
    Ok(outcome)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IncapacityInput {
    pub contributions: u32,
    pub required_carencia: u32,
    pub average_centavos: i64,
    pub coefficient_percent: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IncapacityOutput {
    pub carencia_met: bool,
    pub contribution_gap: u32,
    pub rmi_centavos: Option<i64>,
}

pub fn incapacity(
    input: &IncapacityInput,
) -> Result<CalculationOutcome<IncapacityOutput>, DomainError> {
    if input.average_centavos <= 0 || input.coefficient_percent > 100 {
        return Err(DomainError::InvalidInput);
    }
    let met = input.contributions >= input.required_carencia;
    let gap = input.required_carencia.saturating_sub(input.contributions);
    let rmi = if met {
        let value = i128::from(input.average_centavos) * i128::from(input.coefficient_percent) / 100;
        Some(i64::try_from(value).map_err(|_| DomainError::Overflow)?)
    } else {
        None
    };
    let mut outcome = outcome(
        IncapacityOutput {
            carencia_met: met,
            contribution_gap: gap,
            rmi_centavos: rmi,
        },
        "incapacity_arithmetic_v1",
        "Conferência aritmética de carência e aplicação de coeficiente declarado; doença, incapacidade, DIB e regra legal não foram decididas.",
    );
    outcome.warnings.extend([
        Warning::new(
            "carencia_arithmetic_only",
            "A carência foi comparada com o número declarado de contribuições; vínculos, competências e dispensa legal exigem revisão.",
        ),
        Warning::new(
            "coefficient_declared",
            "O coeficiente foi informado pelo operador e não foi escolhido pelo aplicativo.",
        ),
    ]);
    Ok(outcome)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecialInput {
    pub months: u32,
    pub conversion_percent: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpecialOutput {
    pub common_months: u32,
    pub common_years: u32,
    pub remaining_months: u32,
}

pub fn special_time(
    input: &SpecialInput,
) -> Result<CalculationOutcome<SpecialOutput>, DomainError> {
    if input.months == 0 || input.conversion_percent > 1000 {
        return Err(DomainError::InvalidInput);
    }
    let total = i128::from(input.months) * i128::from(input.conversion_percent) / 100;
    let total = u32::try_from(total).map_err(|_| DomainError::Overflow)?;
    let mut outcome = outcome(
        SpecialOutput {
            common_months: total,
            common_years: total / 12,
            remaining_months: total % 12,
        },
        "special_conversion_v1",
        "Conversão aritmética por fator declarado; agente nocivo, ruído, EPI, período e regra aplicável não foram decididos.",
    );
    outcome.warnings.push(Warning::new(
        "special_enquadramento_required",
        "O enquadramento especial e a escolha do fator dependem de PPP, laudo e regra jurídica confirmada.",
    ));
    Ok(outcome)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevisionDeadlineInput {
    pub first_payment: CivilDate,
    pub years: u32,
    pub months: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevisionDeadlineOutput {
    pub deadline: CivilDate,
    pub total_months: u32,
}

pub fn revision_deadline(
    input: &RevisionDeadlineInput,
) -> Result<CalculationOutcome<RevisionDeadlineOutput>, DomainError> {
    let total_months = input
        .years
        .checked_mul(12)
        .and_then(|value| value.checked_add(input.months))
        .ok_or(DomainError::Overflow)?;
    if total_months == 0 {
        return Err(DomainError::InvalidInput);
    }
    let deadline = input
        .first_payment
        .year_month()
        .checked_add_months(total_months)?
        .on_day(input.first_payment.day())?;
    let mut outcome = outcome(
        RevisionDeadlineOutput { deadline, total_months },
        "revision_deadline_v1",
        "Soma de prazo declarado à data do primeiro pagamento; termo inicial, suspensões e contagem legal não foram decididos.",
    );
    outcome.warnings.push(Warning::new(
        "deadline_rule_required",
        "O prazo decadencial e a contagem dependem de regra jurídica e conferência da data de início.",
    ));
    Ok(outcome)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanningInput {
    pub age_months: u32,
    pub contribution_months: u32,
    pub required_age_months: u32,
    pub required_contribution_months: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanningOutput {
    pub missing_age_months: u32,
    pub missing_contribution_months: u32,
    pub requirements_met: bool,
}

pub fn planning(
    input: &PlanningInput,
) -> Result<CalculationOutcome<PlanningOutput>, DomainError> {
    if input.age_months == 0
        || input.required_age_months == 0
        || input.required_contribution_months == 0
    {
        return Err(DomainError::InvalidInput);
    }
    let missing_age = input.required_age_months.saturating_sub(input.age_months);
    let missing_contrib = input
        .required_contribution_months
        .saturating_sub(input.contribution_months);
    let mut outcome = outcome(
        PlanningOutput {
            missing_age_months: missing_age,
            missing_contribution_months: missing_contrib,
            requirements_met: missing_age == 0 && missing_contrib == 0,
        },
        "planning_gap_v1",
        "Comparação com requisitos declarados; regra aplicável, transição e renda futura não foram escolhidas.",
    );
    outcome.warnings.push(Warning::new(
        "planning_rules_declared",
        "Idade e tempo exigidos foram informados pelo operador; nenhuma regra de transição foi presumida.",
    ));
    Ok(outcome)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TriageInput {
    pub client_name: String,
    pub needs: Vec<String>,
    pub questions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TriageOutput {
    pub title: String,
    pub questionnaire: Vec<String>,
}

pub fn triage(input: &TriageInput) -> Result<CalculationOutcome<TriageOutput>, DomainError> {
    if input.questions.is_empty() || input.questions.len() > 100 {
        return Err(DomainError::InvalidInput);
    }
    let questionnaire = input
        .questions
        .iter()
        .map(|question| question.trim().to_owned())
        .filter(|question| !question.is_empty())
        .collect::<Vec<_>>();
    if questionnaire.is_empty() {
        return Err(DomainError::InvalidInput);
    }
    let mut outcome = outcome(
        TriageOutput {
            title: format!("Triagem — {}", input.client_name.trim()),
            questionnaire,
        },
        "triage_local_v1",
        "Questionário gerado localmente; não há link público, envio automático ou coleta de documentos.",
    );
    outcome.warnings.extend([
        Warning::new(
            "triage_local_only",
            "O Previdenciar não possui backend de triagem; o questionário precisa ser exportado e respondido fora do aplicativo.",
        ),
        Warning::new(
            "needs_not_legal_advice",
            "As necessidades informadas são apenas um roteiro de coleta.",
        ),
    ]);
    let _ = input.needs;
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn arrears_is_exact_and_refuses_reversed_month() {
        let out = arrears(&ArrearsInput {
            monthly_benefit_centavos: 100_000,
            start_month: YearMonth::from_str("2024-01").unwrap(),
            end_month: YearMonth::from_str("2024-03").unwrap(),
            correction_percent: 10,
            interest_percent: 5,
        })
        .unwrap();
        let data = out.data.unwrap();
        assert_eq!(data.months, 3);
        assert_eq!(data.total_centavos, 345_000);
        assert!(arrears(&ArrearsInput {
            monthly_benefit_centavos: 100_000,
            start_month: YearMonth::from_str("2024-03").unwrap(),
            end_month: YearMonth::from_str("2024-01").unwrap(),
            correction_percent: 0,
            interest_percent: 0,
        }).is_err());
    }

    #[test]
    fn pension_split_and_special_conversion_are_arithmetic() {
        let pension = pension_split(&PensionSplitInput {
            base_centavos: 300_000,
            dependents: 3,
            spouse_quota_percent: 50,
            duration_months: 120,
        })
        .unwrap()
        .data
        .unwrap();
        assert_eq!(pension.per_dependent_centavos, 100_000);
        assert_eq!(pension.spouse_quota_centavos, 150_000);
        let special = special_time(&SpecialInput { months: 120, conversion_percent: 140 })
            .unwrap()
            .data
            .unwrap();
        assert_eq!(special.common_months, 168);
        assert_eq!(special.common_years, 14);
    }

    #[test]
    fn incapacity_and_planning_never_invent_missing_requirements() {
        let incapacity = incapacity(&IncapacityInput {
            contributions: 10,
            required_carencia: 12,
            average_centavos: 200_000,
            coefficient_percent: 80,
        })
        .unwrap()
        .data
        .unwrap();
        assert!(!incapacity.carencia_met);
        assert_eq!(incapacity.rmi_centavos, None);
        let planning = planning(&PlanningInput {
            age_months: 700,
            contribution_months: 200,
            required_age_months: 780,
            required_contribution_months: 240,
        })
        .unwrap()
        .data
        .unwrap();
        assert_eq!(planning.missing_age_months, 80);
        assert_eq!(planning.missing_contribution_months, 40);
        assert!(!planning.requirements_met);
    }

    #[test]
    fn revision_deadline_preserves_day_and_triage_requires_question() {
        let deadline = revision_deadline(&RevisionDeadlineInput {
            first_payment: CivilDate::from_str("2020-01-31").unwrap(),
            years: 5,
            months: 0,
        })
        .unwrap()
        .data
        .unwrap();
        assert_eq!(deadline.deadline.to_string(), "2025-01-31");
        assert!(triage(&TriageInput {
            client_name: "Pessoa".into(),
            needs: vec![],
            questions: vec![],
        }).is_err());
    }
}
