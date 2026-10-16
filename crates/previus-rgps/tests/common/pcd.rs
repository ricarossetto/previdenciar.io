use previus_rgps::pcd::{Lc142Policy, Limits, Rate, Snapshot};
use previus_workspace::Calculation;
pub fn scenario() -> (Calculation, Lc142Policy) {
    let mut source=previus_cnis::import_text("CNIS\nCompetência Remuneração Indicadores\n01/2020 1.000,00\n02/2020 2.000,00\n03/2020 3.000,00").unwrap();
    source.identity.birth_date = Some("1960-01-01".into());
    source.links.push(previus_cnis::EmploymentLink {
        sequence: 1,
        raw_employer: Some("EMPREGADOR FICTÍCIO".into()),
        affiliation: Some("Empregado".into()),
        start_date: Some("1990-01-01".into()),
        end_date: None,
        reported_last_salary_month: None,
        inferred_last_salary_end: None,
        prpps: false,
        indicators: vec![],
        validation_status: previus_cnis::ValidationStatus::Unvalidated,
        excluded_from_validated_domain: false,
        source: previus_cnis::SourceRef {
            page: 1,
            line: 1,
            index: 1,
            byte_offset: 0,
        },
    });
    let snapshot = Snapshot {
        source_url: "https://fabrica-de-calculos.app.trf3.jus.br/indices/api/v1/tabela".into(),
        retrieved_at: "2026-10-15T10:00:00Z".into(),
        response_sha256: "a".repeat(64),
        through: "2020-04".into(),
        rates: (1..=4)
            .map(|m| Rate {
                month: format!("2020-{m:02}"),
                inpc: if m == 1 {
                    "0.01"
                } else if m == 3 {
                    "0.02"
                } else {
                    "0"
                }
                .into(),
                igpdi: None,
                ipcr: None,
                inpc_inss: None,
            })
            .collect(),
        limits: (1..=4)
            .map(|m| Limits {
                month: format!("2020-{m:02}"),
                minimum: "1000.00".into(),
                ceiling: "10000.00".into(),
                minimum_source: "Fixture fictícia, não valor oficial".into(),
                ceiling_source: "Fixture fictícia, não valor oficial".into(),
            })
            .collect(),
    };
    let case = Calculation {
        kind: "RGPS".into(),
        identity: source.identity.clone(),
        cnis: Some(source),
        sex: Some("Masculino".into()),
        disability: Some("Moderada".into()),
        dib: "2020-04-01".into(),
        ..Default::default()
    };
    let policy = Lc142Policy {
        version: previus_rgps::pcd::VERSION.into(),
        calculation_date: "2020-04-01".into(),
        correction_month: "2020-04".into(),
        assumptions_confirmed: true,
        review_evidence: "Dados inteiramente fictícios".into(),
        disability_all_periods: true,
        disability_evidence: "Hipótese fictícia de grau constante".into(),
        rgps_confirmed: true,
        project_open_links: true,
        ongoing_evidence: "Continuidade fictícia".into(),
        fill_missing_minimum: false,
        missing_salary_evidence: String::new(),
        project_at_known_ceiling: false,
        projection_evidence: String::new(),
        historical_negative_policy: "preserve".into(),
        apply_inss_overrides: true,
        snapshot: Some(snapshot),
    };
    (case, policy)
}
