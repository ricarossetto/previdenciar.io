use previus_rgps::{calculate_json,calculate_case,capabilities,POLICY_VERSION};
use serde_json::{json,Value};
use previus_workspace::{Calculation,ManualLink,ManualSalary};
fn fixture()->Value { serde_json::from_str(include_str!("fixtures/synthetic-300-months.json")).unwrap() }
fn evaluate(v:Value)->Value { calculate_json(v).expect("well-formed input") }
fn blocker(v:Value,code:&str) {
    let r=evaluate(v);
    assert_eq!(r["status"],"blocked","{r}");
    assert!(r["data"].is_null());
    assert!(r["missing"].as_array().unwrap().iter().any(|v|v==code),"{r}");
}
#[test] fn positive_native_anchor_300_months() {
    let r=evaluate(fixture());
    assert_eq!(r["status"],"experimental");
    assert_eq!(r["data"]["native_result"]["resumo"]["rmi"],"1600.00");
    assert_eq!(r["data"]["native_result"]["resumo"]["salario_beneficio"],"2000.00");
    assert_eq!(r["data"]["native_result"]["resumo"]["carencia_meses"],300);
    assert!(r["data"]["native_result"]["transicoes"]["regras"].is_array());
    assert_eq!(r["legal_validation_performed"],false);
    assert!(r["data"]["native_result"]["segurado"]["nome"].is_null());
}
#[test] fn positive_corrected_fixture_uses_native_tables() {
    let v=serde_json::from_str(include_str!("fixtures/synthetic-corrected.json")).unwrap();
    let r=evaluate(v);
    assert_eq!(r["status"],"experimental","{r}");
    assert_eq!(r["data"]["native_result"]["correcao"]["parcelas"].as_array().unwrap().len(),12);
    assert_eq!(r["data"]["native_result"]["correcao"]["referencia"],"2025-01");
    assert!(r["data"]["native_result"]["resumo"]["rmi"].as_str().unwrap().parse::<f64>().unwrap()>0.0);
}
#[test] fn missing_data_is_listed_never_zero() {
    let r=evaluate(json!({}));assert_eq!(r["status"],"blocked");assert!(r["data"].is_null());assert!(r["missing"].as_array().unwrap().len()>6);
}
#[test] fn missing_sex_blocks() {let mut v=fixture();v["segurado"]["sexo"]=Value::Null;blocker(v,"sex_unconfirmed");}
#[test] fn missing_disability_blocks() {let mut v=fixture();v["segurado"]["deficiencia"]=Value::Null;blocker(v,"disability_unconfirmed");}
#[test] fn disability_is_not_assumed_absent() {let mut v=fixture();v["segurado"]["deficiencia"]=json!("leve");blocker(v,"disability_rules_unsupported");}
#[test] fn dib_required_no_today_fallback() {let mut v=fixture();v["dataReferencia"]=Value::Null;blocker(v,"missing_or_invalid_date");}
#[test] fn historical_dib_policy_blocks() {let mut v=fixture();v["dataReferencia"]=json!("2019-11-12");blocker(v,"historical_policy_unsupported");}
#[test] fn source_end_is_required_no_last_wage_guess() {let mut v=fixture();v["vinculos"][0]["fim"]=Value::Null;blocker(v,"missing_or_invalid_date");}
#[test] fn source_end_after_dib_blocks() {let mut v=fixture();v["vinculos"][0]["fim"]=json!("2026-10-10");blocker(v,"source_end_after_dib");}
#[test] fn reversed_link_blocks() {let mut v=fixture();v["vinculos"][0]["inicio"]=json!("2020-01-01");blocker(v,"link_dates_reversed");}
#[test] fn prpps_source_not_default_rgps() {let mut v=fixture();v["vinculos"][0]["prpps"]=json!(true);blocker(v,"prpps_unvalidated");}
#[test] fn rpps_not_confirmed_by_generic_flag() {let mut v=fixture();v["vinculos"][0]["regime"]=json!("RPPS");blocker(v,"regime_unvalidated");}
#[test] fn ctc_requires_document_and_manual_origin() {let mut v=fixture();v["vinculos"][0]["regime"]=json!("RPPS");v["vinculos"][0]["ctc_confirmada"]=json!(true);blocker(v,"prpps_unvalidated");}
#[test] fn documented_manual_ctc_is_explicit_observation() {let mut v=fixture();v["vinculos"][0]["regime"]=json!("RPPS");v["vinculos"][0]["ctc_confirmada"]=json!(true);v["vinculos"][0]["ctc_evidencia"]=json!("synthetic-ctc/v1");let r=evaluate(v);assert_eq!(r["status"],"experimental");assert!(r["warnings"].as_array().unwrap().iter().any(|w|w.as_str().unwrap().contains("CTC")));}
#[test] fn undocumented_category_blocks() {let mut v=fixture();v["vinculos"][0]["tipo"]=Value::Null;blocker(v,"link_kind_unsupported");}
#[test] fn special_category_not_invented() {let mut v=fixture();v["vinculos"][0]["tipo"]=json!("especial");blocker(v,"link_kind_unsupported");}
#[test] fn conversion_factor_cannot_default_or_invent() {let mut v=fixture();v["vinculos"][0]["fator"]=json!(1.4);blocker(v,"conversion_factor_unsupported");}
#[test] fn wage_confirmation_not_default_true() {let mut v=fixture();v["competencias"][0]["confirmada"]=json!(false);blocker(v,"wage_unconfirmed");}
#[test] fn source_indicator_blocks() {let mut v=fixture();v["competencias"][0]["indicadores"]=json!(["PREC-MENOR-MIN"]);blocker(v,"wage_indicators_unresolved");}
#[test] fn link_indicator_blocks() {let mut v=fixture();v["vinculos"][0]["indicadores"]=json!(["PRPPS"]);blocker(v,"link_indicators_unresolved");}
#[test] fn currency_must_be_explicit() {let mut v=fixture();v["competencias"][0]["moeda"]=Value::Null;blocker(v,"currency_unverified");}
#[test] fn monetary_json_float_rejected() {let mut v=fixture();v["competencias"][0]["salario"]=json!(1234.56);assert!(calculate_json(v).is_err());}
#[test] fn localized_money_not_silently_rounded() {let mut v=fixture();v["competencias"][0]["salario"]=json!("1.234,56");blocker(v,"invalid_amount");}
#[test] fn negative_zero_and_huge_money_block() {for s in ["-1.00","0.00","1000000000000000000000000000000"]{let mut v=fixture();v["competencias"][0]["salario"]=json!(s);blocker(v,"invalid_amount");}}
#[test] fn scientific_and_fractional_cent_values_block() {for s in ["1e3","1.005"]{let mut v=fixture();v["competencias"][0]["salario"]=json!(s);blocker(v,"invalid_amount");}}
#[test] fn duplicate_month_cannot_auto_sum() {let mut v=fixture();v["competencias"][1]["competencia"]=v["competencias"][0]["competencia"].clone();blocker(v,"duplicate_month");}
#[test] fn unicode_date_never_panics() {let mut v=fixture();v["dataReferencia"]=json!("éé-00-00");blocker(v,"missing_or_invalid_date");}
#[test] fn leap_invalid_date_blocked() {let mut v=fixture();v["dataReferencia"]=json!("2025-02-29");blocker(v,"missing_or_invalid_date");}
#[test] fn missing_explicit_option_blocks() {let mut v=fixture();v["opcoes"].as_object_mut().unwrap().remove("descarte");blocker(v,"option_not_selected");}
#[test] fn missing_review_blocks() {let mut v=fixture();v["revisao"]=json!({});blocker(v,"review_not_confirmed");}
#[test] fn uncorrected_discard_is_not_ignored() {let mut v=fixture();v["opcoes"]["descarte"]=json!(true);blocker(v,"uncorrected_discard_unsupported");}
#[test] fn corrected_minimum_divisor_is_not_ignored() {let mut v=fixture();v["opcoes"]["corrigirSalarios"]=json!(true);v["opcoes"]["divisorMinimo"]=json!(true);v["opcoes"]["referenciaCorrecao"]=json!("2025-01");blocker(v,"corrected_divisor_unsupported");}
#[test] fn future_inpc_never_extrapolated_zero() {let mut v=fixture();v["dataReferencia"]=json!("2026-10-10");v["opcoes"]["corrigirSalarios"]=json!(true);v["opcoes"]["referenciaCorrecao"]=json!("2026-10");blocker(v,"inpc_out_of_coverage");}
#[test] fn future_ceiling_never_rolls_last_value() {let mut v=fixture();v["dataReferencia"]=json!("2027-01-01");v["opcoes"]["aplicarTetoVigenteDer"]=json!(true);blocker(v,"ceiling_out_of_coverage");}
#[test] fn inverted_floor_ceiling_blocks() {let mut v=fixture();v["opcoes"]["teto"]=json!("2000.00");v["opcoes"]["piso"]=json!("3000.00");blocker(v,"inverted_limits");}
#[test] fn derived_and_manual_ceiling_conflict_blocks() {let mut v=fixture();v["opcoes"]["aplicarTetoVigenteDer"]=json!(true);v["opcoes"]["teto"]=json!("2000.00");blocker(v,"conflicting_ceiling");}
#[test] fn grace_and_late_payments_do_not_generate_fake_carencia() {let mut v=fixture();v["opcoes"]["graca"]=json!([{"inicio":"2020-01-01","fim":"2020-12-31"}]);blocker(v,"carencia_extras_unsupported");}
#[test] fn ignored_rule_target_blocks() {let mut v=fixture();v["opcoes"]["regraAlvo"]=json!("pedagio-50");blocker(v,"rule_target_unsupported");}
#[test] fn old_currencies_not_sum_in_pbc() {let mut v=fixture();v["competencias"].as_array_mut().unwrap().push(json!({"competencia":"1993-01","salario":"100000.00","moeda":"CR$","confirmada":false}));let r=evaluate(v);assert_eq!(r["status"],"experimental");assert_eq!(r["data"]["pre_1994_wages_excluded"],1);assert_eq!(r["data"]["native_result"]["resumo"]["rmi"],"1600.00");}
#[test] fn embedded_tables_really_referenced() {let c=capabilities();assert_eq!(c["tables"]["inpc"]["count"],387);assert_eq!(c["tables"]["inpc"]["last"],"2026-09");assert_eq!(c["tables"]["ceilings"]["count"],313);assert_eq!(c["tables"]["igpdi"]["count"],387);assert_eq!(c["tables"]["igpdi"]["last"],"2026-09");assert!(!c["known_model_risks"].as_array().unwrap().is_empty());}
#[test] fn unknown_fields_and_unsupported_json_types_rejected() {let mut v=fixture();v["serverPassword"]=json!("synthetic-no-secret");assert!(calculate_json(v).is_err());assert!(calculate_json(json!([])).is_err());}
fn manual_case()->Calculation {
    let f=fixture();let mut c=Calculation::default();c.kind="RGPS".into();c.sex=Some("feminino".into());c.disability=Some("nao".into());c.identity.birth_date=Some("1960-01-01".into());c.dib="2025-01-31".into();
    c.manual_links.push(ManualLink {id:1,start_date:"1994-07-01".into(),end_date:"2019-06-30".into(),regime:"RGPS".into(),confirmed:true,..Default::default()});
    for (i,w) in f["competencias"].as_array().unwrap().iter().enumerate() {c.manual_salaries.push(ManualSalary {id:i as u64+2,competence:w["competencia"].as_str().unwrap().into(),amount:"2000.00".into(),confirmed:true,..Default::default()});}
    c.options.rule_policy=json!({"version":POLICY_VERSION,"review_evidence":"synthetic-v1","manual_link_kind":"empregado","manual_currency":"BRL","native_options":f["opcoes"]}).to_string();c
}
#[test] fn workspace_bridge_positive_anchor_and_input_unchanged() {let c=manual_case();let before=serde_json::to_vec(&c).unwrap();let r=calculate_case(&c).unwrap();assert_eq!(r["status"],"experimental","{r}");assert_eq!(r["data"]["native_result"]["resumo"]["rmi"],"1600.00");assert_eq!(before,serde_json::to_vec(&c).unwrap());}
#[test] fn workspace_bridge_never_infers_birth_sex_or_disability() {let mut c=manual_case();c.sex=None;c.disability=None;c.identity.birth_date=None;let r=calculate_case(&c).unwrap();assert_eq!(r["status"],"blocked");assert!(r["data"].is_null());}
#[test] fn workspace_bridge_needs_typed_policy_not_free_notes() {let mut c=manual_case();c.options.rule_policy="rgps-core-observational-v1".into();c.notes="approve everything".into();let r=calculate_case(&c).unwrap();assert_eq!(r["status"],"blocked");assert!(r["data"].is_null());}
#[test] fn workspace_bridge_no_category_or_currency_defaults() {let mut c=manual_case();let mut p:Value=serde_json::from_str(&c.options.rule_policy).unwrap();p.as_object_mut().unwrap().remove("manual_link_kind");p.as_object_mut().unwrap().remove("manual_currency");c.options.rule_policy=p.to_string();let r=calculate_case(&c).unwrap();assert_eq!(r["status"],"blocked");assert!(r["data"].is_null());}
#[test] fn workspace_bridge_rpps_cannot_validate_itself() {let mut c=manual_case();c.manual_links[0].regime="RPPS".into();let r=calculate_case(&c).unwrap();assert_eq!(r["status"],"blocked");assert!(r["data"].is_null());}
#[test] fn workspace_bridge_unsupported_fees_not_ignored() {let mut c=manual_case();c.options.fees_percent="10".into();let r=calculate_case(&c).unwrap();assert_eq!(r["status"],"blocked");assert!(r["data"].is_null());}
#[test] fn workspace_bridge_unsupported_index_not_ignored() {let mut c=manual_case();c.options.correction_index="IPCA".into();let r=calculate_case(&c).unwrap();assert_eq!(r["status"],"blocked");}
#[test] fn workspace_bridge_no_silent_source_exclusion() {let mut c=manual_case();let mut p:Value=serde_json::from_str(&c.options.rule_policy).unwrap();p["exclude_imported_source"]=json!(true);c.options.rule_policy=p.to_string();let r=calculate_case(&c).unwrap();assert_eq!(r["status"],"blocked");}
#[test] fn workspace_invalid_policy_json_is_error() {let mut c=manual_case();c.options.rule_policy="{invalid}".into();assert!(calculate_case(&c).is_err());}
#[test] fn workspace_unsupported_kind_is_not_rgps_result() {let mut c=manual_case();c.kind="RPPS".into();let r=calculate_case(&c).unwrap();assert_eq!(r["status"],"blocked");assert!(r["data"].is_null());}

