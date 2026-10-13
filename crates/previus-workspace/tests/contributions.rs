use previus_workspace::contributions::{
    amount_cents, ContributionChange, ContributionRecord, ContributionReview,
};
use previus_workspace::{Calculation, ManualSalary};
fn case() -> Calculation {
    let imported = previus_cnis::import_text(
        "CNIS\nCompetência Remuneração Indicadores\n01/2020 100,10\n01/2020 200,20",
    )
    .unwrap();
    let records = imported
        .salary_entries
        .iter()
        .map(|e| ContributionRecord {
            key: format!("source:{}", e.source.index),
            source_indices: vec![e.source.index],
            competence: e.competence.clone(),
            amount: format!("{}.{:02}", e.amount_centavos / 100, e.amount_centavos % 100),
            currency: "unconfirmed".into(),
            link_sequence: e.link_sequence,
            indicators: e.indicators.join(", "),
            confirmed: false,
            excluded: false,
            evidence: String::new(),
        })
        .collect();
    let review = ContributionReview {
        version: 1,
        source_sha256: imported.metadata.source_sha256.clone(),
        records,
        history: vec![],
        applied: false,
    };
    Calculation {
        cnis: Some(imported),
        contribution_review: Some(review),
        ..Default::default()
    }
}
fn operation(case: &mut Calculation, records: Vec<ContributionRecord>, action: &str) {
    let review = case.contribution_review.as_mut().unwrap();
    let before = review.records.clone();
    review.history.push(ContributionChange {
        sequence: review.history.len() + 1,
        action: action.into(),
        reason: "Documento fictício de teste".into(),
        at: "2026-10-13T10:00:00.000Z".into(),
        before,
        after: records.clone(),
    });
    review.records = records;
    review.applied = false;
}
#[test]
fn canonical_cents_are_exact_and_bounded() {
    assert_eq!(
        amount_cents("90071992547409.93").unwrap(),
        9_007_199_254_740_993
    );
    assert_eq!(amount_cents("92233720368547758.07").unwrap(), i64::MAX);
    for invalid in [
        "-1.00",
        "1,00",
        "1.001",
        "01.00",
        "1e3",
        "92233720368547758.08",
    ] {
        assert!(amount_cents(invalid).is_err());
    }
}
#[test]
fn initial_source_is_preserved_and_not_projected_implicitly() {
    let c = case();
    c.contribution_review
        .as_ref()
        .unwrap()
        .validate(&c)
        .unwrap();
    assert!(c
        .contribution_review
        .as_ref()
        .unwrap()
        .projection()
        .is_err());
}
#[test]
fn documented_merge_and_application_preserve_source() {
    let mut c = case();
    let source = serde_json::to_value(&c.cnis).unwrap();
    let mut merged = c.contribution_review.as_ref().unwrap().records[0].clone();
    merged.source_indices = vec![1, 2];
    merged.amount = "300.30".into();
    merged.evidence = "Soma nominal documentada".into();
    operation(&mut c, vec![merged], "merge");
    let mut confirmed = c.contribution_review.as_ref().unwrap().records.clone();
    confirmed[0].currency = "BRL".into();
    confirmed[0].confirmed = true;
    operation(&mut c, confirmed, "confirm");
    c.manual_salaries = c
        .contribution_review
        .as_ref()
        .unwrap()
        .projection()
        .unwrap();
    c.contribution_review.as_mut().unwrap().applied = true;
    c.contribution_review
        .as_ref()
        .unwrap()
        .validate(&c)
        .unwrap();
    assert_eq!(c.manual_salaries[0].amount, "300.30");
    assert_eq!(serde_json::to_value(&c.cnis).unwrap(), source);
    c.manual_salaries[0].amount = "300.31".into();
    assert!(c
        .contribution_review
        .as_ref()
        .unwrap()
        .validate(&c)
        .is_err());
}
#[test]
fn omitted_duplicate_and_tampered_sources_fail_closed() {
    let c = case();
    let mut missing = c.clone();
    missing.contribution_review.as_mut().unwrap().records.pop();
    assert!(missing
        .contribution_review
        .as_ref()
        .unwrap()
        .validate(&missing)
        .is_err());
    let mut duplicate = c.clone();
    duplicate.contribution_review.as_mut().unwrap().records[1].source_indices = vec![1];
    assert!(duplicate
        .contribution_review
        .as_ref()
        .unwrap()
        .validate(&duplicate)
        .is_err());
    let mut altered = c.clone();
    altered.contribution_review.as_mut().unwrap().records[0].amount = "100.11".into();
    assert!(altered
        .contribution_review
        .as_ref()
        .unwrap()
        .validate(&altered)
        .is_err());
    let mut replaced = c;
    replaced.contribution_review.as_mut().unwrap().source_sha256 = Some("f".repeat(64));
    assert!(replaced
        .contribution_review
        .as_ref()
        .unwrap()
        .validate(&replaced)
        .is_err());
}
#[test]
fn history_must_match_final_state_and_preserve_initial_values() {
    let mut c = case();
    let mut rows = c.contribution_review.as_ref().unwrap().records.clone();
    rows[0].amount = "100.11".into();
    rows[0].evidence = "Retificação fictícia".into();
    operation(&mut c, rows, "edit");
    c.contribution_review
        .as_ref()
        .unwrap()
        .validate(&c)
        .unwrap();
    let mut forged = c.clone();
    forged.contribution_review.as_mut().unwrap().history[0].before[0].amount = "100.12".into();
    assert!(forged
        .contribution_review
        .as_ref()
        .unwrap()
        .validate(&forged)
        .is_err());
    c.contribution_review.as_mut().unwrap().history[0].after[0].amount = "100.13".into();
    assert!(c
        .contribution_review
        .as_ref()
        .unwrap()
        .validate(&c)
        .is_err());
}
#[test]
fn legacy_case_adds_optional_review_without_inferring_any_rows() {
    let mut json = serde_json::to_value(Calculation::default()).unwrap();
    json.as_object_mut().unwrap().remove("contribution_review");
    let legacy: Calculation = serde_json::from_value(json).unwrap();
    assert!(legacy.contribution_review.is_none());
}
#[test]
fn excluded_rows_need_evidence_and_never_remove_source() {
    let mut c = case();
    let mut rows = c.contribution_review.as_ref().unwrap().records.clone();
    rows[0].excluded = true;
    assert!(rows[0].evidence.is_empty());
    operation(&mut c, rows.clone(), "exclude");
    assert!(c
        .contribution_review
        .as_ref()
        .unwrap()
        .validate(&c)
        .is_err());
    rows[0].evidence = "Exclusão fictícia documentada".into();
    let mut valid = case();
    operation(&mut valid, rows, "exclude");
    valid
        .contribution_review
        .as_ref()
        .unwrap()
        .validate(&valid)
        .unwrap();
    assert_eq!(valid.cnis.unwrap().salary_entries.len(), 2);
}
#[test]
fn projection_requires_documentary_confirmation_and_unique_months() {
    let mut c = case();
    let review = c.contribution_review.as_mut().unwrap();
    for r in &mut review.records {
        r.currency = "BRL".into();
        r.confirmed = true;
        r.evidence = "Conferência".into();
    }
    assert!(review.projection().is_err());
    review.records[1].excluded = true;
    assert_eq!(review.projection().unwrap().len(), 1);
    review.records[0].currency = "URV".into();
    assert!(review.projection().is_err());
    review.records[0].currency = "BRL".into();
    review.records[0].indicators = "PRPPS".into();
    assert!(review.projection().is_err());
}
#[test]
fn rows_and_history_survive_json_roundtrip() {
    let c = case();
    let json = serde_json::to_string(&c).unwrap();
    let loaded: Calculation = serde_json::from_str(&json).unwrap();
    loaded
        .contribution_review
        .as_ref()
        .unwrap()
        .validate(&loaded)
        .unwrap();
    assert_eq!(loaded.contribution_review.unwrap().records.len(), 2);
}
#[test]
fn mismatched_applied_manual_rows_are_rejected() {
    let c = case();
    let r = c.contribution_review.as_ref().unwrap();
    assert!(r
        .validate_projection(&[ManualSalary {
            amount: "999.00".into(),
            ..Default::default()
        }])
        .is_err());
}

