//! Diagnóstico privado: roda o cenário PcD (LC 142) sobre um CNIS autorizado
//! com o snapshot montado a partir das séries oficiais embutidas e imprime
//! apenas valores agregados. Nada sai para a tela além de números.
use previus_workspace::Calculation;
use serde_json::{json, Value};
use std::path::PathBuf;

fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() < 3 {
        eprintln!("Uso: private_pcd_probe <cnis.pdf> <dib AAAA-MM-DD> <sexo> <grau> <politica:floor|preserve> [saida-privada]");
        std::process::exit(2);
    }
    let source = PathBuf::from(&args[0]);
    let dib = args[1].to_str().expect("dib").to_string();
    let sex = args[2].to_str().expect("sexo").to_string();
    let grade = args[3].to_str().expect("grau").to_string();
    let negative = args[4].to_str().expect("politica").to_string();
    let cnis = previus_cnis::import_pdf(&source).expect("Leitor local do CNIS");
    let correction_month = format!("{}-{}", &dib[..4], &dib[5..7]);
    let snapshot = previus_rgps::pcd::local_snapshot_with_minimum(&correction_month, Some("1621.00")).expect("Snapshot local");
    let policy = json!({
        "lc142": {
        "version": previus_rgps::pcd::VERSION,
        "calculation_date": dib,
        "correction_month": correction_month,
        "assumptions_confirmed": true,
        "review_evidence": "Documento CNIS autorizado e conferência manual do caso real, em área privada.",
        "disability_all_periods": true,
        "disability_evidence": "Grau constante declarado no documento de benefício do caso real, em área privada.",
        "rgps_confirmed": true,
        "project_open_links": false,
        "ongoing_evidence": "",
        "fill_missing_minimum": false,
        "missing_salary_evidence": "Competência sem salário no CNIS lida como valor consolidado anual, quando houver.",
        "project_at_known_ceiling": false,
        "projection_evidence": "",
        "historical_negative_policy": negative,
        "apply_inss_overrides": false,
        "snapshot": snapshot,
        }
    });
    let mut case = Calculation {
        kind: "RGPS".into(),
        name: "Conferência privada do cenário PcD".into(),
        identity: cnis.identity.clone(),
        cnis: Some(cnis),
        sex: Some(sex),
        disability: Some(grade),
        dib: dib.clone(),
        ..Default::default()
    };
    case.options.rule_policy = serde_json::to_string(&policy).expect("política");
    let outcome = previus_rgps::calculate_case(&case).expect("motor");
    let resumo = outcome["data"]["native_result"]["resumo"].clone();
    let correcao = outcome["data"]["native_result"]["correcao"].clone();
    let tempo = outcome["data"]["native_result"]["tempo"].clone();
    let print = |label: &str, value: &Value| println!("{label}: {}", value.as_str().map(str::to_string).unwrap_or_else(||value.to_string()));
    println!("status: {}", outcome["status"].as_str().unwrap_or("?"));
    print("pbc_parcelas", &correcao["pbc"]);
    print("consideradas_80porcento", &correcao["consideradas"]);
    print("descartadas", &correcao["descartadas"]);
    print("soma_considerada", &correcao["soma_considerada"]);
    print("media_salario_beneficio", &correcao["media"]);
    print("carencia_meses", &resumo["carencia_meses"]);
    print("tempo_total_dias_30_360", &tempo["total"]["days_30_360"]);
    print("tempo_anos", &tempo["total"]["years"]);
    print("tempo_meses", &tempo["total"]["months"]);
    print("tempo_dias", &tempo["total"]["days"]);
    print("rmi", &resumo["rmi"]);
    print("coeficiente", &resumo["coeficiente"]);
    print("tempo_atingido_no_cenario", &outcome["data"]["native_result"]["pcd"]["tempo_atingido_no_cenario"]);
    let blockers = outcome["blockers"].as_array().map(|items| items.len()).unwrap_or(0);
    println!("bloqueios: {blockers}");
    if std::env::var("DUMP_PARCELAS").is_ok(){ if let Some(items)=correcao["parcelas"].as_array(){ let mut out=String::new(); for row in items { out.push_str(&format!("{} {} {} {}\n", row["competencia"].as_str().unwrap_or("?"), row["salario"].as_str().unwrap_or("?"), row["indice"].as_str().unwrap_or("?"), row["corrigido"].as_str().unwrap_or("?"))); } std::fs::write(std::env::var("DUMP_PARCELAS").unwrap(),out).expect("dump"); } }
    for item in outcome["blockers"].as_array().into_iter().flatten(){ println!("  - {}: {}", item["code"].as_str().unwrap_or("?"), item["message"].as_str().unwrap_or("?")); }
}