fn imported_case()->Calculation {
    let mut c=manual_case();
    c.cnis=Some(serde_json::from_value(json!({
        "metadata":{"extractor":"synthetic-only","source_sha256":null,"page_count":1,"parsed_pages":1,"legal_validation_performed":false},
        "identity":{},"salary_entries":[],"salary_months":{},
        "links":[{"sequence":1,"raw_employer":null,"affiliation":null,"start_date":"2000-01-01","end_date":null,"reported_last_salary_month":null,"inferred_last_salary_end":null,"indicators":["PRPPS"],"prpps":true,"validation_status":"unresolved_prpps","excluded_from_validated_domain":true,"source":{"page":1,"line":1,"index":1,"byte_offset":0}}],
        "benefits":[],"findings":[],"warnings":[]
    })).unwrap());c
}
#[test] fn workspace_source_prpps_and_missing_end_remain_blocked() {let c=imported_case();let before=serde_json::to_vec(&c).unwrap();let r=calculate_case(&c).unwrap();assert_eq!(r["status"],"blocked");assert!(r["data"].is_null());for code in ["source_review_required","prpps_unvalidated","source_end_missing"]{assert!(r["missing"].as_array().unwrap().contains(&json!(code)));}assert_eq!(before,serde_json::to_vec(&c).unwrap());}
#[test] fn source_exclusion_needs_explicit_document_and_uses_only_manual_rows() {let mut c=imported_case();let mut p:Value=serde_json::from_str(&c.options.rule_policy).unwrap();p["exclude_imported_source"]=json!(true);p["source_exclusion_evidence"]=json!("synthetic-source-review-v1");c.options.rule_policy=p.to_string();let before=serde_json::to_vec(&c).unwrap();let r=calculate_case(&c).unwrap();assert_eq!(r["status"],"experimental","{r}");assert_eq!(r["data"]["native_result"]["resumo"]["competencias"],300);assert_eq!(before,serde_json::to_vec(&c).unwrap());assert!(c.cnis.as_ref().unwrap().links[0].prpps);assert!(c.cnis.as_ref().unwrap().links[0].end_date.is_none());}
#[test] fn missing_expected_survival_does_not_manufacture_ibge_number() {let r=evaluate(fixture());assert!(r["warnings"].as_array().unwrap().iter().any(|w|w.as_str().unwrap().contains("IBGE")));}
#[test] fn correction_reference_cannot_differ_from_dib() {let mut v=fixture();v["opcoes"]["corrigirSalarios"]=json!(true);v["opcoes"]["referenciaCorrecao"]=json!("2024-12");blocker(v,"correction_reference_mismatch");}
#[test] fn wage_without_link_blocks() {let mut v=fixture();v["vinculos"][0]["inicio"]=json!("1994-08-01");blocker(v,"wage_outside_confirmed_links");}