#[cfg(windows)]
#[test]
fn reviewed_overlay_roundtrips_through_actual_dpapi_without_plaintext() {
    let mut c=case();c.id=1;
    let mut rows=c.contribution_review.as_ref().unwrap().records.clone();
    rows[0].amount="110.11".into();rows[0].evidence="SYNTHETIC_REVIEW_PRIVATE_MARKER_56".into();operation(&mut c,rows,"edit");
    let expected=serde_json::to_value(&c.contribution_review).unwrap();
    let mut workspace=previus_workspace::Workspace{calculations:vec![c],next_id:2,..Default::default()};
    let dir=std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/test-fixtures").join(format!("ledger-dpapi-{}",std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();let path=dir.join("workspace.pvr");workspace.save(&path).unwrap();
    let encrypted=std::fs::read(&path).unwrap();assert!(!encrypted.windows(b"SYNTHETIC_REVIEW_PRIVATE_MARKER_56".len()).any(|bytes|bytes==b"SYNTHETIC_REVIEW_PRIVATE_MARKER_56"));
    let loaded=previus_workspace::Workspace::load(&path).unwrap();assert_eq!(serde_json::to_value(&loaded.calculations[0].contribution_review).unwrap(),expected);
    std::fs::remove_file(path).unwrap();std::fs::remove_dir(dir).unwrap();
}
