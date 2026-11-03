use previdenciar_cnis::{
    AmountKind, EmploymentLink, SalaryEntry, SalaryMonth, SourceRef, ValidationStatus,
};
use previdenciar_rgps::prepare_from_cnis;
use previdenciar_workspace::Calculation;

fn base() -> previdenciar_cnis::ImportResult {
    let mut cnis = previdenciar_cnis::import_text(
        "CNIS\nCompetência Remuneração Indicadores\n01/2020 2.000,00\n",
    )
    .unwrap();
    cnis.links.push(EmploymentLink {
        sequence: 1,
        raw_employer: Some("Empregador A".into()),
        affiliation: Some("Empregado ou Agente Público".into()),
        start_date: Some("2020-01-01".into()),
        end_date: None,
        reported_last_salary_month: None,
        inferred_last_salary_end: None,
        indicators: vec!["IREM-INDPEND".into()],
        prpps: false,
        validation_status: ValidationStatus::Unvalidated,
        excluded_from_validated_domain: false,
        source: SourceRef {
            page: 1,
            line: 1,
            index: 1,
            byte_offset: 0,
        },
    });
    cnis.links.push(EmploymentLink {
        sequence: 2,
        raw_employer: Some("Regime próprio".into()),
        affiliation: Some("Servidor".into()),
        start_date: Some("2015-01-01".into()),
        end_date: None,
        reported_last_salary_month: None,
        inferred_last_salary_end: None,
        indicators: vec!["PRPPS".into()],
        prpps: true,
        validation_status: ValidationStatus::Unvalidated,
        excluded_from_validated_domain: false,
        source: SourceRef {
            page: 1,
            line: 2,
            index: 2,
            byte_offset: 0,
        },
    });
    cnis.salary_entries[0].link_sequence = Some(1);
    cnis.salary_entries.push(SalaryEntry {
        competence: "2020-02".into(),
        amount_centavos: 300_000,
        indicators: vec!["PRPPS".into()],
        source: SourceRef {
            page: 1,
            line: 3,
            index: 2,
            byte_offset: 0,
        },
        link_sequence: Some(2),
        amount_kind: AmountKind::Remuneration,
        validation_status: ValidationStatus::Unvalidated,
        excluded_from_validated_domain: false,
    });
    cnis.salary_months.insert(
        "2020-02".into(),
        SalaryMonth {
            count: 1,
            amount_sum_centavos: 300_000,
            entry_indices: vec![2],
            contains_unresolved_prpps: true,
        },
    );
    cnis
}

fn case() -> Calculation {
    let cnis = base();
    Calculation {
        identity: previdenciar_cnis::Identity {
            birth_date: Some("1960-01-01".into()),
            ..cnis.identity.clone()
        },
        cnis: Some(cnis),
        sex: Some("feminino".into()),
        dib: "2021-01-31".into(),
        kind: "RGPS".into(),
        ..Default::default()
    }
}

fn add_salary(
    cnis: &mut previdenciar_cnis::ImportResult,
    competence: &str,
    amount_centavos: i64,
    index: usize,
) {
    cnis.salary_entries.push(SalaryEntry {
        competence: competence.into(),
        amount_centavos,
        indicators: vec![],
        source: SourceRef {
            page: 1,
            line: 3 + index,
            index,
            byte_offset: 0,
        },
        link_sequence: Some(1),
        amount_kind: AmountKind::Remuneration,
        validation_status: ValidationStatus::Unvalidated,
        excluded_from_validated_domain: false,
    });
    cnis.salary_months.insert(
        competence.into(),
        SalaryMonth {
            count: 1,
            amount_sum_centavos: amount_centavos,
            entry_indices: vec![index],
            contains_unresolved_prpps: false,
        },
    );
}

fn confirm_prepared(mut case: Calculation) -> Calculation {
    case.disability = Some("nao".into());
    for link in &mut case.manual_links {
        link.confirmed = true;
        link.indicators.clear();
        if let Some(review) = &mut link.source_review {
            if let Some(last) = review.history.last_mut() {
                last.after.confirmed = true;
                last.after.indicators.clear();
            }
        }
    }
    for wage in &mut case.manual_salaries {
        wage.confirmed = true;
        wage.indicators.clear();
    }
    case
}