#[test] fn native_fator_path_runs_with_corrected_formula() {let mut v=fixture();v["opcoes"]["expectativaSobrevida"]=json!("20.00");let r=evaluate(v);assert_eq!(r["status"],"experimental","{r}");assert!(r["data"]["native_result"]["transicoes"]["regras"].is_array());}
#[test] fn all_public_json_values_match_native_constants_byte_for_byte() {let t:Value=serde_json::from_str(previus_rgps::PUBLIC_ECONOMIC_JSON).unwrap();for (index,(month,value)) in rgps_core::dados::inpc::INPC_MENSAL.iter().enumerate(){assert_eq!(t["inpc"]["values"][index]["period"],*month);assert_eq!(t["inpc"]["values"][index]["value"],*value);}for (index,(month,value)) in rgps_core::dados::igpdi::IGPDI_MENSAL.iter().enumerate(){assert_eq!(t["igpdi"]["values"][index]["period"],*month);assert_eq!(t["igpdi"]["values"][index]["value"],*value);}for (index,(month,value)) in rgps_core::dados::tetos::TETOS_CONTRIBUICAO.iter().enumerate(){assert_eq!(t["ceilings"]["values"][index]["period"],*month);assert_eq!(t["ceilings"]["values"][index]["value"],*value);}}

#[test] fn overlapping_links_known_native_marker_issue_is_blocked() {let mut v=fixture();let l=v["vinculos"][0].clone();v["vinculos"].as_array_mut().unwrap().push(l);blocker(v,"overlapping_links_model_unvalidated");}

#[test] fn public_policy_template_never_auto_confirms_operator_review() {let p=previus_rgps::required_policy_example();assert_eq!(p["version"],POLICY_VERSION);assert!(p["review_evidence"].is_null());assert!(p["manual_link_kind"].is_null());assert!(p["manual_currency"].is_null());assert_eq!(p["exclude_imported_source"],false);assert!(p["native_options"]["corrigirSalarios"].is_null());}

#[test] fn documented_calculation_json_example_is_a_real_working_bridge_fixture() {let c:Calculation=serde_json::from_str(include_str!("fixtures/calculation-synthetic.json")).unwrap();let r=calculate_case(&c).unwrap();assert_eq!(r["status"],"experimental","{r}");assert_eq!(r["data"]["native_result"]["resumo"]["rmi"],"1600.00");}
