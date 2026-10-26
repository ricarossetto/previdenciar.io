use previus_cnis::{AmountKind, EmploymentLink, SalaryEntry, SalaryMonth, SourceRef, ValidationStatus};
use previus_rgps::prepare_from_cnis;
use previus_workspace::Calculation;

fn base() -> previus_cnis::ImportResult {
    let mut cnis = previus_cnis::import_text("CNIS\nCompetência Remuneração Indicadores\n01/2020 2.000,00\n").unwrap();
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
        source: SourceRef { page: 1, line: 1, index: 1, byte_offset: 0 },
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
        source: SourceRef { page: 1, line: 2, index: 2, byte_offset: 0 },
    });
    cnis.salary_entries[0].link_sequence = Some(1);
    cnis.salary_entries.push(SalaryEntry {
        competence: "2020-02".into(),
        amount_centavos: 300_000,
        indicators: vec!["PRPPS".into()],
        source: SourceRef { page: 1, line: 3, index: 2, byte_offset: 0 },
        link_sequence: Some(2),
        amount_kind: AmountKind::Remuneration,
        validation_status: ValidationStatus::Unvalidated,
        excluded_from_validated_domain: false,
    });
    cnis.salary_months.insert(
        "2020-02".into(),
        SalaryMonth { count: 1, amount_sum_centavos: 300_000, entry_indices: vec![2], contains_unresolved_prpps: true },
    );
    cnis
}

fn case() -> Calculation {
    let cnis = base();
    Calculation {
        identity: previus_cnis::Identity { birth_date: Some("1960-01-01".into()), ..cnis.identity.clone() },
        cnis: Some(cnis),
        sex: Some("feminino".into()),
        dib: "2021-01-31".into(),
        kind: "RGPS".into(),
        ..Default::default()
    }
}

#[test]
fn guided_preparation_excludes_prpps_and_fills_missing_end() {
    let case = case();
    let before = serde_json::to_value(&case.cnis).unwrap();
    let (prepared, notes) = prepare_from_cnis(&case, "CNIS conferido em 2026-10-23").unwrap();
    assert_eq!(prepared.manual_links.len(), 1);
    assert_eq!(prepared.manual_links[0].end_date, "2020-01-31");
    assert_eq!(prepared.manual_links[0].regime, "RGPS");
    assert!(prepared.manual_links[0].confirmed);
    assert_eq!(prepared.manual_salaries.len(), 1);
    assert_eq!(prepared.manual_salaries[0].competence, "2020-01");
    assert!(notes.iter().any(|note| note.contains("PRPPS")));
    assert!(notes.iter().any(|note| note.contains("Hipótese preparada")));
    assert!(prepared.options.rule_policy.contains("\"exclude_imported_source\":true"));
    assert_eq!(before, serde_json::to_value(&prepared.cnis).unwrap());
}

#[test]
fn guided_case_reaches_the_engine_as_experimental() {
    let (prepared, _) = prepare_from_cnis(&case(), "CNIS conferido em 2026-10-23").unwrap();
    let result = previus_rgps::calculate_case(&prepared).unwrap();
    assert_eq!(result["status"], "experimental", "{result}");
    assert!(result["data"]["native_result"]["resumo"]["rmi"].is_string(), "{result}");
}

#[test]
fn guided_preparation_never_invents_interview_answers() {
    let mut case = case();
    case.assistant = Some(previus_workspace::AssistantInterview { mode: "assisted".into(), ..Default::default() });
    let (prepared, notes) = prepare_from_cnis(&case, "CNIS conferido em 2026-10-23").unwrap();
    let assistant = prepared.assistant.unwrap();
    assert_eq!(assistant.current_benefit_received, None);
    assert_eq!(assistant.process_kind, None);
    assert_eq!(assistant.goal, None);
    assert!(assistant.review_confirmed);
    assert!(notes.iter().any(|note| note.contains("continua pendente")), "{notes:?}");
}
