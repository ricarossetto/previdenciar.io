use crate::{money::round_ratio, CalculationOutcome, CivilDate, DomainError, Money, RoundingMode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PensionAccumulationInput {
    pub reference_date: CivilDate,
    /// Entries remain in their original order; the earliest wins an equal-value tie.
    pub incomes: Vec<Money>,
    /// Mandatory caller-provided base, NOT an official indicator/default.
    pub reference_base: Money,
    /// Explicit experimental choice, not recovered legacy rounding policy.
    pub rounding: RoundingMode,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccumulationTranche {
    pub ordinal: u8,
    pub basis_amount: Money,
    pub rate_percent: u8,
    pub received_amount: Money,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IncomeOutcome {
    pub original_index: usize,
    pub original_amount: Money,
    pub integral: bool,
    pub received_amount: Money,
    /// Display only. Never multiply original income by this rounded percentage.
    /// None for 0/0 instead of an invented percentage.
    pub received_percentage: Option<String>,
    pub tranches: Vec<AccumulationTranche>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PensionAccumulationOutput {
    pub reference_date: CivilDate,
    pub reference_base: Money,
    pub integral_income_index: usize,
    pub integral_income: Money,
    pub incomes: Vec<IncomeOutcome>,
    pub reduced_total: Money,
    /// Arithmetic derived total; not an independently verified legacy UI field.
    pub combined_total: Money,
    pub rounding: RoundingMode,
    pub explanation: Vec<String>,
}
/// New policy based on bounded UI-074 observation: max integral, others tiered.
/// Cent quantization is explicit at EACH tranche and at percentage display.
/// This is NOT a general RMI, entitlement, legal or official-index calculator.
pub fn calculate_pension_accumulation(input: &PensionAccumulationInput) -> Result<CalculationOutcome<PensionAccumulationOutput>, DomainError> {
    if input.incomes.is_empty() { return Err(DomainError::EmptyIncomes); }
    if input.reference_base.cents() <= 0 { return Err(DomainError::InvalidReferenceBase); }
    for income in &input.incomes { income.ensure_nonnegative()?; }
    let mut highest = 0;
    for i in 1..input.incomes.len() {
        if input.incomes[i] > input.incomes[highest] { highest = i; }
    }
    let mut outcomes = Vec::with_capacity(input.incomes.len());
    let mut reduced_total = Money::ZERO;
    for (index, &original) in input.incomes.iter().enumerate() {
        let integral = index == highest;
        let (received, tranches) = if integral { (original, vec![]) }
            else { reduce_income(original, input.reference_base, input.rounding)? };
        if !integral { reduced_total = reduced_total.checked_add(received)?; }
        outcomes.push(IncomeOutcome { original_index: index, original_amount: original, integral,
            received_amount: received,
            received_percentage: display_percentage(received, original, input.rounding), tranches });
    }
    let integral_income = input.incomes[highest];
    let combined_total = integral_income.checked_add(reduced_total)?;
    Ok(CalculationOutcome::experimental(PensionAccumulationOutput {
        reference_date: input.reference_date, reference_base: input.reference_base,
        integral_income_index: highest, integral_income, incomes: outcomes, reduced_total,
        combined_total, rounding: input.rounding,
        explanation: vec![
            "Maior renda integral; empate resolvido pela primeira posição, política nova e não observada.".to_owned(),
            "Outras rendas: quatro faixas limitadas à base informada, com 100%, 60%, 40%, 20%; excedente com 10%.".to_owned(),
            "Cada faixa é arredondada em centavos conforme modo explícito; percentuais são somente apresentação.".to_owned(),
        ],
    }, "accumulation_cents_v1", &["TEST-05", "UI-074"],
    "Um único cenário foi observado. Base é entrada explícita não validada oficialmente; outras combinações, desempate e arredondamento são políticas experimentais. Não é validação normativa."))
}
fn reduce_income(original: Money, base: Money, rounding: RoundingMode) -> Result<(Money, Vec<AccumulationTranche>), DomainError> {
    let mut remaining = original;
    let mut total = Money::ZERO;
    let mut tranches = Vec::with_capacity(5);
    for (index, percent) in [100_u8, 60, 40, 20, 10].into_iter().enumerate() {
        let basis = if index == 4 { remaining } else { remaining.min(base) };
        remaining = remaining.checked_sub(basis)?;
        let received = basis.apply_percent(percent, rounding)?;
        total = total.checked_add(received)?;
        tranches.push(AccumulationTranche { ordinal: index as u8 + 1, basis_amount: basis,
            rate_percent: percent, received_amount: received });
    }
    Ok((total, tranches))
}
fn display_percentage(received: Money, original: Money, rounding: RoundingMode) -> Option<String> {
    if original == Money::ZERO { return None; }
    let hundredths = round_ratio(i128::from(received.cents()) * 10_000,
        i128::from(original.cents()), rounding);
    Some(format!("{}.{:02}", hundredths / 100, hundredths % 100))
}
