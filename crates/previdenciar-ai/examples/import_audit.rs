use previdenciar_ai::import_audit;
use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    let source = args.next().map(PathBuf::from).unwrap_or_else(|| {
        eprintln!("Uso: cargo run -p previdenciar-ai --example import_audit -- <origem> [destino] [--skip-pdf]");
        std::process::exit(2);
    });
    let mut destination = None;
    let mut include_pdf = true;
    for arg in args {
        match arg.as_str() {
            "--skip-pdf" => include_pdf = false,
            _ => destination = Some(PathBuf::from(arg)),
        }
    }
    let destination = destination.unwrap_or_else(|| {
        std::env::var_os("LOCALAPPDATA")
            .map(|base| PathBuf::from(base).join("Previdenciar").join("conhecimento"))
            .unwrap_or_else(|| PathBuf::from("conhecimento"))
    });
    match import_audit(&source, &destination, include_pdf) {
        Ok(summary) => {
            println!(
                "importado: {} habilidades, {} normas, {} NatJus ({} com texto, {} falhas), {} ementas",
                summary.skills,
                summary.legislation,
                summary.natjus,
                summary.natjus_text,
                summary.natjus_text_failures,
                summary.jurisprudence
            );
            println!("destino: {}", summary.destination);
        }
        Err(error) => {
            eprintln!("ERRO: {error}");
            std::process::exit(1);
        }
    }
}
