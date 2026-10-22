#[path = "../tests/common/mod.rs"] mod common;
use previus_reports::{render_pdf, ReportKind, ReportOptions};
use previus_workspace::{Calculation, Workspace};
use std::{fs::OpenOptions, io::Write, path::Path};

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        return Err("Uso:\n  report_example conference <kind> <novo-arquivo.pdf>\n  report_example positive-reviewed <kind> <novo-arquivo.pdf>\n  report_example detailed_real <caminho-cnis-autorizado.pdf> <novo-arquivo.pdf>".into());
    }
    match args[0].as_str() {
        "conference" | "positive-reviewed" => {
            if args.len() != 3 && args.len() != 4 {
                return Err("Argumentos: <conference|positive-reviewed> <kind> <novo-arquivo.pdf> [título do relatório]".into());
            }
            let kind = ReportKind::parse(&args[1])?;
            let (mut case, workspace, outcome) = if args[0] == "positive-reviewed" {
                let (c, w, o) = common::calculated_case();
                (c, w, Some(o))
            } else {
                let (c, w) = common::synthetic_case();
                (c, w, None)
            };
            if let Some(title) = args.get(3) { case.options.report_title = title.clone(); }
            render_and_save(&case, &workspace, ReportOptions::new(kind, false), outcome.as_ref(), &args[2], &args[0], &args[1], "synthetic_only")
        }
        "detailed_real" => {
            if args.len() != 3 {
                return Err("Argumentos: detailed_real <caminho-cnis-autorizado.pdf> <novo-arquivo.pdf>".into());
            }
            // User explicitly authorized reading this file; outcome is None (conference only, no invented RMI).
            let input_path = Path::new(&args[1]);
            let import_result = previus_cnis::import_pdf(input_path).map_err(|e| format!("Falha ao ler CNIS: {e}"))?;
            let mut case = Calculation {
                name: "Conferência técnica de importação".into(),
                kind: "rgps".into(),
                identity: import_result.identity.clone(),
                cnis: Some(import_result),
                dib: "2026-10-10".into(),
                ..Default::default()
            };
            case.notes = String::new(); // Private notes suppressed
            let workspace = Workspace::default();
            // PRIVATE report options hardcoded to false for real document; never embed personal source file path.
            let options = ReportOptions::new(ReportKind::Detailed, false);
            render_and_save(&case, &workspace, options, None, &args[2], "detailed_real", "detailed", "authorized_real_source_private_report")
        }
        _ => Err("Modo desconhecido. Use conference, positive-reviewed ou detailed_real.".into()),
    }
}

fn render_and_save(
    case: &Calculation,
    workspace: &Workspace,
    options: ReportOptions,
    outcome: Option<&serde_json::Value>,
    output_path_str: &str,
    mode: &str,
    kind_str: &str,
    fixtures_label: &str,
) -> Result<(), String> {
    let output_path = Path::new(output_path_str);
    let started = std::time::Instant::now();
    let bytes = render_pdf(case, workspace, options, outcome)?;
    let (doc, text) = common::extract(&bytes);
    let mut f = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output_path)
        .map_err(|_| "Não foi possível criar arquivo novo; destino existente nunca é sobrescrito.".to_string())?;
    f.write_all(&bytes)
        .and_then(|_| f.sync_all())
        .map_err(|_| "Falha ao gravar arquivo PDF.".to_string())?;
    println!(
        "{}",
        serde_json::json!({
            "status": "created",
            "mode": mode,
            "kind": kind_str,
            "pages": doc.get_pages().len(),
            "bytes": bytes.len(),
            "extracted_characters": text.chars().count(),
            "elapsed_ms": started.elapsed().as_millis(),
            "fixtures": fixtures_label,
            "legal_validation_performed": false,
            "rmi_produced": outcome.is_some()
        })
    );
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
