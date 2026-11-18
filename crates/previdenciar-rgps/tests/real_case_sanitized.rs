use previdenciar_cnis::{
    AmountKind, EmploymentLink, ImportMetadata, ImportResult, SalaryEntry, SourceRef,
    ValidationStatus,
};
use previdenciar_rgps::fabrica::{
    build_body, build_tempo_body, case_to_fabrica_input, VALID_ESPECIES,
};
use previdenciar_rgps::{calculate_json, POLICY_VERSION};
use previdenciar_workspace::Calculation;
use serde_json::{json, Value};

fn mm_yyyy_to_yyyy_mm(s: &str) -> String {
    let (m, y) = s.split_once('/').expect("MM/YYYY");
    format!("{y}-{m}")
}

fn build_demo_rk_input(dib: &str) -> Value {
    let raw: Value = serde_json::from_str(include_str!(
        "../../../../../OpenPrev/openprev/data/demo_caso_rk.json"
    ))
    .unwrap();
    let seg = &raw["segurado"];
    let sex = match seg["sexo"].as_str().unwrap() {
        "M" => "masculino",
        "F" => "feminino",
        other => other,
    };
    let vinculos: Vec<Value> = raw["periodos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            json!({
                "inicio": p["inicio"],
                "fim": p["fim"],
                "tipo": "empregado",
                "regime": "RGPS",
                "confirmado": true,
                "evidencia": "demo_caso_rk.json",
                "indicadores": [],
                "origem": "manual_documentado",
                "prpps": false,
                "ctc_confirmada": false,
                "fator": 1.0
            })
        })
        .collect();
    let competencias: Vec<Value> = raw["remuneracoes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["incluida"].as_bool().unwrap_or(true))
        .map(|r| {
            json!({
                "competencia": mm_yyyy_to_yyyy_mm(r["competencia"].as_str().unwrap()),
                "salario": r["valor"],
                "moeda": "BRL",
                "confirmada": true,
                "indicadores": []
            })
        })
        .collect();
    let ref_month = &dib[..7];
    json!({
        "segurado": {
            "sexo": sex,
            "dataNascimento": seg["nascimento"],
            "deficiencia": "nao",
            "especie": "42"
        },
        "dataReferencia": dib,
        "vinculos": vinculos,
        "competencias": competencias,
        "opcoes": {
            "corrigirSalarios": true,
            "descarte": false,
            "descarteOtimo": true,
            "divisorMinimo": false,
            "aplicarPisoAutomatico": true,
            "aplicarTetoVigenteDer": true,
            "referenciaCorrecao": ref_month
        },
        "revisao": {
            "confirmada": true,
            "evidencia": "demo_caso_rk.json",
            "politica": POLICY_VERSION
        }
    })
}

