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
    let last_grace_month = input.last_contribution_date.year_month().checked_add_months(12)?;
    let latest_payment_date = last_grace_month.on_day(15)?;
    let quality_loss_date = latest_payment_date.next_day()?;
    Ok(CalculationOutcome::experimental(QualityOfInsuredOutput {
        grace_months: 12, last_grace_month, latest_payment_date, quality_loss_date,
        explanation: vec![
            "Art. 15, II, da Lei nº 8.213/1991: até 12 meses após a cessação das contribuições; encerramento no dia 15 do mês, a mesma convenção do relatório de qualidade.".to_owned(),
            "A prorrogação para até 24 meses exige mais de 120 contribuições e o histórico completo; este cálculo rápido não a aplica.".to_owned(),
            "Perda no dia seguinte ao prazo, sem correção de feriados/fins de semana.".to_owned(),
        ],
    }, "facultative_quality_v2", &["TEST-02", "TEST-03", "UI-030", "UI-033"],
    "Apenas dois vetores facultativos foram observados; outras datas são extrapolação experimental. Sem benefício atual e após parar de contribuir. Não determina direito a benefício."))
}
