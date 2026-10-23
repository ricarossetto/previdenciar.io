use previus_core::run_tool;
use serde_json::json;

#[test] fn facade_dates_preserves_experimental_envelope() {
    let result=run_tool("inclusive_dates",json!({"start":"2026-10-10","end":"2026-10-10"})).unwrap();
    assert_eq!(result["status"],"experimental");
    assert_eq!(result["rule_review_status"],"behavior_only");
    assert_eq!(result["data"]["calendar_days"],1);
    assert!(!result["warnings"].as_array().unwrap().is_empty());
}
#[test] fn facade_quality_observation_and_unsupported_branch() {
    let input=json!({"stopped_contributing":true,"currently_receiving_benefit":false,
        "insured_category":"facultative","last_contribution_date":"2024-02-29"});
    let result=run_tool("facultative_quality",input.clone()).unwrap();
    assert_eq!(result["data"]["quality_loss_date"],"2025-02-16");
    let mut unsupported=input; unsupported["insured_category"]=json!("employee");
    let result=run_tool("facultative_quality",unsupported).unwrap();
    assert_eq!(result["status"],"unsupported");
    assert_eq!(result["data"],serde_json::Value::Null);
}
#[test] fn facade_exact_money_and_explicit_base() {
    let result=run_tool("pension_accumulation",json!({"reference_date":"2026-10-10",
        "incomes":[{"currency":"BRL","amount":"5000.00"},{"currency":"BRL","amount":"6000.00"}],
        "reference_base":{"currency":"BRL","amount":"1621.00"},"rounding":"half_away_from_zero"})).unwrap();
    assert_eq!(result["data"]["reduced_total"]["amount"],"3269.40");
    assert_eq!(result["data"]["integral_income"]["amount"],"6000.00");
    assert_eq!(result["status"],"experimental");
}
#[test] fn facade_import_review_returns_no_fake_rmi() {
    let result=run_tool("import_review",json!({"kind":"rmi","records":{
        "salary_months":[],"links":[],"indicators":[],
        "warnings":[{"code":"synthetic_pending","message":"Review required"}]}})).unwrap();
    assert_eq!(result["status"],"unsupported");
    assert_eq!(result["data"],serde_json::Value::Null);
    assert!(result["warnings"].as_array().unwrap().iter().any(|w| w["code"]=="synthetic_pending"));
}
#[test] fn facade_rejects_unknown_fields_missing_inputs_and_wrong_money_types() {
    for input in [json!({"start":"2026-10-10"}),json!({"start":"2026-02-30","end":"2026-10-10"}),
        json!({"start":"2026-10-10","end":"2026-10-10","unknown":true})] {
        assert!(run_tool("inclusive_dates",input).is_err());
    }
    assert!(run_tool("pension_accumulation",json!({"incomes":[1.23]})).is_err());
    assert!(run_tool("rmi",json!({})).is_err());
}
#[test] fn facade_errors_do_not_echo_untrusted_values() {
    let result=run_tool("inclusive_dates",json!({"start":"SYNTHETIC_PRIVATE_VALUE","end":"2026-10-10"})).unwrap_err();
    assert!(!result.contains("SYNTHETIC_PRIVATE_VALUE"));
}
