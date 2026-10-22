//! Diagnóstico local privado da qualidade de segurado sobre um CNIS autorizado.
//! Só imprime contagens agregadas; nenhum dado pessoal sai para a tela.
use previus_cnis::EmploymentLink;
use previus_core::{CivilDate, QualityInput, QualityPeriod, YearMonth};
use std::{collections::BTreeSet, fs::OpenOptions, io::Write, path::PathBuf};

fn period_days(start: CivilDate, end: CivilDate) -> i64 {
    i64::from(end.year() - start.year()) * 360
        + (i64::from(end.month()) - i64::from(start.month())) * 30
        + i64::from(end.day().min(30))
        - i64::from(start.day().min(30))
        + 1
}

fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        eprintln!("Uso: private_quality_probe <CNIS-local.pdf> <novo-diretorio-privado>");
        std::process::exit(2);
    }
    let source = PathBuf::from(&args[0]);
    let output = PathBuf::from(&args[1]);
    assert!(!output.exists(), "Use um diretório novo; nenhuma evidência anterior é sobrescrita");
    std::fs::create_dir_all(&output).expect("Diretório privado");
    let cnis = previus_cnis::import_pdf(&source).expect("Leitor local do CNIS");
    let mut periods = Vec::new();
    for link in &cnis.links {
        let start = link.start_date.as_deref().and_then(|value| value.parse::<CivilDate>().ok());
        let end = link.end_date.as_deref().and_then(|value| value.parse::<CivilDate>().ok());
        let mut contribution_months = Vec::new();
        for entry in cnis.salary_entries.iter().filter(|entry| entry.link_sequence == Some(link.sequence)) {
            if let Ok(month) = entry.competence.parse::<YearMonth>() { contribution_months.push(month); }
        }
        let benefit_period = link.affiliation.as_deref().map(|value| value.to_lowercase().contains("benefício")).unwrap_or(false);
        periods.push(QualityPeriod {
            label: format!("Vínculo {:02}", link.sequence),
            start, end,
            affiliation: link.affiliation.clone(),
            employer: link.raw_employer.clone(),
            contribution_months,
            disregarded_months: Vec::new(),
            had_benefit_episode: false,
            benefit_period,
        });
    }
    let filed_before_july_1991 = cnis.links.iter().filter_map(|link| link.start_date.as_deref())
        .any(|value| value <= "1991-07-24");
    let summary = previus_core::insured_quality(&QualityInput {
        periods,
        required_carencia_months: 180,
        implementation_year: None,
        filed_before_july_1991,
    }).expect("Apuração da qualidade");
    // Tempo de serviço: união dos períodos de vínculo, convenção 30/360 inclusiva.
    let mut spans: Vec<(CivilDate, CivilDate)> = Vec::new();
    for link in &cnis.links {
        let Some(start) = link.start_date.as_deref().and_then(|value| value.parse::<CivilDate>().ok()) else { continue };
        let end = link.end_date.as_deref().and_then(|value| value.parse::<CivilDate>().ok())
            .unwrap_or(CivilDate::new(2026, 10, 20).expect("Data de corte explícita"));
        spans.push((start, end));
    }
    spans.sort();
    let mut merged: Vec<(CivilDate, CivilDate)> = Vec::new();
    for (a, b) in spans {
        match merged.last_mut() {
            Some(last) if a <= last.1 => { last.1 = last.1.max(b); }
            _ => merged.push((a, b)),
        }
    }
    let days: i64 = merged.iter().map(|(a, b)| period_days(*a, *b)).sum();
    let mut counted_months: BTreeSet<YearMonth> = BTreeSet::new();
    for period in &summary.periods {
        let _ = period;
    }
    let mut links_summary = Vec::new();
    for period in &summary.periods {
        links_summary.push(serde_json::json!({
            "label": period.label,
            "counted_months": period.counted_months,
            "overlap_months": period.overlap_months,
            "benefit_months": period.benefit_months,
            "operator_months": period.disregarded_months,
            "first_payment": period.first_payment.map(|value| value.to_string()),
            "last_payment": period.last_payment.map(|value| value.to_string()),
            "grace_base": period.grace_base_until.map(|value| value.to_br()),
            "grace_until": period.grace_until.map(|value| value.to_br()),
            "grace_prorogued": period.grace_prorogued,
        }));
        counted_months.insert(period.first_payment.unwrap_or(YearMonth::new(1900, 1).expect("mês")));
    }
    let report = serde_json::json!({
        "import": cnis.summary(),
        "total_counted_months": summary.total_counted_months,
        "total_overlap_months": summary.total_overlap_months,
        "total_benefit_months": summary.total_benefit_months,
        "total_operator_months": summary.total_disregarded_months,
        "required_carencia_months": summary.required_carencia_months,
        "required_carencia_basis": summary.required_carencia_basis,
        "carencia_observed": summary.carencia_observed,
        "art_142_candidate_year": summary.art_142_candidate_year,
        "service_time_30_360": {"days": days, "years": days / 360, "months": (days % 360) / 30, "days_remainder": days % 30},
        "links": links_summary,
    });
    let mut file = OpenOptions::new().write(true).create_new(true)
        .open(output.join("quality.private.json")).expect("Arquivo privado novo");
    file.write_all(&serde_json::to_vec_pretty(&report).expect("JSON")).expect("Gravação");
    // PDF real do renderer Rust sobre o mesmo CNIS, gravado só na área privada.
    let case = previus_workspace::Calculation {
        kind: "RGPS".into(),
        name: "Conferência privada da qualidade de segurado".into(),
        identity: cnis.identity.clone(),
        cnis: Some(cnis),
        dib: "2026-10-20".into(),
        ..Default::default()
    };
    let workspace = previus_workspace::Workspace::default();
    let bytes = previus_reports::render_pdf(&case, &workspace, previus_reports::ReportOptions::new(previus_reports::ReportKind::Quality, false), None)
        .expect("PDF real do renderer");
    let mut pdf = OpenOptions::new().write(true).create_new(true)
        .open(output.join("quality.private.pdf")).expect("PDF privado novo");
    pdf.write_all(&bytes).expect("Gravação do PDF");
    println!(
        "{{\"status\":\"ok\",\"counted_months\":{},\"overlap_months\":{},\"benefit_months\":{},\"operator_months\":{},\"required_carencia\":{},\"service_days_30_360\":{},\"service_years\":{},\"service_months\":{},\"service_days\":{}}}",
        summary.total_counted_months, summary.total_overlap_months, summary.total_benefit_months,
        summary.total_disregarded_months, summary.required_carencia_months, days, days / 360, (days % 360) / 30, days % 30
    );
}