#[test]
fn guided_preparation_excludes_prpps_and_keeps_review_pending() {
    let case = case();
    let before = serde_json::to_value(&case.cnis).unwrap();
    let (prepared, notes) = prepare_from_cnis(&case, "CNIS conferido em 2026-10-23").unwrap();
    assert_eq!(prepared.manual_links.len(), 1);
    assert_eq!(prepared.manual_links[0].end_date, "2020-01-31");
    assert_eq!(prepared.manual_links[0].regime, "RGPS");
    assert!(
        !prepared.manual_links[0].confirmed,
        "preparação não pode confirmar automaticamente"
    );
    assert_eq!(prepared.manual_links[0].indicators, "IREM-INDPEND");
    assert_eq!(prepared.manual_salaries.len(), 1);
    assert_eq!(prepared.manual_salaries[0].competence, "2020-01");
    assert!(
        !prepared.manual_salaries[0].confirmed,
        "salário não pode ser confirmado automaticamente"
    );
    assert!(
        prepared.disability.is_none(),
        "deficiência não pode ser inferida"
    );
    assert!(notes.iter().any(|note| note.contains("PRPPS")));
    assert!(notes
        .iter()
        .any(|note| note.contains("revisão documental permanece pendente")));
    assert!(notes
        .iter()
        .any(|note| note.contains("Condição de deficiência não declarada")));
    assert!(prepared
        .options
        .rule_policy
        .contains("\"exclude_imported_source\":true"));
    assert_eq!(before, serde_json::to_value(&prepared.cnis).unwrap());
}

#[test]
fn guided_explicit_no_disability_is_preserved_without_note() {
    let mut case = case();
    case.disability = Some("Sem deficiência declarada".into());
    let (prepared, notes) = prepare_from_cnis(&case, "CNIS conferido em 2026-10-23").unwrap();
    assert_eq!(
        prepared.disability.as_deref(),
        Some("Sem deficiência declarada")
    );
    assert!(!notes
        .iter()
        .any(|note| note.contains("Condição de deficiência não declarada")));
}

#[test]
fn guided_preparation_pending_never_produces_financial_result() {
    let (prepared, _) = prepare_from_cnis(&case(), "CNIS conferido em 2026-10-23").unwrap();
    let result = previdenciar_rgps::calculate_case(&prepared).unwrap();
    assert_eq!(result["status"], "blocked", "{result}");
    assert!(result["data"].is_null());
    let missing = result["missing"].as_array().unwrap();
    for code in [
        "link_unconfirmed",
        "wage_unconfirmed",
        "link_indicators_unresolved",
        "disability_unconfirmed",
    ] {
        assert!(
            missing.iter().any(|value| value == code),
            "missing {code}: {result}"
        );
    }
}

#[test]
fn guided_preparation_after_explicit_confirmation_reaches_engine() {
    let (prepared, _) = prepare_from_cnis(&case(), "CNIS conferido em 2026-10-23").unwrap();
    let result = previdenciar_rgps::calculate_case(&confirm_prepared(prepared)).unwrap();
    assert_eq!(result["status"], "experimental", "{result}");
    assert!(
        result["data"]["native_result"]["resumo"]["rmi"].is_string(),
        "{result}"
    );
}

#[test]
fn guided_preparation_never_invents_interview_answers_or_review() {
    let mut case = case();
    case.assistant = Some(previdenciar_workspace::AssistantInterview {
        mode: "assisted".into(),
        ..Default::default()
    });
    let (prepared, notes) = prepare_from_cnis(&case, "CNIS conferido em 2026-10-23").unwrap();
    let assistant = prepared.assistant.unwrap();
    assert_eq!(assistant.current_benefit_received, None);
    assert_eq!(assistant.process_kind, None);
    assert_eq!(assistant.goal, None);
    assert!(!assistant.review_confirmed);
    assert!(
        notes.iter().any(|note| note.contains("continua pendente")),
        "{notes:?}"
    );
}

#[test]
fn guided_preparation_clamps_end_at_dib_and_preserves_original() {
    let mut case = case();
    case.dib = "2023-06-30".into();
    case.cnis.as_mut().unwrap().links[0].end_date = Some("2024-07-31".into());
    let source_before = serde_json::to_value(&case.cnis).unwrap();
    let (prepared, notes) = prepare_from_cnis(&case, "CNIS conferido em 2026-10-23").unwrap();
    assert_eq!(prepared.manual_links[0].end_date, "2023-06-30");
    assert!(!prepared.manual_links[0].confirmed);
    let history = &prepared.manual_links[0]
        .source_review
        .as_ref()
        .unwrap()
        .history[0];
    assert_eq!(history.before.end_date, "2024-07-31");
    assert_eq!(history.after.end_date, "2023-06-30");
    assert!(notes
        .iter()
        .any(|note| note.contains("limitados a 2023-06-30")));
    assert_eq!(source_before, serde_json::to_value(&prepared.cnis).unwrap());
}

