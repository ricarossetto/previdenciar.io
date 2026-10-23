#[path = "common/pcd.rs"]
mod fixture;
use fixture::scenario;

#[test]
fn independent_constant_grade_scenario_uses_real_decimal_correction_and_80_percent_selection() {
    let (c, p) = scenario();
    let out = previus_rgps::pcd::calculate(&c, &p).unwrap();
    assert_eq!(out["status"], "experimental");
    let n = &out["data"]["native_result"];
    assert_eq!(n["resumo"]["rmi"], "2550.00");
    assert_eq!(n["correcao"]["consideradas"], 2);
    assert_eq!(n["correcao"]["parcelas"][0]["corrigido"], "1030.20");
    assert_eq!(n["pcd"]["actual_entitlement_verified"], false);
}
#[test]
fn source_and_inputs_are_never_modified() {
    let (c, p) = scenario();
    let before = serde_json::to_value(&c).unwrap();
    previus_rgps::pcd::calculate(&c, &p).unwrap();
    assert_eq!(before, serde_json::to_value(&c).unwrap());
}
#[test]
fn missing_salary_is_not_zero_and_is_declared() {
    let (mut c, mut p) = scenario();
    c.cnis.as_mut().unwrap().salary_months.remove("2020-02");
    let out = previus_rgps::pcd::calculate(&c, &p).unwrap();
    // Sem lançamento na origem, a competência não é tempo de contribuição:
    // ela fica fora do PBC e a omissão é declarada. Nunca vira zero.
    assert_eq!(out["status"], "experimental");
    assert_eq!(out["data"]["native_result"]["correcao"]["pbc"], 2);
    assert!(out["warnings"].as_array().unwrap().iter().any(|w| w.as_str().unwrap_or("").contains("2020-02")));
    p.fill_missing_minimum = true;
    p.missing_salary_evidence = "Hipótese fictícia".into();
    let out = previus_rgps::pcd::calculate(&c, &p).unwrap();
    assert_eq!(
        out["data"]["native_result"]["pcd"]["filled_minimum_count"],
        1
    );
}
#[test]
fn below_minimum_competence_is_not_time_or_pbc_until_complemented() {
    let (c0, p0) = scenario();
    let before = previus_rgps::pcd::calculate(&c0, &p0).unwrap()["data"]["native_result"]["resumo"]
        ["carencia_meses"]
        .as_u64()
        .unwrap();
    let (mut c, p) = scenario();
    c.cnis.as_mut().unwrap().salary_months.get_mut("2020-02").unwrap().amount_sum_centavos = 50000;
    let out = previus_rgps::pcd::calculate(&c, &p).unwrap();
    let n = &out["data"]["native_result"];
    assert_eq!(out["status"], "experimental");
    assert_eq!(n["correcao"]["pbc"], 2);
    assert_eq!(n["resumo"]["carencia_meses"].as_u64().unwrap(), before - 1);
    assert!(!n["correcao"]["parcelas"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["competencia"] == "2020-02"));
    let warnings = out["warnings"].as_array().unwrap();
    assert!(warnings.iter().any(|w| w.as_str().unwrap_or("").contains("abaixo do salário mínimo")));
    assert!(warnings.iter().any(|w| w.as_str().unwrap_or("").contains("2020-02")));
}
#[test]
fn local_snapshot_carries_monthly_minimum_and_ceiling() {
    let snapshot = previus_rgps::pcd::local_snapshot("2026-09").unwrap();
    let l2012 = snapshot.limits.iter().find(|limit| limit.month == "2012-06").unwrap();
    assert_eq!(l2012.minimum, "622.00");
    assert_eq!(l2012.ceiling, "3916.20");
    let l2026 = snapshot.limits.iter().find(|limit| limit.month == "2026-09").unwrap();
    assert_eq!(l2026.minimum, "1621.00");
    assert_eq!(l2026.ceiling, "8475.55");
    // O piso nunca pode ser o teto por omissão.
    assert!(snapshot.limits.iter().all(|limit| limit.minimum != limit.ceiling));
}
#[test]
fn future_salary_needs_declared_projection_and_is_not_imported() {
    let (mut c, mut p) = scenario();
    c.dib = "2020-06-01".into();
    assert_eq!(
        previus_rgps::pcd::calculate(&c, &p).unwrap()["status"],
        "blocked"
    );
    p.project_at_known_ceiling = true;
    p.projection_evidence = "Projeção fictícia pelo teto congelado".into();
    let out = previus_rgps::pcd::calculate(&c, &p).unwrap();
    assert_eq!(out["data"]["native_result"]["pcd"]["projection_count"], 2);
    assert_eq!(out["data"]["native_result"]["resumo"]["rmi"], "6275.00");
}
#[test]
fn no_global_medical_or_regime_inference() {
    let (c, mut p) = scenario();
    p.disability_all_periods = false;
    assert_eq!(
        previus_rgps::pcd::calculate(&c, &p).unwrap()["status"],
        "blocked"
    );
    p.disability_all_periods = true;
    p.rgps_confirmed = false;
    assert_eq!(
        previus_rgps::pcd::calculate(&c, &p).unwrap()["status"],
        "blocked"
    );
}
#[test]
fn closed_loop_monthly_snapshot_rejects_missing_and_duplicate_data() {
    let (c, mut p) = scenario();
    p.snapshot.as_mut().unwrap().rates.remove(0);
    assert!(previus_rgps::pcd::calculate(&c, &p).is_err());
    let (_, mut p) = scenario();
    let duplicate = p.snapshot.as_ref().unwrap().rates[0].clone();
    p.snapshot.as_mut().unwrap().rates.push(duplicate);
    assert!(p.snapshot.as_ref().unwrap().validate().is_err());
}
#[test]
fn malformed_decimal_date_and_method_fail_closed() {
    let (mut c, mut p) = scenario();
    c.dib = "2020-02-30".into();
    assert!(previus_rgps::pcd::calculate(&c, &p).is_err());
    let (c, _) = scenario();
    p.historical_negative_policy = "guess".into();
    assert_eq!(
        previus_rgps::pcd::calculate(&c, &p).unwrap()["status"],
        "blocked"
    );
}
#[test]
fn insufficient_time_has_no_benefit_rmi() {
    let (mut c, p) = scenario();
    c.cnis.as_mut().unwrap().links[0].start_date = Some("2019-01-01".into());
    let o = previus_rgps::pcd::calculate(&c, &p).unwrap();
    assert!(o["data"]["native_result"]["resumo"]["rmi"].is_null());
}

