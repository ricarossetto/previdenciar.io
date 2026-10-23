use previus_core::{finance::run_finance, run_tool};
use serde_json::{json, Value};

fn example(tool: &str) -> Value {
    let schema: Value = serde_json::from_str(include_str!("finance-schema.json")).unwrap();
    schema["x-examples"].as_array().unwrap().iter()
        .find(|e| e["tool"] == tool).unwrap()["input"].clone()
}
fn run(tool: &str, input: Value) -> Value { run_finance(tool,input).unwrap() }
fn calc(operation: Value) -> Value {
    run("financial_formula",json!({"formula":operation,"rounding":"half_away_from_zero","provenance":"synthetic"}))
}
fn amount(value: &Value, name: &str) -> String { value["data"][name]["amount"].as_str().unwrap().to_owned() }
fn cents(value: &Value) -> i64 {
    previus_core::Money::parse_decimal(value["amount"].as_str().unwrap()).unwrap().cents()
}
#[test]
fn published_examples_and_public_dispatch_are_executable() {
    let schema: Value = serde_json::from_str(include_str!("finance-schema.json")).unwrap();
    assert_eq!(schema["oneOf"].as_array().unwrap().len(),7);
    for e in schema["x-examples"].as_array().unwrap() {
        let tool = e["tool"].as_str().unwrap();
        let direct = run(tool,e["input"].clone());
        assert_eq!(run_tool(tool,e["input"].clone()).unwrap(),direct);
        assert_eq!(direct["status"],"experimental");
        assert_eq!(direct["rules_verified"],false);
        let expected_source = if tool == "irrf_oficial" { "rgps_core::dados::irrf" } else { "operator_input" };
        assert_eq!(direct["reference_tables_source"],expected_source);
        assert_eq!(direct["provenance"],e["input"]["provenance"]);
        assert!(direct["assumptions"].as_array().unwrap().len() >= 4);
        assert!(!direct["warnings"].as_array().unwrap().is_empty());
        assert!(direct["data"].is_object());
    }
}
#[test]
fn every_required_top_level_field_is_really_required() {
    for tool in ["update_values","irrf_refund","irrf_oficial","undue_discounts","financial_formula","currency_conversion","benefit_adjustment"] {
        let original = example(tool);
        for key in original.as_object().unwrap().keys() {
            let mut missing = original.clone();
            missing.as_object_mut().unwrap().remove(key);
            assert!(run_finance(tool,missing).is_err(),"{tool} must require {key}");
        }
    }
}
#[test]
fn unknown_top_level_fields_are_rejected() {
    for tool in ["update_values","irrf_refund","irrf_oficial","undue_discounts","financial_formula","currency_conversion","benefit_adjustment"] {
        let mut input = example(tool); input["typo"] = json!(true);
        assert!(run_finance(tool,input).is_err());
    }
}
#[test]
fn provenance_is_mandatory_nonblank_bounded_and_control_free() {
    for invalid in ["", "  ", "source\nnot_allowed", "\u{0000}"] {
        let mut input = example("update_values"); input["provenance"] = json!(invalid);
        assert!(run_finance("update_values",input).is_err());
    }
    let mut input = example("update_values"); input["provenance"] = json!("s".repeat(513));
    assert!(run_finance("update_values",input).is_err());
}
#[test]
fn decimal_rates_reject_nan_infinity_exponents_locale_and_extra_places() {
    for bad in [json!("NaN"),json!("inf"),json!("1e2"),json!("1,2"),json!(" 1"),json!("+1"),json!("1."),json!("0.000000001"),json!(1.0),json!(null)] {
        let mut input = example("update_values"); input["interest_percent_per_month"] = bad;
        assert!(run_finance("update_values",input).is_err());
    }
}
#[test]
fn money_rejects_bad_units_locale_non_cents_and_numeric_json() {
    for bad in [json!("1.001"),json!("NaN"),json!("1,00"),json!("1e3"),json!(1),json!({"currency":"USD","amount":"1.00"}),json!({"currency":"BRL","amount":"1.00","unexpected":1})] {
        let mut input = example("update_values"); input["principal_money"] = bad;
        assert!(run_finance("update_values",input).is_err());
    }
}
#[test]
fn existing_money_objects_are_compatible_with_string_adapter() {
    let original = example("update_values");
    let mut input = original.clone();
    input["principal_money"] = json!({"currency":"BRL","amount":"1000.00"});
    input["fees_fixed_amount"] = json!({"currency":"BRL","amount":"5.00"});
    assert_eq!(run("update_values",input),run("update_values",original));
}
#[test]
fn update_correction_interest_and_both_fee_types_are_nontrivial() {
    let result = run("update_values",example("update_values"));
    assert_eq!(amount(&result,"principal"),"1000.00");
    assert_eq!(amount(&result,"principal_corrected"),"1030.20");
    assert_eq!(amount(&result,"correction"),"30.20");
    assert_eq!(amount(&result,"juros"),"20.60");
    assert_eq!(amount(&result,"fees_base"),"1050.80");
    assert_eq!(amount(&result,"fees_percentage_amount"),"105.08");
    assert_eq!(amount(&result,"honorarios"),"110.08");
    assert_eq!(amount(&result,"total"),"1160.88");
    assert_eq!(result["data"]["timeline"].as_array().unwrap().len(),2);
}
#[test]
fn update_compound_interest_has_period_cent_rounding() {
    let mut input = example("update_values"); input["interest_mode"] = json!("compound");
    let result = run("update_values",input);
    assert_eq!(amount(&result,"juros"),"20.71");
    assert_eq!(amount(&result,"total"),"1161.00");
    assert_eq!(result["rounding_policy"]["compound_interest"],"each_period");
}
#[test]
fn update_zero_months_is_identity_plus_explicit_fees() {
    let mut input = example("update_values"); input["months"] = json!(0); input["monthly_correction_percent"] = json!([]);
    input["interest_mode"] = json!("compound");
    let result = run("update_values",input);
    assert_eq!(amount(&result,"principal_corrected"),"1000.00");
    assert_eq!(amount(&result,"juros"),"0.00");
    assert_eq!(amount(&result,"total"),"1105.00");
}
#[test]
fn update_rejects_period_mismatch_negative_interest_and_negative_principal() {
    for (key,value) in [("months",json!(3)),("months",json!(1201)),("months",json!(-1)),("interest_percent_per_month",json!("-1")),("principal_money",json!("-1")),("fees_fixed_amount",json!("-1")),("fees_percent",json!("-1")),("interest_mode",json!("guess"))] {
        let mut input=example("update_values"); input[key]=value;
        assert!(run_finance("update_values",input).is_err());
    }
}
#[test]
fn update_negative_correction_is_explicit_and_not_negative_interest() {
    let mut input=example("update_values"); input["monthly_correction_percent"]=json!(["-10","0"]);
    let result=run("update_values",input);
    assert_eq!(amount(&result,"principal_corrected"),"900.00");
    assert_eq!(amount(&result,"correction"),"-100.00");
}
#[test]
fn correction_cannot_destroy_or_reverse_the_principal_at_minus_100() {
    for bad in ["-100","-100.00000001","-101"] {
        let mut input=example("update_values"); input["monthly_correction_percent"]=json!([bad,"0"]);
        assert!(run_finance("update_values",input).is_err());
    }
}
#[test]
fn update_and_compound_overflow_return_errors_not_zero_or_null() {
    let mut input=example("update_values"); input["principal_money"]=json!("90000000000000.00");
    assert!(run_finance("update_values",input).is_err());
    let mut input=example("update_values"); input["monthly_correction_percent"]=json!(["0","0"]);
    input["interest_percent_per_month"]=json!("1000000"); input["interest_mode"]=json!("compound"); input["principal_money"]=json!("1000000");
    assert!(run_finance("update_values",input).is_err());
}
#[test]
fn update_supports_zero_principal_without_inventing_values() {
    let mut input=example("update_values"); input["principal_money"]=json!("0"); input["fees_fixed_amount"]=json!("0");
    let result=run("update_values",input);
    assert_eq!(amount(&result,"total"),"0.00");
}
#[test]
fn rounding_half_cents_is_explicit() {
    let mut input=example("update_values"); input["principal_money"]=json!("0.01");
    input["months"]=json!(1); input["monthly_correction_percent"]=json!(["50"]);
    input["interest_percent_per_month"]=json!("0");input["fees_percent"]=json!("0");input["fees_fixed_amount"]=json!("0");
    assert_eq!(amount(&run("update_values",input.clone()),"principal_corrected"),"0.02");
    input["rounding"]=json!("toward_zero");
    assert_eq!(amount(&run("update_values",input),"principal_corrected"),"0.01");
}
#[test]
fn tax_rate_minus_deduction_has_signed_balance_and_label() {
    let result=run("irrf_refund",example("irrf_refund"));
    assert_eq!(amount(&result,"total_calculated_tax"),"300.00");
    assert_eq!(amount(&result,"refund_minus_due"),"-120.00");
    assert_eq!(result["data"]["balance_kind"],"additional_due");
    assert_eq!(amount(&result,"net_refund"),"0.00");
    assert_eq!(amount(&result,"net_additional_due"),"120.00");
}
#[test]
fn tax_marginal_model_accumulates_only_slices() {
    let mut input=example("irrf_refund"); input["schedule_model"]=json!("marginal");
    for b in input["tax_brackets"].as_array_mut().unwrap() { b["deduction"]=json!("0"); }
    let result=run("irrf_refund",input);
    assert_eq!(amount(&result,"total_calculated_tax"),"300.00");
}
#[test]
fn tax_boundary_is_lower_inclusive_upper_exclusive() {
    let mut input=example("irrf_refund");
    input["rows"]=json!([
        {"month":"2026-01","taxable_amount":"999.99","withheld":"0"},
        {"month":"2026-02","taxable_amount":"1000","withheld":"0"},
        {"month":"2026-03","taxable_amount":"2000","withheld":"100"}]);
    let r=run("irrf_refund",input);
    assert_eq!(r["data"]["rows"][0]["calculated_tax"]["amount"],"0.00");
    assert_eq!(r["data"]["rows"][1]["calculated_tax"]["amount"],"0.00");
    assert_eq!(r["data"]["rows"][2]["calculated_tax"]["amount"],"100.00");
    assert_eq!(r["data"]["balance_kind"],"balanced");
}
#[test]
fn tax_preserves_row_refunds_and_dues_before_informational_offset() {
    let mut input=example("irrf_refund");
    input["rows"]=json!([
        {"month":"2026-01","taxable_amount":"3000","withheld":"500"},
        {"month":"2026-02","taxable_amount":"3000","withheld":"250"}]);
    let r=run("irrf_refund",input);
    assert_eq!(amount(&r,"total_refund_before_offset"),"200.00");
    assert_eq!(amount(&r,"total_due_before_offset"),"50.00");
    assert_eq!(amount(&r,"refund_minus_due"),"150.00");
    assert_eq!(r["data"]["balance_kind"],"refund");
}
#[test]
fn tax_empty_rows_empty_brackets_duplicates_and_invalid_month_rejected() {
    let mut a=example("irrf_refund");a["rows"]=json!([]); assert!(run_finance("irrf_refund",a).is_err());
    let mut a=example("irrf_refund");a["tax_brackets"]=json!([]);assert!(run_finance("irrf_refund",a).is_err());
    let mut a=example("irrf_refund");let row=a["rows"][0].clone();a["rows"]=json!([row.clone(),row]);assert!(run_finance("irrf_refund",a).is_err());
    let mut a=example("irrf_refund");a["rows"][0]["month"]=json!("2026-13");assert!(run_finance("irrf_refund",a).is_err());
}
#[test]
fn tax_requires_sorted_full_coverage_and_no_overlaps_or_gaps() {
    for (index,key,value) in [(0,"lower",json!("1")),(1,"lower",json!("999")),(1,"lower",json!("1001")),(2,"upper",json!("5000")),(0,"upper",json!(null)),(1,"upper",json!("1000"))] {
        let mut input=example("irrf_refund");input["tax_brackets"][index][key]=value;
        assert!(run_finance("irrf_refund",input).is_err());
    }
    let mut input=example("irrf_refund");input["tax_brackets"].as_array_mut().unwrap().swap(0,1);
    assert!(run_finance("irrf_refund",input).is_err());
}
#[test]
fn tax_deductions_cannot_be_silently_ignored_or_create_negative_tax() {
    let mut a=example("irrf_refund");a["schedule_model"]=json!("marginal");assert!(run_finance("irrf_refund",a).is_err());
    let mut a=example("irrf_refund");a["tax_brackets"][0]["deduction"]=json!("0.01");assert!(run_finance("irrf_refund",a).is_err());
    let mut a=example("irrf_refund");a["tax_brackets"][2]["deduction"]=json!("301");assert!(run_finance("irrf_refund",a).is_err());
}
#[test]
fn tax_rates_are_progressive_and_at_most_100_percent() {
    for bad in ["-1","101"] {
        let mut a=example("irrf_refund");a["tax_brackets"][2]["rate"]=json!(bad);
        assert!(run_finance("irrf_refund",a).is_err());
    }
    let mut a=example("irrf_refund");a["tax_brackets"][2]["rate"]=json!("9");
    assert!(run_finance("irrf_refund",a).is_err());
}
#[test]
fn tax_rounds_once_per_row_after_summing_exact_marginal_slices() {
    let input=json!({"rows":[{"month":"2026-01","taxable_amount":"0.03","withheld":"0"}],
        "tax_brackets":[{"lower":"0","upper":"0.01","rate":"25","deduction":"0"},
        {"lower":"0.01","upper":null,"rate":"25","deduction":"0"}],
        "schedule_model":"marginal","rounding":"half_away_from_zero","provenance":"synthetic"});
    let r=run("irrf_refund",input);
    assert_eq!(amount(&r,"total_calculated_tax"),"0.01");
}
#[test]
fn discounts_double_refund_is_actual_math_and_manual_hypothesis() {
    let r=run("undue_discounts",example("undue_discounts"));
    assert_eq!(amount(&r,"original_total"),"100.00");
    assert_eq!(amount(&r,"refund_principal"),"200.00");
    assert_eq!(amount(&r,"principal_corrected"),"264.00");
    assert_eq!(amount(&r,"correction"),"64.00");
    assert_eq!(amount(&r,"juros"),"5.28");
    assert_eq!(amount(&r,"total"),"269.28");
    assert_eq!(r["data"]["double_refund_hypothesis"],true);
    assert!(r["data"]["statutory_claim"].is_null());
}
#[test]
fn discounts_single_refund_and_per_item_audit_are_preserved() {
    let mut input=example("undue_discounts");input["double_refund"]=json!(false);
    let r=run("undue_discounts",input);
    assert_eq!(amount(&r,"total"),"134.64");
    assert_eq!(r["data"]["items"][0]["timeline"].as_array().unwrap().len(),2);
    assert_eq!(r["data"]["items"][0]["date"],"2026-01-01");
}
#[test]
fn discounts_compound_differs_from_simple_without_touching_hypothesis() {
    let mut input=example("undue_discounts");input["interest_mode"]=json!("compound");
    let r=run("undue_discounts",input);
    assert_eq!(amount(&r,"juros"),"5.31");
    assert_eq!(amount(&r,"total"),"269.31");
}
#[test]
fn discounts_require_valid_dates_positive_factors_and_declared_months() {
    for (key,value) in [("date",json!("2026-02-30")),("date",json!("2027-01-01")),("months",json!(3)),("amount",json!("-1")),("correction_factors",json!(["0","1"])),("correction_factors",json!(["-1","1"]))] {
        let mut input=example("undue_discounts");input["items"][0][key]=value;
        assert!(run_finance("undue_discounts",input).is_err());
    }
}
#[test]
fn discounts_empty_items_missing_amount_and_unknown_nested_fields_fail_closed() {
    let mut input=example("undue_discounts");input["items"]=json!([]);assert!(run_finance("undue_discounts",input).is_err());
    let mut input=example("undue_discounts");input["items"][0].as_object_mut().unwrap().remove("amount");assert!(run_finance("undue_discounts",input).is_err());
    let mut input=example("undue_discounts");input["items"][0]["typo"]=json!(true);assert!(run_finance("undue_discounts",input).is_err());
}
#[test]
fn discounts_zero_months_keeps_principal_and_never_infers_elapsed_dates() {
    let mut input=example("undue_discounts");input["items"][0]["months"]=json!(0);input["items"][0]["correction_factors"]=json!([]);
    let r=run("undue_discounts",input);
    assert_eq!(amount(&r,"total"),"200.00");
}
#[test]
fn conversion_uses_exact_ratio_and_preserves_declared_units() {
    let r=run("currency_conversion",example("currency_conversion"));
    assert_eq!(r["data"]["converted"]["amount"],"66.67");
    assert_eq!(r["data"]["converted"]["cents"],6667);
    assert_eq!(r["data"]["converted"]["unit"],"nominal_B");
    assert!(r["data"]["converted"].get("currency").is_none());
}
#[test]
fn conversion_negative_ties_round_away_from_zero_or_towards_zero() {
    let mut input=example("currency_conversion");input["amount"]=json!("-0.01");input["numerator"]=json!("1");input["denominator"]=json!("2");
    assert_eq!(run("currency_conversion",input.clone())["data"]["converted"]["amount"],"-0.01");
    input["rounding"]=json!("toward_zero");
    assert_eq!(run("currency_conversion",input)["data"]["converted"]["amount"],"0.00");
}
#[test]
fn conversion_rejects_zero_denominator_negative_multiplier_missing_units_and_overflow() {
    for (key,value) in [("denominator",json!("0")),("denominator",json!("-1")),("numerator",json!("-1")),("source_unit",json!("")),("target_unit",json!("  ")),("target_unit",json!("unsafe\nunit"))] {
        let mut input=example("currency_conversion");input[key]=value;
        assert!(run_finance("currency_conversion",input).is_err());
    }
    let mut input=example("currency_conversion");input["amount"]=json!("90000000000000.00");
    let valid=run_finance("currency_conversion",input).unwrap(); // 2/3 reduces magnitude safely.
    assert_eq!(valid["data"]["converted"]["amount"],"60000000000000.00");
    let mut input=example("currency_conversion");input["amount"]=json!("90000000000000.00");input["numerator"]=json!("4");
    assert!(run_finance("currency_conversion",input).is_err());
}
#[test]
fn conversion_integer_ratio_property_for_many_values() {
    for c in -100..=100 {
        let mut input=example("currency_conversion");input["amount"]=json!(previus_core::Money::from_cents(c).to_decimal());
        input["numerator"]=json!("7");input["denominator"]=json!("1");
        assert_eq!(run("currency_conversion",input)["data"]["converted"]["cents"],c * 7);
    }
}
#[test]
fn conversion_zero_numerator_is_declared_zero_not_missing_fallback() {
    let mut input=example("currency_conversion");input["numerator"]=json!("0");
    assert_eq!(run("currency_conversion",input)["data"]["converted"]["amount"],"0.00");
}
#[test]
fn adjustment_applies_user_events_and_retains_timeline() {
    let r=run("benefit_adjustment",example("benefit_adjustment"));
    assert_eq!(amount(&r,"end_value"),"1045.00");
    assert_eq!(amount(&r,"adjustment_total"),"45.00");
    assert_eq!(r["data"]["timeline"][1]["month"],"2026-03");
    assert_eq!(r["data"]["timeline"][1]["adjustment"]["amount"],"-55.00");
}
#[test]
fn adjustment_does_not_fill_missing_months_or_rates() {
    let mut input=example("benefit_adjustment");input["period_rates"]=json!([]);
    let r=run("benefit_adjustment",input);
    assert_eq!(amount(&r,"end_value"),"1000.00");
    assert!(r["data"]["timeline"].as_array().unwrap().is_empty());
    let mut input=example("benefit_adjustment");input["period_rates"][0].as_object_mut().unwrap().remove("rate_percent");
    assert!(run_finance("benefit_adjustment",input).is_err());
}
#[test]
fn adjustment_rejects_duplicate_unsorted_invalid_months_and_impossible_rates() {
    for (key,value) in [("month",json!("2026-01")),("month",json!("2025-12")),("month",json!("0000-01")),("rate_percent",json!("-100"))] {
        let mut input=example("benefit_adjustment");input["period_rates"][1][key]=value;
        assert!(run_finance("benefit_adjustment",input).is_err());
    }
}
#[test]
fn future_present_and_discount_use_actual_mathematics() {
    let future=calc(json!({"operation":"future_value","principal_money":"1000","rate_percent_per_period":"1","periods":2}));
    assert_eq!(amount(&future,"future_value"),"1020.10");
    let present=calc(json!({"operation":"present_value","future_money":"1020.10","rate_percent_per_period":"1","periods":2}));
    assert_eq!(amount(&present,"present_value"),"1000.00");
    let discount=calc(json!({"operation":"discount","future_money":"1020.10","rate_percent_per_period":"1","periods":2}));
    assert_eq!(amount(&discount,"discount"),"20.10");
}
#[test]
fn simple_and_compound_interest_formulas_are_distinct() {
    let simple=calc(json!({"operation":"simple_interest","principal_money":"1000","rate_percent_per_period":"1","periods":2}));
    let compound=calc(json!({"operation":"compound_interest","principal_money":"1000","rate_percent_per_period":"1","periods":2}));
    assert_eq!(amount(&simple,"interest"),"20.00");
    assert_eq!(amount(&compound,"interest"),"20.10");
}
#[test]
fn payment_formula_supports_end_and_beginning_and_zero_rate() {
    let mut formula=json!({"operation":"payment","principal_money":"1000","rate_percent_per_period":"1","periods":12,"payment_timing":"end"});
    assert_eq!(amount(&calc(formula.clone()),"payment"),"88.85");
    formula["payment_timing"]=json!("beginning");
    assert_eq!(amount(&calc(formula.clone()),"payment"),"87.97");
    formula["rate_percent_per_period"]=json!("0");formula["principal_money"]=json!("1200");
    assert_eq!(amount(&calc(formula),"payment"),"100.00");
}
#[test]
fn equivalent_rate_and_implied_rate_are_nontrivial() {
    let r=calc(json!({"operation":"equivalent_rate","rate_percent":"1","source_periods":1,"target_periods":12}));
    assert_eq!(r["data"]["equivalent_percent"],"12.68250301");
    let r=calc(json!({"operation":"rate_from_values","present_money":"1000","future_money":"1020.10","periods":2}));
    assert_eq!(r["data"]["rate_percent_per_period"],"1.00000000");
}
#[test]
fn annuity_present_and_future_end_vs_beginning() {
    let mut formula=json!({"operation":"annuity_present_value","payment_money":"100","rate_percent_per_period":"1","periods":12,"payment_timing":"end"});
    assert_eq!(amount(&calc(formula.clone()),"present_value"),"1125.51");
    formula["payment_timing"]=json!("beginning");
    assert_eq!(amount(&calc(formula.clone()),"present_value"),"1136.76");
    formula["operation"]=json!("annuity_future_value");formula["payment_timing"]=json!("end");
    assert_eq!(amount(&calc(formula.clone()),"future_value"),"1268.25");
    formula["payment_timing"]=json!("beginning");
    assert_eq!(amount(&calc(formula),"future_value"),"1280.93");
}
#[test]
fn annuities_are_stable_at_zero_and_small_rates() {
    for rate in ["0","0.00000001"] {
        let r=calc(json!({"operation":"annuity_future_value","payment_money":"100","rate_percent_per_period":rate,"periods":12,"payment_timing":"end"}));
        assert_eq!(amount(&r,"future_value"),"1200.00");
        let r=calc(json!({"operation":"annuity_present_value","payment_money":"100","rate_percent_per_period":rate,"periods":12,"payment_timing":"end"}));
        assert_eq!(amount(&r,"present_value"),"1200.00");
    }
}
#[test]
fn loan_schedule_closes_and_conserves_integer_cents_every_row() {
    let r=calc(json!({"operation":"loan_schedule","principal_money":"1000","rate_percent_per_period":"1","periods":12}));
    let rows=r["data"]["timeline"].as_array().unwrap();
    assert_eq!(rows.len(),12);
    assert_eq!(amount(&r,"regular_payment"),"88.85");
    assert_eq!(amount(&r,"closing_balance"),"0.00");
    let (mut paid,mut interest,mut amortization)=(0,0,0);
    for row in rows {
        let a=cents(&row["amortization"]);let i=cents(&row["interest"]);let p=cents(&row["payment"]);
        assert_eq!(p,a+i);
        assert_eq!(cents(&row["opening_balance"])-a,cents(&row["closing_balance"]));
        assert!(cents(&row["closing_balance"])>=0);
        paid+=p;interest+=i;amortization+=a;
    }
    assert_eq!(amortization,100_000);assert_eq!(paid,amortization+interest);
    assert_eq!(paid,cents(&r["data"]["total_paid"]));
    assert_eq!(interest,cents(&r["data"]["total_interest"]));
}
#[test]
fn loan_schedule_last_payment_adjusts_a_residual_cent() {
    let r=calc(json!({"operation":"loan_schedule","principal_money":"1","rate_percent_per_period":"0","periods":3}));
    assert_eq!(amount(&r,"regular_payment"),"0.33");
    assert_eq!(r["data"]["timeline"][2]["payment"]["amount"],"0.34");
    assert_eq!(amount(&r,"total_paid"),"1.00");
}
#[test]
fn formula_zero_periods_has_identity_where_defined_and_error_elsewhere() {
    let r=calc(json!({"operation":"future_value","principal_money":"123.45","rate_percent_per_period":"1","periods":0}));
    assert_eq!(amount(&r,"future_value"),"123.45");
    let r=calc(json!({"operation":"annuity_future_value","payment_money":"100","rate_percent_per_period":"1","periods":0,"payment_timing":"end"}));
    assert_eq!(amount(&r,"future_value"),"0.00");
    for formula in [json!({"operation":"payment","principal_money":"100","rate_percent_per_period":"1","periods":0,"payment_timing":"end"}),json!({"operation":"rate_from_values","present_money":"100","future_money":"200","periods":0}),json!({"operation":"loan_schedule","principal_money":"100","rate_percent_per_period":"1","periods":0})] {
        let mut input=example("financial_formula");input["formula"]=formula;
        assert!(run_finance("financial_formula",input).is_err());
    }
}
#[test]
fn equivalent_rate_zero_target_is_zero_but_zero_source_is_error() {
    let mut input=example("financial_formula");
    input["formula"]=json!({"operation":"equivalent_rate","rate_percent":"10","source_periods":12,"target_periods":0});
    assert_eq!(run("financial_formula",input.clone())["data"]["equivalent_percent"],"0.00000000");
    input["formula"]["source_periods"]=json!(0);assert!(run_finance("financial_formula",input).is_err());
}
#[test]
fn formulas_reject_overflow_nonfinite_subflow_impossible_rates_and_missing_timing() {
    for formula in [
        json!({"operation":"future_value","principal_money":"1","rate_percent_per_period":"1000000","periods":1200}),
        json!({"operation":"future_value","principal_money":"1","rate_percent_per_period":"-99.99999999","periods":1200}),
        json!({"operation":"future_value","principal_money":"1","rate_percent_per_period":"-100","periods":1}),
        json!({"operation":"future_value","principal_money":"1","rate_percent_per_period":"0","periods":1201}),
        json!({"operation":"future_value","principal_money":"-1","rate_percent_per_period":"1","periods":1}),
        json!({"operation":"payment","principal_money":"1","rate_percent_per_period":"1","periods":1}),
        json!({"operation":"rate_from_values","present_money":"0","future_money":"1","periods":1}),
        json!({"operation":"loan_schedule","principal_money":"100","rate_percent_per_period":"-1","periods":2}),
        json!({"operation":"simple_interest","principal_money":"100","rate_percent_per_period":"-50","periods":3})] {
        let mut input=example("financial_formula");input["formula"]=formula;
        assert!(run_finance("financial_formula",input).is_err());
    }
}
#[test]
fn formula_union_rejects_conflicting_or_unknown_fields() {
    let mut input=example("financial_formula");input["formula"]["future_money"]=json!("999");
    assert!(run_finance("financial_formula",input).is_err());
    let mut input=example("financial_formula");input["formula"]["operation"]=json!("invented");
    assert!(run_finance("financial_formula",input).is_err());
}
#[test]
fn formulas_disclose_approximation_and_never_claim_legal_validation() {
    let r=calc(json!({"operation":"future_value","principal_money":"100","rate_percent_per_period":"1","periods":12}));
    assert_eq!(r["rounding_policy"]["formula_numeric_backend"],"f64_experimental");
    assert_eq!(r["rules_verified"],false);
    assert!(r["assumptions"].as_array().unwrap().iter().any(|v| v.as_str().unwrap().contains("f64")));
}
#[test]
fn negative_formula_rates_are_allowed_only_when_explicit_and_defined() {
    let r=calc(json!({"operation":"future_value","principal_money":"100","rate_percent_per_period":"-10","periods":2}));
    assert_eq!(amount(&r,"future_value"),"81.00");
    assert_eq!(amount(&r,"interest"),"-19.00");
}
#[test]
fn hard_limits_on_input_depth_strings_rows_and_periods_are_fail_closed() {
    let mut input=example("update_values");input["provenance"]=json!("x".repeat(4097));
    assert!(run_finance("update_values",input).is_err());
    let mut nested=json!(null);for _ in 0..18 { nested=json!([nested]); }
    assert!(run_finance("update_values",nested).is_err());
    let input=json!({"rows":vec![json!({});10001]});
    assert!(run_finance("irrf_refund",input).is_err());
    let mut input=example("benefit_adjustment");input["period_rates"]=json!(vec![json!({"month":"2026-01","rate_percent":"1"});1201]);
    assert!(run_finance("benefit_adjustment",input).is_err());
}
#[test]
fn discounts_total_work_limit_is_enforced() {
    let mut input=example("undue_discounts");
    let item=json!({"amount":"1","date":"2026-01-01","months":1200,"correction_factors":vec!["1";1200]});
    input["items"]=json!(vec![item;11]);
    assert!(run_finance("undue_discounts",input).is_err());
}
#[test]
fn unknown_tool_and_errors_do_not_echo_private_or_untrusted_input() {
    assert!(run_finance("pension_accumulation",json!({})).is_err());
    assert!(run_finance("not_financial",json!({})).is_err());
    let mut input=example("update_values");input["principal_money"]=json!("SYNTHETIC_PRIVATE_MARKER");
    let err=run_finance("update_values",input).unwrap_err();
    assert!(!err.contains("SYNTHETIC_PRIVATE_MARKER"));
}
#[test]
fn eight_place_decimal_ratios_are_exact_before_cent_rounding() {
    let mut input=example("currency_conversion");input["amount"]=json!("12345.67");
    input["numerator"]=json!("0.12345678");input["denominator"]=json!("0.12345678");
    assert_eq!(run("currency_conversion",input)["data"]["converted"]["amount"],"12345.67");
}
#[test]
fn maximum_exact_money_identity_is_not_overflow() {
    let mut input=example("currency_conversion");input["amount"]=json!("90000000000000.00");
    input["numerator"]=json!("1");input["denominator"]=json!("1");
    assert_eq!(run("currency_conversion",input)["data"]["converted"]["cents"],9_000_000_000_000_000_i64);
}
#[test]
fn formula_rounding_toward_zero_is_applied_at_output_boundary() {
    let mut input=example("financial_formula");
    input["formula"]=json!({"operation":"future_value","principal_money":"0.01","rate_percent_per_period":"50","periods":1});
    assert_eq!(amount(&run("financial_formula",input.clone()),"future_value"),"0.02");
    input["rounding"]=json!("toward_zero");
    assert_eq!(amount(&run("financial_formula",input),"future_value"),"0.01");
}

