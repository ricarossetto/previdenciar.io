use previus_core::{domain::{calculate_imported_records, ContributionLink, ImportedRecords, Indicator, Regime, SalaryMonth, UnsupportedCalculationKind},
    pension::{calculate_pension_accumulation, PensionAccumulationInput},
    quality::{calculate_facultative_quality, InsuredCategory, QualityOfInsuredInput},
    CivilDate, DomainError, Money, OutcomeStatus, RoundingMode, RuleReviewStatus, Warning};

fn date() -> CivilDate { "2026-10-10".parse().unwrap() }
fn pension(cents: &[i64], base: i64) -> PensionAccumulationInput {
    PensionAccumulationInput { reference_date:date(), incomes:cents.iter().map(|c| Money::from_cents(*c)).collect(),
        reference_base:Money::from_cents(base),rounding:RoundingMode::HalfAwayFromZero }
}
fn imports() -> ImportedRecords {
    ImportedRecords {
        salary_months:vec![SalaryMonth {
            competence:"2024-02".parse().unwrap(),amount:Some(Money::from_cents(10000)),
            link_id:Some("synthetic-link-1".into()), indicators:vec![],source:None,
        }],
        links:vec![ContributionLink { id:"synthetic-link-1".into(),start:Some("2024-01-01".parse().unwrap()),
            end:Some("2024-12-31".parse().unwrap()),regime:Regime::Rgps,indicators:vec![],source:None }],
        indicators:vec![],warnings:vec![],
    }
}
fn codes(records: &ImportedRecords) -> Vec<String> { records.review_warnings().into_iter().map(|w| w.code).collect() }
#[test] fn negative_income_and_nonpositive_base_are_rejected() {
    assert_eq!(calculate_pension_accumulation(&pension(&[-1,100],100)),Err(DomainError::NegativeAmount));
    for base in [0,-1] { assert_eq!(calculate_pension_accumulation(&pension(&[100],base)),Err(DomainError::InvalidReferenceBase)); }
    assert_eq!(calculate_pension_accumulation(&pension(&[],100)),Err(DomainError::EmptyIncomes));
}
#[test] fn all_zero_incomes_have_no_fake_percentage() {
    let output = calculate_pension_accumulation(&pension(&[0,0],100)).unwrap().data.unwrap();
    assert_eq!(output.combined_total,Money::ZERO);
    assert_eq!(output.integral_income_index,0);
    assert!(output.incomes.iter().all(|i| i.received_percentage.is_none()));
}
#[test] fn equal_incomes_use_first_position_explicitly() {
    let outcome = calculate_pension_accumulation(&pension(&[50000,50000],10000)).unwrap();
    assert!(outcome.warnings.iter().any(|w| w.code == "scope_limit"));
    assert_eq!(outcome.data.unwrap().integral_income_index,0);
}
#[test] fn reference_base_changes_result_and_is_never_defaulted() {
    let a = calculate_pension_accumulation(&pension(&[500000,600000],162100)).unwrap().data.unwrap();
    let b = calculate_pension_accumulation(&pension(&[500000,600000],100000)).unwrap().data.unwrap();
    assert_ne!(a.reduced_total,b.reduced_total);
    assert_eq!(b.reduced_total.cents(),230000);
    assert!(serde_json::from_str::<PensionAccumulationInput>(r#"{"reference_date":"2026-10-10","incomes":[],"rounding":"toward_zero"}"#).is_err());
}
#[test] fn all_five_bands_and_highest_pension_are_supported_experimentally() {
    let result = calculate_pension_accumulation(&pension(&[60000,50000,1000],10000)).unwrap().data.unwrap();
    assert_eq!(result.integral_income_index,0);
    assert_eq!(result.incomes[1].tranches.iter().map(|t| t.received_amount.cents()).collect::<Vec<_>>(),vec![10000,6000,4000,2000,1000]);
    assert_eq!(result.incomes[1].received_amount.cents(),23000);
    assert_eq!(result.incomes[2].received_amount.cents(),1000);
}
#[test] fn band_boundaries_and_no_hidden_float() {
    for amount in 0..=1500 {
        let result = calculate_pension_accumulation(&pension(&[2000,amount],100)).unwrap().data.unwrap();
        let reduced = &result.incomes[1];
        assert_eq!(reduced.tranches.iter().map(|t| t.basis_amount.cents()).sum::<i64>(),amount);
        assert_eq!(reduced.tranches.iter().map(|t| t.received_amount.cents()).sum::<i64>(),reduced.received_amount.cents());
        assert!(reduced.received_amount.cents() <= amount);
        assert_eq!(reduced.tranches.len(),5);
    }
}
#[test] fn rounding_mode_is_carried_in_json() {
    let mut input = pension(&[100,3],1);
    let a = calculate_pension_accumulation(&input).unwrap().data.unwrap();
    input.rounding=RoundingMode::TowardZero;
    let b = calculate_pension_accumulation(&input).unwrap().data.unwrap();
    assert_ne!(a.reduced_total,b.reduced_total);
    let encoded = serde_json::to_value(b).unwrap();
    assert_eq!(encoded["rounding"],"toward_zero");
}
#[test] fn combined_overflow_is_error_never_wrapped_output() {
    assert_eq!(calculate_pension_accumulation(&pension(&[i64::MAX,i64::MAX],100)),Err(DomainError::Overflow));
    // Valid maximum single income does not overflow an intermediate percentage.
    let one=calculate_pension_accumulation(&pension(&[i64::MAX],1)).unwrap().data.unwrap();
    assert_eq!(one.combined_total.cents(),i64::MAX);
}
#[test] fn unsupported_quality_branches_produce_no_dates() {
    for (stopped,benefit,category) in [(false,false,InsuredCategory::Facultative),
        (true,true,InsuredCategory::Facultative),(true,false,InsuredCategory::Employee),
        (true,false,InsuredCategory::IndividualContributor),(true,false,InsuredCategory::Other)] {
        let output=calculate_facultative_quality(&QualityOfInsuredInput {
            stopped_contributing:stopped,currently_receiving_benefit:benefit,insured_category:category,last_contribution_date:date()
        }).unwrap();
        assert_eq!(output.status,OutcomeStatus::Unsupported);
        assert_eq!(output.rule_review_status,RuleReviewStatus::MissingRules);
        assert!(output.data.is_none());
        assert!(!output.warnings.is_empty());
    }
}
#[test] fn facultative_calendar_overflow_is_error() {
    assert_eq!(calculate_facultative_quality(&QualityOfInsuredInput {
        stopped_contributing:true,currently_receiving_benefit:false,insured_category:InsuredCategory::Facultative,
        last_contribution_date:"9999-12-31".parse().unwrap()
    }),Err(DomainError::Overflow));
}
#[test] fn import_preserves_same_month_records_and_unknown_amount() {
    let mut records=imports();
    let mut second=records.salary_months[0].clone(); second.amount=None;
    records.salary_months.push(second);
    let snapshot=records.clone();
    assert_eq!(records.distinct_salary_months(),1);
    assert_eq!(records.salary_months.len(),2);
    let warnings=codes(&records);
    assert!(warnings.contains(&"repeated_competence".to_owned()));
    assert!(warnings.contains(&"missing_salary".to_owned()));
    assert_eq!(records,snapshot);
}
#[test] fn prpps_rpps_indicators_are_never_auto_resolved() {
    let mut records=imports();
    records.links[0].regime=Regime::Rpps;
    records.links[0].indicators.push(Indicator {code:"PRPPS".into(),unresolved:true});
    let snapshot=records.clone();
    let output=calculate_imported_records(&records,UnsupportedCalculationKind::Rmi);
    for code in ["rpps_review_required","prpps_preserved","unresolved_indicator","missing_rules"] {
        assert!(output.warnings.iter().any(|w| w.code==code));
    }
    assert_eq!(records,snapshot);
    assert!(output.data.is_none());
}
#[test] fn domain_review_warns_without_fixing_unknown_records() {
    let mut records=imports();
    records.links[0].regime=Regime::Unknown;
    records.links[0].end=Some("2023-12-31".parse().unwrap());
    records.links.push(records.links[0].clone());
    records.salary_months[0].link_id=Some("missing-synthetic-link".into());
    records.salary_months[0].amount=Some(Money::from_cents(-1));
    for code in ["unknown_regime","reversed_link","duplicate_link_id","negative_salary","unknown_salary_link"] {
        assert!(codes(&records).contains(&code.to_owned()),"missing {code}");
    }
}
#[test] fn empty_identifier_and_missing_dates_are_visible() {
    let mut records=imports();
    records.links[0].id=String::new(); records.links[0].start=None; records.links[0].end=None;
    records.salary_months[0].link_id=None;
    for code in ["missing_link_id","incomplete_link_dates","missing_salary_link"] {
        assert!(codes(&records).contains(&code.to_owned()));
    }
}
#[test] fn entitlement_rmi_ctc_time_are_all_unsupported_preserving_import_warnings() {
    let mut records=imports();
    records.warnings.push(Warning::new("synthetic_parser_warning","Synthetic unresolved record"));
    for kind in [UnsupportedCalculationKind::BenefitEntitlement,UnsupportedCalculationKind::Rmi,
        UnsupportedCalculationKind::Ctc,UnsupportedCalculationKind::ContributionTime] {
        let output=calculate_imported_records(&records,kind);
        assert_eq!(output.status,OutcomeStatus::Unsupported);
        assert_eq!(output.rule_review_status,RuleReviewStatus::MissingRules);
        assert!(output.data.is_none());
        assert!(output.warnings.contains(&records.warnings[0]));
        assert_eq!(serde_json::to_value(output).unwrap()["data"],serde_json::Value::Null);
    }
}
#[test] fn import_json_round_trip_keeps_all_raw_indicators() {
    let mut records=imports();
    records.indicators.push(Indicator{code:"SYNTHETIC_PENDING".into(),unresolved:true});
    let encoded=serde_json::to_string(&records).unwrap();
    assert_eq!(serde_json::from_str::<ImportedRecords>(&encoded).unwrap(),records);
}
#[test] fn json_outputs_round_trip_with_status_and_warnings() {
    let output=calculate_pension_accumulation(&pension(&[500000,600000],162100)).unwrap();
    let encoded=serde_json::to_value(&output).unwrap();
    assert_eq!(encoded["status"],"experimental");
    assert_eq!(encoded["rule_review_status"],"behavior_only");
    assert_eq!(encoded["data"]["incomes"][0]["received_amount"]["amount"],"3269.40");
    assert!(!encoded["warnings"].as_array().unwrap().is_empty());
    assert_eq!(serde_json::from_value::<previus_core::CalculationOutcome<previus_core::pension::PensionAccumulationOutput>>(encoded).unwrap(),output);
}
