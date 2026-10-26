#[path = "../tests/common/mod.rs"]
mod common;

fn main() {
    let kind = std::env::args().nth(2).unwrap_or_else(|| "analysis".into());
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "typst-real.pdf".into());
    let (case, _workspace, outcome) = common::calculated_case();
    match previus_reports::render_report(&case, &kind, Some(&outcome)) {
        Ok(pdf) => {
            std::fs::write(&out, &pdf).expect("write");
            println!("{{\"status\":\"ok\",\"kind\":\"{kind}\",\"bytes\":{}}}", pdf.len());
        }
        Err(e) => {
            eprintln!("ERRO: {e}");
            std::process::exit(1);
        }
    }
}
