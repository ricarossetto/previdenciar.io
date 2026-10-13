//! Parameter-driven financial mathematics, NOT a tax/benefit eligibility engine.
//! All inputs fail closed; no official tables, dates, rates or legal presumptions.
//! Money accepts a canonical decimal string or the existing BRL Money object.
//! See tests/finance-schema.json for the stable JSON integration contract/examples.
//! Corrections/interest use checked integer cents and exact eight-place decimals.
//! Annuity/exponential formulas explicitly disclose f64 approximation and cent quantization.
use crate::{CivilDate, Money, RoundingMode, YearMonth};
use serde::{de::DeserializeOwned, Deserialize, Deserializer};
use serde_json::{json, Value};
use std::collections::BTreeSet;

type ResultF<T> = Result<T, String>;
const SCALE: i128 = 100_000_000;
const PERCENT: i128 = SCALE * 100;
const MAX_CENTS: i64 = 9_000_000_000_000_000;
const MAX_PERIODS: u32 = 1200;
const MAX_ROWS: usize = 10_000;
const MAX_EVENTS: usize = 12_000;
fn fail(message: &str) -> String { message.to_owned() }
fn decode<T: DeserializeOwned>(input: Value) -> ResultF<T> {
    serde_json::from_value(input).map_err(|_| fail("Entrada financeira inválida, incompleta ou com campos desconhecidos."))
}
fn check_size(value: &Value, depth: u32, budget: &mut usize) -> ResultF<()> {
    if depth > 16 || *budget == 0 { return Err(fail("Limite estrutural de entrada excedido.")); }
    *budget -= 1;
    match value {
        Value::String(s) if s.len() > 4096 => return Err(fail("Texto de entrada excede o limite.")),
        Value::Array(a) => {
            if a.len() > MAX_ROWS { return Err(fail("Quantidade de entradas excede o limite.")); }
            for v in a { check_size(v, depth + 1, budget)?; }
        },
        Value::Object(o) => for (k, v) in o {
            if k.len() > 128 { return Err(fail("Nome de campo excede o limite.")); }
            check_size(v, depth + 1, budget)?;
        },
        _ => (),
    }
    Ok(())
}
#[derive(Clone, Copy, Debug)]
struct Decimal(i128);
impl<'de> Deserialize<'de> for Decimal {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        parse_decimal(&s).map(Self).map_err(serde::de::Error::custom)
    }
}
fn parse_decimal(s: &str) -> ResultF<i128> {
    if s.is_empty() || s.len() > 24 { return Err(fail("Decimal fora dos limites.")); }
    let (sign, s) = match s.strip_prefix('-') { Some(v) => (-1, v), None => (1, s) };
    let mut pieces = s.split('.');
    let whole = pieces.next().ok_or_else(|| fail("Decimal inválido."))?;
    let frac = pieces.next();
    if whole.is_empty() || !whole.bytes().all(|c| c.is_ascii_digit()) || pieces.next().is_some()
        || frac.is_some_and(|f| f.is_empty() || f.len() > 8 || !f.bytes().all(|c| c.is_ascii_digit())) {
        return Err(fail("Decimal deve usar ponto e até oito casas, sem expoente."));
    }
    let whole: i128 = whole.parse().map_err(|_| fail("Decimal fora dos limites."))?;
    if whole > 1_000_000 { return Err(fail("Decimal fora dos limites.")); }
    let f = match frac {
        Some(f) => f.parse::<i128>().map_err(|_| fail("Decimal inválido."))? * 10_i128.pow(8 - f.len() as u32),
        None => 0,
    };
    let value = (whole * SCALE + f) * sign;
    if value.abs() > 1_000_000 * SCALE { return Err(fail("Decimal fora dos limites.")); }
    Ok(value)
}
fn decimal_string(value: i128) -> String {
    let a = value.abs();
    format!("{}{}.{:08}", if value < 0 { "-" } else { "" }, a / SCALE, a % SCALE)
}
#[derive(Deserialize)]
#[serde(untagged)]
enum Amount { Decimal(String), Existing(Money) }
impl Amount {
    fn cents(&self) -> ResultF<i64> {
        let money = match self {
            Self::Decimal(s) => Money::parse_decimal(s).map_err(|_| fail("Valor monetário inválido."))?,
            Self::Existing(m) => *m,
        };
        bounded(i128::from(money.cents()))
    }
    fn nonnegative(&self) -> ResultF<i64> {
        let v = self.cents()?;
        if v < 0 { Err(fail("Valor monetário deve ser não negativo.")) } else { Ok(v) }
    }
}
fn money(cents: i64) -> Money { Money::from_cents(cents) }
fn bounded(value: i128) -> ResultF<i64> {
    if !(-i128::from(MAX_CENTS)..=i128::from(MAX_CENTS)).contains(&value) {
        Err(fail("Overflow ou limite monetário excedido."))
    } else { Ok(value as i64) }
}
fn add(a: i64, b: i64) -> ResultF<i64> { bounded(i128::from(a) + i128::from(b)) }
fn sub(a: i64, b: i64) -> ResultF<i64> { bounded(i128::from(a) - i128::from(b)) }
fn ratio(numerator: i128, denominator: i128, mode: RoundingMode) -> ResultF<i64> {
    if denominator <= 0 { return Err(fail("Denominador deve ser positivo.")); }
    let q = numerator / denominator;
    let r = numerator % denominator;
    let rounded = if mode == RoundingMode::HalfAwayFromZero && r.abs() * 2 >= denominator {
        q.checked_add(numerator.signum()).ok_or_else(|| fail("Overflow de arredondamento."))?
    } else { q };
    bounded(rounded)
}
fn multiply(value: i64, n: i128, d: i128, mode: RoundingMode) -> ResultF<i64> {
    ratio(i128::from(value).checked_mul(n).ok_or_else(|| fail("Overflow de produto."))?, d, mode)
}
fn period_count(periods: u32) -> ResultF<()> {
    if periods > MAX_PERIODS { Err(fail("Máximo de 1200 períodos.")) } else { Ok(()) }
}
fn positive_period_count(periods: u32) -> ResultF<()> {
    period_count(periods)?;
    if periods == 0 { Err(fail("Esta fórmula exige ao menos um período.")) } else { Ok(()) }
}
fn source(provenance: &str) -> ResultF<()> {
    if provenance.trim().is_empty() || provenance.len() > 512 || provenance.chars().any(char::is_control) {
        Err(fail("Procedência dos parâmetros é obrigatória, sem controles, até 512 bytes."))
    } else { Ok(()) }
}
fn correction_rate(rate: Decimal) -> ResultF<()> {
    if rate.0 <= -PERCENT { Err(fail("Taxa de correção deve ser maior que -100%.")) } else { Ok(()) }
}
fn interest_rate(rate: Decimal) -> ResultF<()> {
    if rate.0 < 0 { Err(fail("Juros não podem ser negativos nesta operação.")) } else { Ok(()) }
}
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum InterestMode { Simple, Compound }
fn interest(principal: i64, rate: Decimal, periods: u32, mode: InterestMode, rounding: RoundingMode) -> ResultF<i64> {
    period_count(periods)?;
    interest_rate(rate)?;
    match mode {
        InterestMode::Simple => multiply(principal, rate.0 * i128::from(periods), PERCENT, rounding),
        InterestMode::Compound => {
            let mut balance = principal;
            for _ in 0..periods { balance = add(balance, multiply(balance, rate.0, PERCENT, rounding)?)?; }
            sub(balance, principal)
        },
    }
}
fn apply_rates(mut cents: i64, rates: &[Decimal], rounding: RoundingMode) -> ResultF<(i64, Vec<Value>)> {
    let mut timeline = Vec::with_capacity(rates.len());
    for (i, rate) in rates.iter().enumerate() {
        correction_rate(*rate)?;
        let before = cents;
        cents = multiply(cents, PERCENT + rate.0, PERCENT, rounding)?;
        timeline.push(json!({"period": i + 1, "rate_percent": decimal_string(rate.0),
            "before": money(before), "correction": money(sub(cents, before)?), "after": money(cents)}));
    }
    Ok((cents, timeline))
}
fn envelope(tool: &str, provenance: &str, rounding: RoundingMode, data: Value, extra: &[&str], approximate: bool) -> Value {
    let mut assumptions = vec![
        "Somente matemática com parâmetros fornecidos pelo operador; nenhuma tabela oficial embutida.",
        "Não comprova direito, elegibilidade, obrigação tributária ou validação normativa.",
        "Valores monetários canônicos com ponto decimal; saída em centavos; sem conversão histórica automática.",
    ];
    assumptions.extend_from_slice(extra);
    if approximate { assumptions.push("Fórmula exponencial/anuidade usa f64 finito e quantização explícita em centavos; aproximação numérica experimental."); }
    json!({"status":"experimental", "rules_verified":false, "rule_review_status":"missing_rules",
        "policy_version":"operator_financial_math_v1", "tool":tool, "reference_tables_source":"operator_input",
        "provenance":provenance, "rounding":rounding,
        "rounding_policy":{"monetary_scale":"integer_cents", "rate_decimal_places":8,
            "correction":"each_supplied_period", "compound_interest":"each_period", "simple_interest":"once_at_end",
            "tax":"once_per_row", "computed_rate":"eight_decimal_places_using_declared_rounding", "formula_numeric_backend":if approximate { "f64_experimental" } else { "checked_i128_ratios" }},
        "assumptions":assumptions, "data":data,
        "warnings":[{"code":"operator_parameters_only","message":"Parâmetros declarados pelo operador; regras e tabelas legais não validadas."}],
        "evidence_refs":[]})
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpdateInput {
    principal_money: Amount,
    monthly_correction_percent: Vec<Decimal>,
    interest_percent_per_month: Decimal,
    months: u32,
    interest_mode: InterestMode,
    fees_percent: Decimal,
    fees_fixed_amount: Amount,
    rounding: RoundingMode,
    provenance: String,
}
fn update(input: UpdateInput) -> ResultF<Value> {
    source(&input.provenance)?;
    period_count(input.months)?;
    if input.monthly_correction_percent.len() != input.months as usize {
        return Err(fail("Informe exatamente uma correção percentual por mês declarado."));
    }
    interest_rate(input.fees_percent)?;
    let principal = input.principal_money.nonnegative()?;
    let fixed = input.fees_fixed_amount.nonnegative()?;
    let (corrected, timeline) = apply_rates(principal, &input.monthly_correction_percent, input.rounding)?;
    let juros = interest(corrected, input.interest_percent_per_month, input.months, input.interest_mode, input.rounding)?;
    let fees_base = add(corrected, juros)?;
    let percentage_fees = multiply(fees_base, input.fees_percent.0, PERCENT, input.rounding)?;
    let fees = add(percentage_fees, fixed)?;
    Ok(envelope("update_values", &input.provenance, input.rounding,
        json!({"principal":money(principal), "principal_corrected":money(corrected),
            "correction":money(sub(corrected,principal)?), "juros":money(juros),
            "fees_base":money(fees_base), "fees_percentage_amount":money(percentage_fees),
            "fees_fixed_amount":money(fixed), "honorarios":money(fees),
            "total":money(add(fees_base,fees)?), "timeline":timeline}),
        &["Correção sucessiva por mês, arredondada em cada mês. Juros incidem sobre o principal corrigido, sem capitalizar honorários.",
          "Honorários percentuais sobre principal corrigido mais juros; valor fixo somado depois. Convenção matemática, não critério judicial."], false))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TaxRow { month: YearMonth, taxable_amount: Amount, withheld: Amount }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Bracket { lower: Amount, upper: Option<Amount>, rate: Decimal, deduction: Amount }
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TaxModel { RateMinusDeduction, Marginal }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TaxInput {
    rows: Vec<TaxRow>, tax_brackets: Vec<Bracket>, schedule_model: TaxModel,
    rounding: RoundingMode, provenance: String,
}
struct CheckedBracket { lower: i64, upper: Option<i64>, rate: i128, deduction: i64 }
fn brackets(input: &[Bracket], model: TaxModel) -> ResultF<Vec<CheckedBracket>> {
    if input.is_empty() || input.len() > 100 { return Err(fail("Tabela exige entre 1 e 100 faixas.")); }
    let mut checked: Vec<CheckedBracket> = Vec::with_capacity(input.len());
    for (i, b) in input.iter().enumerate() {
        let lower = b.lower.nonnegative()?;
        let upper = b.upper.as_ref().map(Amount::nonnegative).transpose()?;
        let deduction = b.deduction.nonnegative()?;
        if b.rate.0 < 0 || b.rate.0 > PERCENT || upper.is_some_and(|u| u <= lower)
            || (i == 0 && lower != 0) || (i + 1 == input.len()) != upper.is_none() {
            return Err(fail("Faixas devem cobrir [0,infinito), em ordem, com limite final aberto e taxas de 0 a 100%."));
        }
        if let Some(previous) = checked.last() {
            if previous.upper != Some(lower) || b.rate.0 < previous.rate {
                return Err(fail("Faixas devem ser contíguas, sem sobreposição e com taxas não decrescentes."));
            }
            if matches!(model, TaxModel::RateMinusDeduction) {
                let previous_tax = i128::from(lower) * previous.rate - i128::from(previous.deduction) * PERCENT;
                let new_tax = i128::from(lower) * b.rate.0 - i128::from(deduction) * PERCENT;
                if new_tax < previous_tax { return Err(fail("Dedução produz queda de imposto entre faixas.")); }
            }
        }
        if matches!(model, TaxModel::Marginal) && deduction != 0 {
            return Err(fail("Modelo marginal exige deduções zero; dedução não será ignorada."));
        }
        if matches!(model, TaxModel::RateMinusDeduction)
            && i128::from(lower) * b.rate.0 < i128::from(deduction) * PERCENT {
            return Err(fail("Dedução produz imposto negativo no início da faixa."));
        }
        checked.push(CheckedBracket { lower, upper, rate:b.rate.0, deduction });
    }
    Ok(checked)
}
fn tax_for(amount: i64, bs: &[CheckedBracket], model: TaxModel, rounding: RoundingMode) -> ResultF<i64> {
    match model {
        TaxModel::RateMinusDeduction => {
            let b = bs.iter().find(|b| amount >= b.lower && b.upper.is_none_or(|u| amount < u))
                .ok_or_else(|| fail("Valor sem faixa tributária."))?;
            ratio(i128::from(amount) * b.rate - i128::from(b.deduction) * PERCENT, PERCENT, rounding)
        },
        TaxModel::Marginal => {
            let mut numerator = 0_i128;
            for b in bs {
                if amount <= b.lower { break; }
                let slice = amount.min(b.upper.unwrap_or(amount)) - b.lower;
                numerator = numerator.checked_add(i128::from(slice) * b.rate).ok_or_else(|| fail("Overflow tributário."))?;
            }
            ratio(numerator, PERCENT, rounding)
        },
    }
}
fn tax(input: TaxInput) -> ResultF<Value> {
    source(&input.provenance)?;
    if input.rows.is_empty() { return Err(fail("Informe ao menos uma competência tributária.")); }
    let bs = brackets(&input.tax_brackets, input.schedule_model)?;
    let mut seen = BTreeSet::new();
    let mut rows = Vec::with_capacity(input.rows.len());
    let (mut total_withheld, mut total_tax, mut total_refund, mut total_due) = (0,0,0,0);
    for row in &input.rows {
        if !seen.insert(row.month) { return Err(fail("Competência duplicada; agregue a base explicitamente antes do cálculo.")); }
        let taxable = row.taxable_amount.nonnegative()?;
        let withheld = row.withheld.nonnegative()?;
        let calculated = tax_for(taxable, &bs, input.schedule_model, input.rounding)?;
        let balance = sub(withheld,calculated)?;
        total_withheld = add(total_withheld,withheld)?;
        total_tax = add(total_tax,calculated)?;
        total_refund = add(total_refund,balance.max(0))?;
        total_due = add(total_due,(-balance).max(0))?;
        rows.push(json!({"month":row.month, "taxable_amount":money(taxable), "withheld":money(withheld),
            "calculated_tax":money(calculated), "refund_minus_due":money(balance),
            "balance_kind":balance_kind(balance), "refund":money(balance.max(0)), "additional_due":money((-balance).max(0))}));
    }
    let balance = sub(total_withheld,total_tax)?;
    Ok(envelope("irrf_refund", &input.provenance, input.rounding,
        json!({"rows":rows, "total_withheld":money(total_withheld), "total_calculated_tax":money(total_tax),
            "total_refund_before_offset":money(total_refund), "total_due_before_offset":money(total_due),
            "refund_minus_due":money(balance), "balance_kind":balance_kind(balance),
            "net_refund":money(balance.max(0)), "net_additional_due":money((-balance).max(0))}),
        &["Faixas em intervalos [inferior,superior); última faixa sem teto. Tabela informada aplica-se a todas as competências deste pedido.",
          "Base tributável já deve conter ajustes declarados pelo operador. Não calcula isenção, RRA, dependentes ou enquadramento legal.",
          "Saldo positivo indica restituição matemática; negativo indica complemento matemático. Compensação entre meses apenas informativa."], false))
}
fn balance_kind(balance: i64) -> &'static str {
    if balance > 0 { "refund" } else if balance < 0 { "additional_due" } else { "balanced" }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiscountItem { amount: Amount, date: CivilDate, correction_factors: Vec<Decimal>, months: u32 }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiscountsInput {
    items: Vec<DiscountItem>, reference_date: CivilDate, double_refund: bool,
    interest_percent_per_month: Decimal, interest_mode: InterestMode,
    rounding: RoundingMode, provenance: String,
}
fn discounts(input: DiscountsInput) -> ResultF<Value> {
    source(&input.provenance)?;
    interest_rate(input.interest_percent_per_month)?;
    if input.items.is_empty() { return Err(fail("Informe ao menos um desconto.")); }
    let mut rows = Vec::with_capacity(input.items.len());
    let (mut original_total,mut refund_principal,mut corrected_total,mut interest_total,mut events) = (0,0,0,0,0_usize);
    for item in &input.items {
        period_count(item.months)?;
        if item.date > input.reference_date || item.correction_factors.len() != item.months as usize {
            return Err(fail("Data posterior à referência ou quantidade de fatores diferente dos meses declarados."));
        }
        events = events.checked_add(item.correction_factors.len()).ok_or_else(|| fail("Limite de eventos excedido."))?;
        if events > MAX_EVENTS { return Err(fail("Máximo de 12000 eventos de correção por cálculo.")); }
        let amount = item.amount.nonnegative()?;
        let base = multiply(amount, if input.double_refund { 2 } else { 1 }, 1, input.rounding)?;
        let mut corrected = base;
        let mut timeline = Vec::with_capacity(item.correction_factors.len());
        for (i, factor) in item.correction_factors.iter().enumerate() {
            if factor.0 <= 0 { return Err(fail("Fator multiplicativo deve ser positivo.")); }
            let before = corrected;
            corrected = multiply(corrected, factor.0, SCALE, input.rounding)?;
            timeline.push(json!({"period":i + 1,"factor":decimal_string(factor.0),"before":money(before),"after":money(corrected)}));
        }
        let juros = interest(corrected,input.interest_percent_per_month,item.months,input.interest_mode,input.rounding)?;
        original_total = add(original_total,amount)?;
        refund_principal = add(refund_principal,base)?;
        corrected_total = add(corrected_total,corrected)?;
        interest_total = add(interest_total,juros)?;
        rows.push(json!({"date":item.date,"months":item.months,"original_amount":money(amount),
            "refund_principal":money(base),"corrected":money(corrected),"correction":money(sub(corrected,base)?),
            "juros":money(juros),"total":money(add(corrected,juros)?),"timeline":timeline}));
    }
    Ok(envelope("undue_discounts", &input.provenance, input.rounding,
        json!({"items":rows,"original_total":money(original_total),"refund_principal":money(refund_principal),
            "principal_corrected":money(corrected_total),"correction":money(sub(corrected_total,refund_principal)?),
            "juros":money(interest_total),"total":money(add(corrected_total,interest_total)?),
            "double_refund_hypothesis":input.double_refund,"statutory_claim":Value::Null}),
        &["Devolução em dobro é hipótese manual, aplicada antes da correção; nenhum direito ou fundamento legal é presumido.",
          "Fatores são multiplicadores, não percentuais, um por mês declarado. Datas apenas auditam a origem; meses são informados, sem prorrata automático.",
          "Juros calculados sobre a parcela corrigida, sem juros sobre honorários."], false))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConversionInput {
    amount: Amount, numerator: Decimal, denominator: Decimal, source_unit: String, target_unit: String,
    rounding: RoundingMode, provenance: String,
}
fn unit(s: &str) -> ResultF<()> {
    if s.trim().is_empty() || s.len() > 64 || s.chars().any(char::is_control) {
        Err(fail("Unidade nominal obrigatória, sem controles, até 64 bytes."))
    } else { Ok(()) }
}
fn conversion(input: ConversionInput) -> ResultF<Value> {
    source(&input.provenance)?;
    unit(&input.source_unit)?;
    unit(&input.target_unit)?;
    if input.numerator.0 < 0 || input.denominator.0 <= 0 { return Err(fail("Numerador não negativo e denominador positivo são obrigatórios.")); }
    let amount = input.amount.cents()?;
    let result = multiply(amount,input.numerator.0,input.denominator.0,input.rounding)?;
    Ok(envelope("currency_conversion",&input.provenance,input.rounding,
        json!({"source":{"amount":money(amount).to_decimal(),"cents":amount,"unit":input.source_unit},
            "converted":{"amount":money(result).to_decimal(),"cents":result,"unit":input.target_unit},
            "numerator":decimal_string(input.numerator.0),"denominator":decimal_string(input.denominator.0)}),
        &["Conversão nominal pela razão numerador/denominador informada; não conhece moedas históricas, câmbio, datas de vigência ou índices.",
          "Nesta operação, centavos significam centésimos da unidade declarada, não BRL presumido."],false))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AdjustmentPeriod { month: YearMonth, rate_percent: Decimal }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AdjustmentInput { start_value: Amount, period_rates: Vec<AdjustmentPeriod>, rounding: RoundingMode, provenance: String }
fn adjustment(input: AdjustmentInput) -> ResultF<Value> {
    source(&input.provenance)?;
    if input.period_rates.len() > MAX_PERIODS as usize { return Err(fail("Máximo de 1200 eventos de reajuste.")); }
    let start = input.start_value.nonnegative()?;
    let (mut value, mut previous) = (start,None);
    let mut timeline = Vec::with_capacity(input.period_rates.len());
    for p in &input.period_rates {
        if previous.is_some_and(|m| p.month <= m) { return Err(fail("Reajustes devem estar em meses distintos e ordem crescente.")); }
        correction_rate(p.rate_percent)?;
        let before = value;
        value = multiply(value,PERCENT + p.rate_percent.0,PERCENT,input.rounding)?;
        timeline.push(json!({"month":p.month,"rate_percent":decimal_string(p.rate_percent.0),
            "before":money(before),"adjustment":money(sub(value,before)?),"after":money(value)}));
        previous = Some(p.month);
    }
    Ok(envelope("benefit_adjustment",&input.provenance,input.rounding,
        json!({"start_value":money(start),"end_value":money(value),"adjustment_total":money(sub(value,start)?),"timeline":timeline}),
        &["Aplica somente eventos explicitamente informados; meses omitidos não recebem taxa presumida. Sem piso, teto, fator ou índice oficial automático."],false))
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum PaymentTiming { End, Beginning }
#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum Formula {
    FutureValue { principal_money: Amount, rate_percent_per_period: Decimal, periods: u32 },
    PresentValue { future_money: Amount, rate_percent_per_period: Decimal, periods: u32 },
    Payment { principal_money: Amount, rate_percent_per_period: Decimal, periods: u32, payment_timing: PaymentTiming },
    EquivalentRate { rate_percent: Decimal, source_periods: u32, target_periods: u32 },
    SimpleInterest { principal_money: Amount, rate_percent_per_period: Decimal, periods: u32 },
    CompoundInterest { principal_money: Amount, rate_percent_per_period: Decimal, periods: u32 },
    Discount { future_money: Amount, rate_percent_per_period: Decimal, periods: u32 },
    AnnuityPresentValue { payment_money: Amount, rate_percent_per_period: Decimal, periods: u32, payment_timing: PaymentTiming },
    AnnuityFutureValue { payment_money: Amount, rate_percent_per_period: Decimal, periods: u32, payment_timing: PaymentTiming },
    LoanSchedule { principal_money: Amount, rate_percent_per_period: Decimal, periods: u32 },
    RateFromValues { present_money: Amount, future_money: Amount, periods: u32 },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FormulaInput { formula: Formula, rounding: RoundingMode, provenance: String }
fn quantize(value_cents: f64, rounding: RoundingMode) -> ResultF<i64> {
    if !value_cents.is_finite() || value_cents.abs() > MAX_CENTS as f64 { return Err(fail("Fórmula não finita ou fora do limite monetário.")); }
    let rounded = match rounding { RoundingMode::HalfAwayFromZero => value_cents.round(), RoundingMode::TowardZero => value_cents.trunc() };
    bounded(rounded as i128)
}
fn rate_float(rate: Decimal) -> ResultF<f64> {
    correction_rate(rate)?;
    Ok(rate.0 as f64 / PERCENT as f64)
}
fn growth(rate: f64, periods: u32) -> ResultF<f64> {
    period_count(periods)?;
    let g = (rate.ln_1p() * f64::from(periods)).exp();
    if !g.is_finite() || g <= 0.0 { Err(fail("Fator exponencial não finito ou subfluxo numérico.")) } else { Ok(g) }
}
// Stable near zero: expm1 avoids cancellation in ((1+r)^n - 1).
fn annuity_factor(rate: f64, periods: u32, present: bool, timing: PaymentTiming) -> ResultF<f64> {
    period_count(periods)?;
    let factor = if rate == 0.0 { f64::from(periods) }
        else if present { -(-f64::from(periods) * rate.ln_1p()).exp_m1() / rate }
        else { (f64::from(periods) * rate.ln_1p()).exp_m1() / rate };
    let factor = factor * if matches!(timing,PaymentTiming::Beginning) { 1.0 + rate } else { 1.0 };
    if !factor.is_finite() || factor < 0.0 { Err(fail("Fator de anuidade não finito.")) } else { Ok(factor) }
}
fn rate_output(rate_percent: f64, rounding: RoundingMode) -> ResultF<String> {
    if !rate_percent.is_finite() || rate_percent.abs() > 1_000_000.0 { return Err(fail("Taxa resultante não finita ou fora dos limites.")); }
    let scaled = rate_percent * SCALE as f64;
    let rounded = match rounding { RoundingMode::HalfAwayFromZero => scaled.round(), RoundingMode::TowardZero => scaled.trunc() };
    Ok(decimal_string(rounded as i128))
}
fn formulas(input: FormulaInput) -> ResultF<Value> {
    source(&input.provenance)?;
    let rounding = input.rounding;
    let operation = match &input.formula {
        Formula::FutureValue {..} => "future_value", Formula::PresentValue {..} => "present_value",
        Formula::Payment {..} => "payment", Formula::EquivalentRate {..} => "equivalent_rate",
        Formula::SimpleInterest {..} => "simple_interest", Formula::CompoundInterest {..} => "compound_interest",
        Formula::Discount {..} => "discount", Formula::AnnuityPresentValue {..} => "annuity_present_value",
        Formula::AnnuityFutureValue {..} => "annuity_future_value", Formula::LoanSchedule {..} => "loan_schedule",
        Formula::RateFromValues {..} => "rate_from_values",
    };
    let approximate = operation != "simple_interest";
    let mut data = match input.formula {
        Formula::FutureValue {principal_money,rate_percent_per_period,periods}
        | Formula::CompoundInterest {principal_money,rate_percent_per_period,periods} => {
            let principal = principal_money.nonnegative()?;
            let end = quantize(principal as f64 * growth(rate_float(rate_percent_per_period)?,periods)?,rounding)?;
            json!({"principal":money(principal),"future_value":money(end),"interest":money(sub(end,principal)?),"periods":periods})
        },
        Formula::PresentValue {future_money,rate_percent_per_period,periods}
        | Formula::Discount {future_money,rate_percent_per_period,periods} => {
            let future = future_money.nonnegative()?;
            let present = quantize(future as f64 / growth(rate_float(rate_percent_per_period)?,periods)?,rounding)?;
            json!({"future_value":money(future),"present_value":money(present),"discount":money(sub(future,present)?),"periods":periods})
        },
        Formula::SimpleInterest {principal_money,rate_percent_per_period,periods} => {
            let principal = principal_money.nonnegative()?;
            period_count(periods)?;
            correction_rate(rate_percent_per_period)?;
            let i = multiply(principal,rate_percent_per_period.0 * i128::from(periods),PERCENT,rounding)?;
            let future = add(principal,i)?;
            if future < 0 { return Err(fail("Juros simples produzem saldo negativo.")); }
            json!({"principal":money(principal),"future_value":money(future),"interest":money(i),"periods":periods})
        },
        Formula::Payment {principal_money,rate_percent_per_period,periods,payment_timing} => {
            positive_period_count(periods)?;
            let principal = principal_money.nonnegative()?;
            let factor = annuity_factor(rate_float(rate_percent_per_period)?,periods,true,payment_timing)?;
            let payment = quantize(principal as f64 / factor,rounding)?;
            json!({"principal":money(principal),"payment":money(payment),"periods":periods,
                "payment_timing":if matches!(payment_timing,PaymentTiming::End) { "end" } else { "beginning" }})
        },
        Formula::EquivalentRate {rate_percent,source_periods,target_periods} => {
            positive_period_count(source_periods)?;
            period_count(target_periods)?;
            let r = rate_float(rate_percent)?;
            let equivalent = (r.ln_1p() * f64::from(target_periods) / f64::from(source_periods)).exp_m1() * 100.0;
            json!({"equivalent_percent":rate_output(equivalent,rounding)?,"source_periods":source_periods,"target_periods":target_periods})
        },
        Formula::AnnuityPresentValue {payment_money,rate_percent_per_period,periods,payment_timing} => {
            let payment = payment_money.nonnegative()?;
            let factor = annuity_factor(rate_float(rate_percent_per_period)?,periods,true,payment_timing)?;
            json!({"payment":money(payment),"present_value":money(quantize(payment as f64 * factor,rounding)?),"periods":periods})
        },
        Formula::AnnuityFutureValue {payment_money,rate_percent_per_period,periods,payment_timing} => {
            let payment = payment_money.nonnegative()?;
            let factor = annuity_factor(rate_float(rate_percent_per_period)?,periods,false,payment_timing)?;
            json!({"payment":money(payment),"future_value":money(quantize(payment as f64 * factor,rounding)?),"periods":periods})
        },
        Formula::LoanSchedule {principal_money,rate_percent_per_period,periods} => {
            positive_period_count(periods)?;
            interest_rate(rate_percent_per_period)?;
            let principal = principal_money.nonnegative()?;
            let factor = annuity_factor(rate_float(rate_percent_per_period)?,periods,true,PaymentTiming::End)?;
            let payment = quantize(principal as f64 / factor,rounding)?;
            let (mut balance,mut total_interest,mut total_paid) = (principal,0,0);
            let mut timeline = Vec::with_capacity(periods as usize);
            for period in 1..=periods {
                let i = multiply(balance,rate_percent_per_period.0,PERCENT,rounding)?;
                let due = add(balance,i)?;
                let paid = if period == periods { due } else { payment.min(due) };
                if paid < i { return Err(fail("Prestação arredondada produziria amortização negativa.")); }
                let amortization = sub(paid,i)?;
                let opening = balance;
                balance = sub(balance,amortization)?;
                total_interest = add(total_interest,i)?;
                total_paid = add(total_paid,paid)?;
                timeline.push(json!({"period":period,"opening_balance":money(opening),"interest":money(i),
                    "payment":money(paid),"amortization":money(amortization),"closing_balance":money(balance)}));
            }
            json!({"principal":money(principal),"regular_payment":money(payment),"total_paid":money(total_paid),
                "total_interest":money(total_interest),"closing_balance":money(balance),"timeline":timeline})
        },
        Formula::RateFromValues {present_money,future_money,periods} => {
            positive_period_count(periods)?;
            let present = present_money.nonnegative()?;
            let future = future_money.nonnegative()?;
            if present == 0 || future == 0 { return Err(fail("Taxa implícita exige valores positivos.")); }
            let rate = ((future as f64 / present as f64).ln() / f64::from(periods)).exp_m1() * 100.0;
            json!({"rate_percent_per_period":rate_output(rate,rounding)?,"periods":periods})
        },
    };
    data["operation"] = json!(operation);
    Ok(envelope("financial_formula",&input.provenance,rounding,data,
        &["Sem tarifas, tributos ou inflação presumidos. Períodos são unidades declaradas e uniformes.",
          "Futuro/presente capitalizam sem arredondar os períodos intermediários. Fluxos de anuidade seguem o momento explicitamente selecionado.",
          "Em cronograma de empréstimo, juros e prestações em centavos por período; última prestação ajusta o saldo residual."],approximate))
}

/// Stable integration entry point. Unknown IDs and invalid inputs return errors,
/// never fabricated zero results. No IO, personal data, official tables or clock.
/// Existing pension_accumulation intentionally remains in crate::run_tool.
pub fn run_finance(tool: &str, input: Value) -> Result<Value, String> {
    check_size(&input,0,&mut 100_000)?;
    match tool {
        "update_values" => update(decode(input)?),
        "irrf_refund" => tax(decode(input)?),
        "undue_discounts" => discounts(decode(input)?),
        "financial_formula" => formulas(decode(input)?),
        "currency_conversion" => conversion(decode(input)?),
        "benefit_adjustment" => adjustment(decode(input)?),
        _ => Err(fail("Ferramenta financeira desconhecida ou não implementada.")),
    }
}
