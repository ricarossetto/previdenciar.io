mod common;
use common::extract;
use previus_reports::{render_pdf, ReportKind, ReportOptions};
use previus_workspace::contributions::{
    ContributionChange, ContributionRecord, ContributionReview,
};
use previus_workspace::{Calculation, Workspace};
fn reviewed() -> Calculation {
    let cnis =
        previus_cnis::import_text("CNIS\nCompetência Remuneração Indicadores\n01/2020 100,10")
            .unwrap();
    let e = &cnis.salary_entries[0];
    let before = ContributionRecord {
        key: format!("source:{}", e.source.index),
        source_indices: vec![e.source.index],
        competence: e.competence.clone(),
        amount: "100.10".into(),
        currency: "unconfirmed".into(),
        link_sequence: e.link_sequence,
        indicators: e.indicators.join(", "),
        confirmed: false,
        excluded: false,
        evidence: String::new(),
        amount_kind: e.amount_kind.clone(),
    };
    let after = ContributionRecord {
        amount: "110.11".into(),
        currency: "BRL".into(),
        evidence: "PRIVATE_SYNTHETIC_EVIDENCE_42".into(),
        ..before.clone()
    };
    let review = ContributionReview {
        version: 1,
        source_sha256: cnis.metadata.source_sha256.clone(),
        records: vec![after.clone()],
        history: vec![ContributionChange {
            sequence: 1,
            action: "edit".into(),
            reason: "PRIVATE_SYNTHETIC_EVIDENCE_42".into(),
            at: "2026-10-13T10:00:00Z".into(),
            before: vec![before],
            after: vec![after],
        }],
        applied: false,
    };
    Calculation {
        cnis: Some(cnis),
        contribution_review: Some(review),
        ..Default::default()
    }
}
#[test]
fn real_private_pdf_contains_original_and_adjusted_amounts_but_hides_evidence() {
    let c = reviewed();
    let source = serde_json::to_value(&c.cnis).unwrap();
    let bytes = render_pdf(
        &c,
        &Workspace::default(),
        ReportOptions::new(ReportKind::Salaries, false,),
        None,
    )
    .unwrap();
    let (_, text) = extract(&bytes);
    assert!(text.contains("Livro de revisão de contribuições"));
    assert!(text.contains("100,10"));
    assert!(text.contains("110,11"));
    assert!(text.contains("Justificativa ocultada"));
    assert!(!text.contains("PRIVATE_SYNTHETIC_EVIDENCE_42"));
    assert_eq!(serde_json::to_value(&c.cnis).unwrap(), source);
}
#[test]
fn authorized_synthetic_report_includes_audit_evidence_without_fabricating_outcome() {
    let c = reviewed();
    let bytes = render_pdf(
        &c,
        &Workspace::default(),
        ReportOptions::new(ReportKind::Detailed, true,),
        None,
    )
    .unwrap();
    let (_, text) = extract(&bytes);
    assert!(text.contains("PRIVATE_SYNTHETIC_EVIDENCE_42"));
    assert!(text.contains("Revisão não aplicada"));
    assert!(text.contains("DADOS PENDENTES"));
}
