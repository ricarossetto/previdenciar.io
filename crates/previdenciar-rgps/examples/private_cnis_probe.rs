//! Private diagnostic: import a CNIS PDF and print aggregate counts only.
use std::path::PathBuf;

fn main() {
    let path = PathBuf::from(std::env::args_os().nth(1).expect("caminho do CNIS"));
    let cnis = previdenciar_cnis::import_pdf(&path).expect("importar CNIS");
    let mut links_sem_fim = 0;
    let mut links_prpps = 0;
    let mut links_com_indicador = 0;
    let mut afiliacoes = std::collections::BTreeMap::new();
    for link in &cnis.links {
        if link.end_date.is_none() { links_sem_fim += 1; }
        if link.prpps { links_prpps += 1; }
        if !link.indicators.is_empty() { links_com_indicador += 1; }
        *afiliacoes.entry(link.affiliation.clone().unwrap_or_else(|| "?".into())).or_insert(0usize) += 1;
    }
    let meses = cnis.salary_months.len();
    let mut meses_concomitantes = 0;
    let mut meses_prpps = 0;
    let mut meses_com_indicador = 0;
    let mut meses_abaixo_minimo = 0;
    for (competencia, mes) in &cnis.salary_months {
        if mes.count > 1 { meses_concomitantes += 1; }
        if mes.contains_unresolved_prpps { meses_prpps += 1; }
        let tem_indicador = cnis.salary_entries.iter().any(|entry| entry.competence == *competencia && !entry.indicators.is_empty());
        if tem_indicador { meses_com_indicador += 1; }
        if competencia.as_str() >= "1994-07" {
            if let Some((_, minimo)) = rgps_core::dados::salario_minimo::SALARIO_MINIMO.iter().filter(|(periodo, _)| *periodo <= competencia.as_str()).last() {
                if let Ok(minimo) = minimo.parse::<f64>() {
                    if (mes.amount_sum_centavos as f64 / 100.0) < minimo { meses_abaixo_minimo += 1; }
                }
            }
        }
    }
    println!("links={} prpps={} sem_fim={} com_indicador={} afiliacoes={:?}", cnis.links.len(), links_prpps, links_sem_fim, links_com_indicador, afiliacoes);
    println!("meses={} concomitantes={} prpps={} com_indicador={} abaixo_minimo={}", meses, meses_concomitantes, meses_prpps, meses_com_indicador, meses_abaixo_minimo);
    println!("warnings={:?}", cnis.warnings.iter().map(|w| w.code.as_str()).collect::<Vec<_>>());
    println!("beneficios={}", cnis.benefits.len());
    for b in &cnis.benefits { println!("beneficio seq={} esp={:?} status={:?} {:?}..{:?} como_vinculo={}", b.sequence, b.species, b.status, b.start_date, b.end_date, b.included_as_employment_link); }
    for l in cnis.links.iter().filter(|l| l.affiliation.as_deref().is_some_and(|a| a.to_lowercase().contains("benef"))) { println!("link_beneficio {} {:?}..{:?} {:?}", l.sequence, l.start_date, l.end_date, l.affiliation); }
    let case = previdenciar_workspace::Calculation {
        identity: cnis.identity.clone(),
        cnis: Some(cnis.clone()),
        sex: Some(std::env::args().nth(2).unwrap_or_else(|| "feminino".into())),
        disability: std::env::args().nth(4),
        dib: std::env::args().nth(3).unwrap_or_else(|| "2026-10-20".into()),
        kind: "RGPS".into(),
        ..Default::default()
    };
    match previdenciar_rgps::prepare_from_cnis(&case, "Conferência privada do CNIS em 2026-10-23") {
        Ok((prepared, notes)) => {
            for note in &notes { println!("nota: {note}"); }
            for link in &prepared.manual_links {
                println!("vinculo_preparado {}..{} {}", link.start_date, link.end_date, link.employer.chars().take(30).collect::<String>());
            }
            let nao_cobertos: Vec<_> = prepared.manual_salaries.iter().filter(|wage| {
                !prepared.manual_links.iter().any(|link| wage.competence.as_str() >= &link.start_date[..7] && wage.competence.as_str() <= &link.end_date[..7])
            }).map(|wage| wage.competence.clone()).collect();
            println!("nao_cobertos={:?}", nao_cobertos);            println!("preparado: vinculos={} salarios={}", prepared.manual_links.len(), prepared.manual_salaries.len());
            if let Ok(out) = std::env::var("PROBE_CASE_OUT") {
                std::fs::write(&out, serde_json::to_vec_pretty(&prepared).unwrap()).expect("gravar caso privado");
            }
            match previdenciar_rgps::calculate_case(&prepared) {
                Ok(result) => {
                    println!("status={} rmi={}", result["status"], result["data"]["native_result"]["resumo"]["rmi"]);
                    for blocker in result["blockers"].as_array().into_iter().flatten() { println!("bloqueio: {} {}", blocker["code"], blocker["message"]); }
                    if std::env::var("PROBE_DUMP").is_ok() {
                        let mut v = result["data"]["native_result"].clone();
                        if let Some(o) = v.as_object_mut() { o.remove("salarios"); o.remove("competencias"); o.remove("memoria"); }
                        println!("PROJECAO {}", serde_json::to_string_pretty(&result["data"]["projecao_regras"]).unwrap_or_default());
                        println!("RESUMO {}", serde_json::to_string_pretty(&v["resumo"]).unwrap_or_default());
                        println!("CHAVES {:?}", v.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()));
                        println!("TEMPO {}", serde_json::to_string(&v["tempo"]).unwrap_or_default());
                        println!("TRANSICOES {}", serde_json::to_string_pretty(&v["transicoes"]).unwrap_or_default().chars().take(6000).collect::<String>());
                        std::process::exit(0);
                    }
                    for blocker in result["blockers"].as_array().into_iter().flatten() {
                        println!("bloqueio: {} {}", blocker["code"], blocker["message"]);
                    }
                }
                Err(error) => println!("erro_motor: {error}"),
            }
        }
        Err(error) => println!("erro_preparo: {error}"),
    }    for link in cnis.links.iter().take(20) {
        println!("link {} {}..{} afiliacao={:?} prpps={} indicadores={:?} fim={:?}", link.sequence, link.start_date.as_deref().unwrap_or("?"), link.end_date.as_deref().unwrap_or("?"), link.affiliation, link.prpps, link.indicators, link.inferred_last_salary_end.as_ref().map(|v| v.date_candidate.clone()));
    }
}