#[test]
fn scientific_provider_rates_are_normalized_exactly_without_relaxing_wage_rules() {
    assert_eq!(
        previus_rgps::pcd::canonical_snapshot_decimal("5e-5").unwrap(),
        "0.00005"
    );
    assert_eq!(
        previus_rgps::pcd::canonical_snapshot_decimal("-2E-4").unwrap(),
        "-0.0002"
    );
    assert!(previus_rgps::pcd::canonical_snapshot_decimal("NaN").is_err());
}

#[test]
fn a_legitimate_closed_link_after_the_dib_is_preserved_but_does_not_enter_historical_calculation() {
    let (mut c, p) = scenario();
    let mut future = c.cnis.as_ref().unwrap().links[0].clone();
    future.sequence = 2;
    future.start_date = Some("2021-01-01".into());
    future.end_date = Some("2021-12-31".into());
    c.cnis.as_mut().unwrap().links.push(future);
    let out = previus_rgps::pcd::calculate(&c, &p).unwrap();
    assert_eq!(out["status"], "experimental");
    assert_eq!(out["data"]["native_result"]["resumo"]["rmi"], "2550.00");
    assert_eq!(c.cnis.unwrap().links.len(), 2);
}

#[test]
fn displayed_factors_follow_reference_truncation_without_truncating_the_calculation() {
    let (c, mut p) = scenario();
    for rate in &mut p.snapshot.as_mut().unwrap().rates {
        rate.inpc = "0".into();
    }
    p.snapshot.as_mut().unwrap().rates[0].inpc = "0.0000006".into();
    let out = previus_rgps::pcd::calculate(&c, &p).unwrap();
    let row = &out["data"]["native_result"]["correcao"]["parcelas"][0];
    assert_eq!(row["indice"], "1.000000");
    assert_eq!(row["corrigido"], "1000.00");
}
#[test]
fn overlapping_employment_is_not_counted_twice() {
    let (mut c, p) = scenario();
    let before = previus_rgps::pcd::calculate(&c, &p).unwrap();
    let mut overlap = c.cnis.as_ref().unwrap().links[0].clone();
    overlap.sequence = 2;
    overlap.start_date = Some("1995-01-01".into());
    overlap.end_date = Some("2000-12-31".into());
    c.cnis.as_mut().unwrap().links.push(overlap);
    let after = previus_rgps::pcd::calculate(&c, &p).unwrap();
    assert_eq!(
        before["data"]["native_result"]["tempo"],
        after["data"]["native_result"]["tempo"]
    );
}
#[test]
fn an_unconfirmed_policy_never_produces_a_financial_result() {
    let (c, mut p) = scenario();
    p.assumptions_confirmed = false;
    let out = previus_rgps::pcd::calculate(&c, &p).unwrap();
    assert_eq!(out["status"], "blocked");
    assert!(out["data"].is_null());
}
#[test]
fn sex_and_constant_grade_remain_explicit_parameters() {
    for (sex, grade, years) in [
        ("Masculino", "Grave", 25),
        ("Feminino", "Grave", 20),
        ("Masculino", "Moderada", 29),
        ("Feminino", "Moderada", 24),
        ("Masculino", "Leve", 33),
        ("Feminino", "Leve", 28),
    ] {
        let (mut c, p) = scenario();
        c.sex = Some(sex.into());
        c.disability = Some(grade.into());
        let out = previus_rgps::pcd::calculate(&c, &p).unwrap();
        assert_eq!(out["data"]["native_result"]["pcd"]["anos_exigidos"], years);
    }
}
#[test]
fn manual_salaries_without_a_traceable_book_are_not_silently_ignored() {
    let (mut c, p) = scenario();
    c.manual_salaries.push(previus_workspace::ManualSalary {
        competence: "2020-01".into(),
        amount: "9000.00".into(),
        ..Default::default()
    });
    let out = previus_rgps::pcd::calculate(&c, &p).unwrap();
    assert_eq!(out["status"], "blocked");
    assert!(out["data"].is_null());
    assert!(out["blockers"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["code"] == "lc142_unreviewed_salaries"));
}

#[test]
fn salary_ledger_shows_only_the_days_covered_by_joined_periods() {
    let (mut c, p) = scenario();
    let mut second = c.cnis.as_ref().unwrap().links[0].clone();
    c.cnis.as_mut().unwrap().links[0].end_date = Some("2020-02-03".into());
    second.sequence = 2;
    second.start_date = Some("2020-02-16".into());
    c.cnis.as_mut().unwrap().links.push(second);
    let out = previus_rgps::pcd::calculate(&c, &p).unwrap();
    let rows = out["data"]["native_result"]["correcao"]["parcelas"]
        .as_array()
        .unwrap();
    assert_eq!(rows[0]["dias"], 30);
    assert_eq!(rows[1]["dias"], 18);
    assert_eq!(rows[2]["dias"], 30);
}
