mod common;
#[path = "../../previus-rgps/tests/common/pcd.rs"]
mod fixture;
use common::{compact, extract};
use previus_reports::{render_pdf, ReportKind, ReportOptions};
use previus_workspace::Workspace;

#[test]
fn every_pcd_report_is_a_real_pdf_with_declared_assumptions_and_hidden_evidence() {
    let (mut case, mut policy) = fixture::scenario();
    case.identity.full_name = Some("IDENTIDADEPCDMARCADOR".into());
    case.identity.cpf = Some("CPFPCDMARCADOR".into());
    case.notes = "NOTAPCDMARCADOR".into();
    policy.review_evidence = "REVISAOPCDMARCADOR".into();
    policy.disability_evidence = "LAUDOPCDMARCADOR".into();
    let outcome = previus_rgps::pcd::calculate(&case, &policy).unwrap();
    let before = serde_json::to_value(&case).unwrap();
    for kind in [
        ReportKind::Detailed,
        ReportKind::Summary,
        ReportKind::Periods,
        ReportKind::Salaries,
        ReportKind::Memory,
        ReportKind::Transitions,
        ReportKind::Indicators,
    ] {
        let bytes = render_pdf(
            &case,
            &Workspace::default(),
            ReportOptions::new(kind, false,),
            Some(&outcome),
        )
        .unwrap();
        assert!(bytes.starts_with(b"%PDF-"));
        let (doc, text) = extract(&bytes);
        assert!(!doc.get_pages().is_empty());
        let flat = compact(&text);
        for required in [
            "LC 142",
            "PREMISSAS DECLARADAS",
            "Grau de deficiência constante",
            "Hash do snapshot",
        ] {
            assert!(flat.contains(&compact(required)), "missing {required}");
        }
        for hidden in [
            "IDENTIDADEPCDMARCADOR",
            "CPFPCDMARCADOR",
            "NOTAPCDMARCADOR",
            "REVISAOPCDMARCADOR",
            "LAUDOPCDMARCADOR",
        ] {
            assert!(!flat.contains(hidden), "leaked {hidden}");
        }
        if matches!(
            kind,
            ReportKind::Summary | ReportKind::Memory | ReportKind::Salaries | ReportKind::Detailed
        ) {
            assert!(flat.contains("2.550,00"));
        }
    }
    assert_eq!(before, serde_json::to_value(&case).unwrap());
}

#[test]
fn pcd_report_hides_a_stored_rmi_when_assumptions_are_not_confirmed() {
    let (mut case, mut policy) = fixture::scenario();
    case.result = Some(serde_json::json!({"rmi":"987654.32"}));
    policy.assumptions_confirmed = false;
    let outcome = previus_rgps::pcd::calculate(&case, &policy).unwrap();
    let bytes = render_pdf(
        &case,
        &Workspace::default(),
        ReportOptions::new(ReportKind::Summary, false,),
        Some(&outcome),
    )
    .unwrap();
    let (_, text) = extract(&bytes);
    assert!(text.contains("DADOS PENDENTES"));
    assert!(!text.contains("987.654,32"));
}

#[test]
fn personal_pcd_headers_obey_the_same_layout_budget_as_other_reports() {
    let (case, policy) = fixture::scenario();
    let outcome = previus_rgps::pcd::calculate(&case, &policy).unwrap();
    let mut workspace = Workspace::default();
    workspace.settings.office_name = "x".repeat(121);
    assert!(render_pdf(
        &case,
        &workspace,
        ReportOptions::new(ReportKind::Summary, true),
        Some(&outcome)
    )
    .is_err());
}