#[test]
fn formula_rate_quantization_respects_declared_mode_at_eight_places() {
    let mut input=example("financial_formula");
    input["formula"]=json!({"operation":"equivalent_rate","rate_percent":"1.32","source_periods":1,"target_periods":12});
    let rounded=run("financial_formula",input.clone());
    let rate=rounded["data"]["equivalent_percent"].as_str().unwrap().parse::<f64>().unwrap();
    input["rounding"]=json!("toward_zero");
    let truncated=run("financial_formula",input);
    let lower=truncated["data"]["equivalent_percent"].as_str().unwrap().parse::<f64>().unwrap();
    assert_eq!(rounded["data"]["operation"],"equivalent_rate");
    assert_eq!(rounded["data"]["equivalent_percent"],"17.04211833");
    assert_eq!(truncated["data"]["equivalent_percent"],"17.04211832");
    assert!(rate>=lower);
}
#[test]
fn simple_interest_reports_exact_numeric_backend() {
    let r=calc(json!({"operation":"simple_interest","principal_money":"100","rate_percent_per_period":"2.5","periods":3}));
    assert_eq!(r["rounding_policy"]["formula_numeric_backend"],"checked_i128_ratios");
    assert_eq!(r["data"]["operation"],"simple_interest");
    assert_eq!(amount(&r,"interest"),"7.50");
}
#[test]
fn embedded_irrf_matches_the_published_table() {
    let r = run("irrf_oficial", json!({"competencia":"2026-10","base":"5000.00","dependentes":0,"isencao_65":null,"provenance":"synthetic"}));
    assert_eq!(r["data"]["irrf"], "466.27");
    assert_eq!(r["data"]["aliquota"], "27.50");
    assert_eq!(r["data"]["tabela_competencia"], "2026-10");
    let r = run("irrf_oficial", json!({"competencia":"2026-10","base":"3000.00","dependentes":1,"isencao_65":null,"provenance":"synthetic"}));
    assert_eq!(r["data"]["irrf"], "28.62");
    let r = run("irrf_oficial", json!({"competencia":"2026-10","base":"2000.00","dependentes":0,"isencao_65":null,"provenance":"synthetic"}));
    assert_eq!(r["data"]["irrf"], "0.00");
    assert!(run_finance("irrf_oficial", json!({"competencia":"1994-06","base":"100.00","dependentes":0,"isencao_65":null,"provenance":"synthetic"})).is_err());
}