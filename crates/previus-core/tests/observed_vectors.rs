// Synthetic oracles only, copied from runtime/behavioral-test-vectors.json.
// Source hash and policy provenance are recorded in src/lib.rs.
use previus_core::{dates::{count_inclusive_calendar_days, InclusiveIntervalInput},
    pension::{calculate_pension_accumulation, PensionAccumulationInput},
    quality::{calculate_facultative_quality, InsuredCategory, QualityOfInsuredInput},
    CivilDate, Money, OutcomeStatus, RoundingMode, RuleReviewStatus};
use serde_json::Value;

fn vectors() -> Value { serde_json::from_str(include_str!("fixtures/observed-vectors.json")).unwrap() }
fn get(id: &str) -> Value { vectors()["tests"].as_array().unwrap().iter().find(|v| v["id"] == id).unwrap().clone() }
fn date(v: &Value) -> CivilDate { v.as_str().unwrap().parse().unwrap() }
fn money(v: &Value) -> Money { Money::parse_decimal(v.as_str().unwrap()).unwrap() }
fn check_quality(id: &str) {
    let v = get(id);
    let result = calculate_facultative_quality(&QualityOfInsuredInput {
        stopped_contributing: true, currently_receiving_benefit: false,
        insured_category: InsuredCategory::Facultative,
        last_contribution_date: date(&v["input"]["last_contribution"]),
    }).unwrap();
    assert_eq!(result.status, OutcomeStatus::Experimental);
    assert_eq!(result.rule_review_status, RuleReviewStatus::BehaviorOnly);
    assert!(!result.warnings.is_empty());
    let output = result.data.unwrap();
    assert_eq!(output.grace_months, v["output"]["grace_months"].as_u64().unwrap() as u32);
    assert_eq!(output.last_grace_month.to_string(), v["output"]["last_grace_month"].as_str().unwrap());
    assert_eq!(output.latest_payment_date, date(&v["output"]["payment_deadline"]));
    assert_eq!(output.quality_loss_date, date(&v["output"]["loss_date"]));
}
#[test] fn test_02_facultative_observation() { check_quality("TEST-02"); }
#[test] fn test_03_facultative_leap_observation() { check_quality("TEST-03"); }
#[test] fn test_04_same_day_ui047() {
    let v = get("TEST-04");
    let outcome = count_inclusive_calendar_days(&InclusiveIntervalInput {
        start: date(&v["input"]["start"]), end: date(&v["input"]["end"]),
    }).unwrap();
    assert_eq!(outcome.status, OutcomeStatus::Experimental);
    let result = outcome.data.unwrap();
    assert_eq!(result.calendar_days, v["output"]["calendar_days"].as_u64().unwrap());
    assert_eq!(format!("{} / {}", result.duration_text, result.total_days_text), v["output"]["display"].as_str().unwrap());
}
#[test] fn test_05_pension_exact_uia_ui074() {
    let v = get("TEST-05");
    let result = calculate_pension_accumulation(&PensionAccumulationInput {
        reference_date: date(&v["input"]["date"]),
        incomes: vec![money(&v["input"]["pension"]), money(&v["input"]["second_income"]),
            money(&v["input"]["third_income"]), money(&v["input"]["fourth_income"])],
        // Explicit UI fixture input, NOT a hardcoded application indicator.
        reference_base: money(&v["output"]["displayed_reference_base"]),
        rounding: RoundingMode::HalfAwayFromZero,
    }).unwrap();
    assert_eq!(result.status, OutcomeStatus::Experimental);
    assert_eq!(result.rule_review_status, RuleReviewStatus::BehaviorOnly);
    let output = result.data.unwrap();
    assert_eq!(output.integral_income, money(&v["output"]["unreduced_income"]));
    assert_eq!(output.integral_income_index, 1);
    assert_eq!(output.reduced_total, money(&v["output"]["reduced_pension"]));
    assert_eq!(output.incomes[0].received_percentage.as_deref(), v["output"]["reduced_pension_percent"].as_str());
    for (i, band) in v["output"]["bands"].as_array().unwrap().iter().enumerate() {
        assert_eq!(output.incomes[0].tranches[i].received_amount, money(band));
    }
    // combined_total is derived, NOT asserted as a legacy UI primary oracle.
    assert_eq!(output.combined_total, output.integral_income.checked_add(output.reduced_total).unwrap());
}
