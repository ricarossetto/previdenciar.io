use previus_workspace::contributions::{
    ContributionChange, ContributionRecord, ContributionReview,
};
use previus_workspace::Calculation;
fn reviewed() -> Calculation {
    let mut c: Calculation =
        serde_json::from_str(include_str!("fixtures/calculation-synthetic.json")).unwrap();
    let before: Vec<_> = c
        .manual_salaries
        .iter()
        .enumerate()
        .map(|(i, w)| ContributionRecord {
            key: format!("manual:{i}"),
            source_indices: vec![],
            competence: w.competence.clone(),
            amount: w.amount.clone(),
            currency: "unconfirmed".into(),
            link_sequence: None,
            indicators: w.indicators.clone(),
            confirmed: false,
            excluded: false,
            evidence: String::new(),
        })
        .collect();
    let after: Vec<_> = before
        .iter()
        .cloned()
        .map(|mut r| {
            r.currency = "BRL".into();
            r.confirmed = true;
            r.evidence = "Documento fictício conferido".into();
            r
        })
        .collect();
    c.contribution_review = Some(ContributionReview {
        version: 1,
        source_sha256: None,
        records: after.clone(),
        history: vec![ContributionChange {
            sequence: 1,
            action: "confirm".into(),
            reason: "Conferência fictícia".into(),
            at: "2026-10-13T10:00:00Z".into(),
            before,
            after,
        }],
        applied: true,
    });
    c
}
#[test]
fn reviewed_projection_reaches_the_real_synthetic_rgps_result() {
    let c = reviewed();
    let result = previus_rgps::calculate_case(&c).unwrap();
    assert_eq!(result["status"], "experimental");
    assert_eq!(result["data"]["native_result"]["resumo"]["rmi"], "1600.00");
}
#[test]
fn a_visual_edit_without_application_blocks_stale_manual_values() {
    let mut c = reviewed();
    c.contribution_review.as_mut().unwrap().applied = false;
    let result = previus_rgps::calculate_case(&c).unwrap();
    assert_eq!(result["status"], "blocked");
    assert!(result["data"].is_null());
    assert!(result["blockers"]
        .as_array()
        .unwrap()
        .iter()
        .any(|b| b["code"] == "contribution_review_unapplied"));
}
#[test]
fn forged_application_or_modified_manual_rows_fail_before_the_engine() {
    let mut c = reviewed();
    c.manual_salaries[0].amount = "999.00".into();
    assert!(previus_rgps::calculate_case(&c).is_err());
}
