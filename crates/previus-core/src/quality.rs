use crate::{CalculationOutcome, CivilDate, DomainError, YearMonth};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsuredCategory { Facultative, Employee, IndividualContributor, Other }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualityOfInsuredInput {
    pub stopped_contributing: bool,
    pub currently_receiving_benefit: bool,
    pub insured_category: InsuredCategory,
    /// Caller provides the date; there is no clock or implicit "today".
    pub last_contribution_date: CivilDate,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QualityOfInsuredOutput {
    pub grace_months: u32,
    pub last_grace_month: YearMonth,
    pub latest_payment_date: CivilDate,
    pub quality_loss_date: CivilDate,
    pub explanation: Vec<String>,
}
/// Experimental extrapolation of ONLY TEST-02/UI-030 and TEST-03/UI-033.
/// Calendar-month shift, day 15 of second subsequent month, next-day loss.
/// No weekend/holiday adjustment or legal interpretation is implemented.
pub fn calculate_facultative_quality(input: &QualityOfInsuredInput) -> Result<CalculationOutcome<QualityOfInsuredOutput>, DomainError> {
    if !input.stopped_contributing || input.currently_receiving_benefit
        || input.insured_category != InsuredCategory::Facultative {
        return Ok(CalculationOutcome::unsupported("facultative_quality_v1", vec![]));
    }
    let last_grace_month = input.last_contribution_date.year_month().checked_add_months(6)?;
    let latest_payment_date = last_grace_month.checked_add_months(2)?.on_day(15)?;
    let quality_loss_date = latest_payment_date.next_day()?;
    Ok(CalculationOutcome::experimental(QualityOfInsuredOutput {
        grace_months: 6, last_grace_month, latest_payment_date, quality_loss_date,
        explanation: vec![
            "Mês da contribuição informado explicitamente; somam-se seis meses civis ao mês, sem deslocar o dia.".to_owned(),
            "Prazo experimental: dia 15 do segundo mês após o último mês de graça.".to_owned(),
            "Perda experimental: dia seguinte ao prazo, sem correção de feriados/fins de semana.".to_owned(),
        ],
    }, "facultative_quality_v1", &["TEST-02", "TEST-03", "UI-030", "UI-033"],
    "Apenas dois vetores facultativos foram observados; outras datas são extrapolação experimental. Sem benefício atual e após parar de contribuir. Não determina direito a benefício."))
}