#[test]
fn demo_caso_rk_calculates_all_rules_and_optimal_discard_at_2023_and_2026_11_17() {
    for dib in ["2023-06-30", "2026-11-17"] {
        let input = build_demo_rk_input(dib);
        let outcome = calculate_json(input).expect("valid demo input");
        assert_eq!(outcome["status"], "experimental", "DIB {dib}: {outcome}");
        let native = &outcome["data"]["native_result"];
        let rules = native["transicoes"]["regras"].as_array().unwrap();
        assert!(!rules.is_empty(), "expected transition rules on {dib}");
        let rmi: f64 = native["resumo"]["rmi"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        assert!(rmi > 0.0, "expected positive RMI on {dib}: {outcome}");
        let descartes = native["simulacao_descartes"].as_array().unwrap();
        assert!(
            !descartes.is_empty(),
            "expected simulacao_descartes on {dib}: {outcome}"
        );
    }
}

#[test]
fn fabrica_maps_unconfirmed_cnis_with_pcd_and_overlapping_periods_without_local_approval() {
    let src = SourceRef {
        page: 1,
        line: 1,
        index: 1,
        byte_offset: 0,
    };
    let mut case = Calculation::default();
    case.kind = "RGPS".into();
    case.dib = "2026-11-17".into();
    case.disability = Some("moderada".into());
    case.especie = Some("41".into());
    case.identity.birth_date = Some("1965-05-10".into());
    case.cnis = Some(ImportResult {
        metadata: ImportMetadata {
            extractor: "test".into(),
            source_sha256: None,
            page_count: 1,
            parsed_pages: 1,
            legal_validation_performed: false,
        },
        identity: previdenciar_cnis::Identity {
            sex: Some("feminino".into()),
            birth_date: Some("1965-05-10".into()),
            ..Default::default()
        },
        contribution_months: vec![],
        salary_entries: vec![
            SalaryEntry {
                competence: "2010-01".into(),
                amount_centavos: 250_000,
                indicators: vec![],
                source: src.clone(),
                link_sequence: Some(1),
                amount_kind: AmountKind::Remuneration,
                validation_status: ValidationStatus::Unvalidated,
                excluded_from_validated_domain: false,
            },
            SalaryEntry {
                competence: "2010-01".into(),
                amount_centavos: 180_000,
                indicators: vec![],
                source: src.clone(),
                link_sequence: Some(2),
                amount_kind: AmountKind::Remuneration,
                validation_status: ValidationStatus::Unvalidated,
                excluded_from_validated_domain: false,
            },
        ],
        salary_months: Default::default(),
        links: vec![
            EmploymentLink {
                sequence: 1,
                raw_employer: Some("Empresa A".into()),
                affiliation: Some("Empregado".into()),
                start_date: Some("2000-01-01".into()),
                end_date: Some("2020-12-31".into()),
                reported_last_salary_month: None,
                inferred_last_salary_end: None,
                indicators: vec![],
                prpps: false,
                validation_status: ValidationStatus::Unvalidated,
                excluded_from_validated_domain: false,
                source: src.clone(),
            },
            EmploymentLink {
                sequence: 2,
                raw_employer: Some("Empresa B".into()),
                affiliation: Some("Contribuinte Individual".into()),
                start_date: Some("2009-01-01".into()),
                end_date: None,
                reported_last_salary_month: None,
                inferred_last_salary_end: None,
                indicators: vec![],
                prpps: false,
                validation_status: ValidationStatus::Unvalidated,
                excluded_from_validated_domain: false,
                source: src,
            },
        ],
        benefits: vec![],
        findings: vec![],
        warnings: vec![],
    });

    let fabrica_in = case_to_fabrica_input(&case).expect("direct fabrica mapping");
    assert_eq!(fabrica_in.sexo, "F");
    assert_eq!(fabrica_in.tipo.as_deref(), Some("PCD"));
    assert_eq!(fabrica_in.periodos.len(), 2);
    assert_eq!(fabrica_in.periodos[1].data_final, "2010-01-31");

    // When open link has no salary entries, falls back to DIB so TRF3 accepts it directly.
    let mut open_no_sal = case.clone();
    open_no_sal.cnis.as_mut().unwrap().links[1].end_date = None;
    open_no_sal.cnis.as_mut().unwrap().salary_entries.retain(|e| e.link_sequence != Some(2));
    let open_in = case_to_fabrica_input(&open_no_sal).expect("open link fabrica mapping");
    assert_eq!(open_in.periodos[1].data_final, "2026-11-17");

    let tempo_body = build_tempo_body(&fabrica_in).expect("tempo body");
    assert_eq!(tempo_body["dadosSegurado"]["tipo"], "PCD");
    let rmi_body = build_body(&fabrica_in).expect("rmi body");
    assert_eq!(rmi_body["periodosContribuicao"].as_array().unwrap().len(), 2);

    for sp in VALID_ESPECIES {
        let mut alt = fabrica_in.clone();
        alt.especie = sp;
        alt.isencao_carencia = false;
        assert!(build_body(&alt).is_ok(), "species {sp} should build body");
    }
}

#[test]
fn pcd_por_idade_lc142_computes_70_plus_1_and_favorable_factor() {
    let mut c = Calculation::default();
    c.kind = "RGPS".into();
    c.sex = Some("feminino".into());
    c.disability = Some("leve".into());
    c.especie = Some("41".into());
    c.identity.birth_date = Some("1968-01-01".into());
    c.dib = "2025-06-01".into();
    c.manual_links.push(previdenciar_workspace::ManualLink {
        id: 1,
        start_date: "2008-01-01".into(),
        end_date: "2025-05-31".into(),
        regime: "RGPS".into(),
        confirmed: true,
        ..Default::default()
    });
    for y in 2008..=2025 {
        let m_end = if y == 2025 { 5 } else { 12 };
        for m in 1..=m_end {
            c.manual_salaries.push(previdenciar_workspace::ManualSalary {
                id: 10,
                competence: format!("{y:04}-{m:02}"),
                amount: "4000.00".into(),
                confirmed: true,
                ..Default::default()
            });
        }
    }
    c.options.rule_policy = json!({
        "version": POLICY_VERSION,
        "review_evidence": "LC 142 Por Idade",
        "manual_link_kind": "empregado",
        "manual_currency": "BRL"
    })
    .to_string();
    let out = previdenciar_rgps::calculate_case(&c).unwrap();
    assert_eq!(out["status"], "experimental", "{out}");
    let pcd = &out["data"]["native_result"]["pcd"];
    assert_eq!(pcd["modalidade"], "idade");
    assert_eq!(pcd["por_idade"]["cumprido"], true);
    assert_eq!(pcd["por_idade"]["idade_exigida"], 55);
    assert_eq!(pcd["por_idade"]["coeficiente_pct"], 87);
    assert!(out["data"]["native_result"]["resumo"]["rmi"].is_string());
}
