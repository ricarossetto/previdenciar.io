use previdenciar_workspace::{Calculation, ManualLink, ManualSalary};
use serde_json::json;

#[test]
fn pcd_unconfirmed_still_blocks_without_inventing_results_and_confirmed_routes_to_lc142() {
    for degree in ["Grave", "Moderada", "Leve"] {
        let c = Calculation {
            kind: "RGPS".into(),
            disability: Some(degree.into()),
            ..Default::default()
        };
        let result = previdenciar_rgps::calculate_case(&c).unwrap();
        assert_eq!(result["status"], "blocked");
        assert!(result["data"].is_null());
        assert!(result["blockers"].as_array().unwrap().iter().any(|b| b["code"].as_str().unwrap_or("").starts_with("lc142_")));
    }

    let mut ok = Calculation::default();
    ok.kind = "RGPS".into();
    ok.sex = Some("masculino".into());
    ok.disability = Some("leve".into());
    ok.especie = Some("42".into());
    ok.identity.birth_date = Some("1970-01-01".into());
    ok.dib = "2026-11-17".into();
    ok.manual_links.push(ManualLink {
        id: 1,
        start_date: "1990-01-01".into(),
        end_date: "2026-11-17".into(),
        regime: "RGPS".into(),
        confirmed: true,
        ..Default::default()
    });
    for y in 1994..=2026 {
        let m_start = if y == 1994 { 7 } else { 1 };
        let m_end = if y == 2026 { 10 } else { 12 };
        for m in m_start..=m_end {
            ok.manual_salaries.push(ManualSalary {
                id: 100,
                competence: format!("{y:04}-{m:02}"),
                amount: "3000.00".into(),
                confirmed: true,
                ..Default::default()
            });
        }
    }
    ok.options.rule_policy = json!({
        "version": previdenciar_rgps::POLICY_VERSION,
        "review_evidence": "Conferência sintética PcD",
        "manual_link_kind": "empregado",
        "manual_currency": "BRL"
    })
    .to_string();
    let outcome = previdenciar_rgps::calculate_case(&ok).unwrap();
    assert_eq!(outcome["status"], "experimental", "{outcome}");
    assert_eq!(outcome["data"]["native_result"]["pcd"]["grau"], "leve");
}