#[test]
fn guided_preparation_excludes_link_starting_after_dib() {
    let mut case = case();
    case.dib = "2023-06-30".into();
    case.cnis.as_mut().unwrap().links[0].start_date = Some("2024-09-01".into());
    case.cnis.as_mut().unwrap().links[0].end_date = Some("2026-06-30".into());
    let source_before = serde_json::to_value(&case.cnis).unwrap();
    let (prepared, notes) = prepare_from_cnis(&case, "CNIS conferido em 2026-10-23").unwrap();
    let review = prepared.manual_links[0].source_review.as_ref().unwrap();
    assert!(review.excluded);
    assert!(!prepared.manual_links[0].confirmed);
    assert!(notes
        .iter()
        .any(|note| note.contains("iniciados após a DIB")));
    assert_eq!(source_before, serde_json::to_value(&prepared.cnis).unwrap());
}

#[test]
fn guided_preparation_uses_pbc_end_before_dib() {
    let mut case = case();
    case.dib = "2023-06-30".into();
    let cnis = case.cnis.as_mut().unwrap();
    cnis.links[0].end_date = Some("2024-07-31".into());
    add_salary(cnis, "2023-05", 200_000, 3);
    add_salary(cnis, "2023-06", 210_000, 4);
    add_salary(cnis, "2023-07", 220_000, 5);
    let source_before = serde_json::to_value(&case.cnis).unwrap();
    let (prepared, notes) = prepare_from_cnis(&case, "CNIS conferido em 2026-10-23").unwrap();
    let competences: Vec<_> = prepared
        .manual_salaries
        .iter()
        .map(|wage| wage.competence.as_str())
        .collect();
    assert!(competences.contains(&"2023-05"), "{competences:?}");
    assert!(!competences.contains(&"2023-06"), "{competences:?}");
    assert!(!competences.contains(&"2023-07"), "{competences:?}");
    assert!(notes.iter().any(|note| note.contains("posteriores ao PBC")));
    assert_eq!(source_before, serde_json::to_value(&prepared.cnis).unwrap());
}

#[test]
fn guided_preparation_same_dib_date_is_included_in_time() {
    let mut case = case();
    case.dib = "2023-06-30".into();
    case.cnis.as_mut().unwrap().links[0].end_date = Some("2023-06-30".into());
    let (prepared, _) = prepare_from_cnis(&case, "CNIS conferido em 2026-10-23").unwrap();
    assert_eq!(prepared.manual_links[0].end_date, "2023-06-30");
    assert!(
        !prepared.manual_links[0]
            .source_review
            .as_ref()
            .unwrap()
            .excluded
    );
}

#[test]
fn guided_real_case_analogue_loses_no_false_pbc_blockers() {
    let mut case = case();
    case.dib = "2023-06-30".into();
    let cnis = case.cnis.as_mut().unwrap();
    cnis.links[0].end_date = Some("2024-07-31".into());
    add_salary(cnis, "2023-05", 200_000, 3);
    add_salary(cnis, "2023-06", 210_000, 4);
    add_salary(cnis, "2023-07", 220_000, 5);
    let (prepared, _) = prepare_from_cnis(&case, "CNIS conferido em 2026-10-23").unwrap();
    assert_eq!(prepared.manual_links[0].end_date, "2023-06-30");
    assert!(!prepared
        .manual_salaries
        .iter()
        .any(|wage| wage.competence == "2023-06" || wage.competence == "2023-07"));
    let result = previdenciar_rgps::calculate_case(&prepared).unwrap();
    assert_eq!(result["status"], "blocked", "{result}");
    let missing = result["missing"].as_array().unwrap();
    for code in [
        "source_end_after_dib",
        "wage_after_dib",
        "wage_in_dib_month",
    ] {
        assert!(
            !missing.iter().any(|value| value == code),
            "o período efetivo não deve reintroduzir {code}: {result}"
        );
    }
}
