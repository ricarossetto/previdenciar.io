//! Verifica que cada relatório do novo motor gera um PDF válido a partir de um
//! caso calculado real (fixture sintética).
#[path = "common/mod.rs"]
mod common;

#[test]
fn every_typst_report_renders_a_valid_pdf() {
    let (case, _workspace, outcome) = common::calculated_case();
    for kind in [
        "analysis", "periods", "transitions", "memory", "quality", "roi", "regras", "summary",
        "salaries", "indicators",
    ] {
        let pdf = previus_reports::render_report(&case, kind, Some(&outcome))
            .unwrap_or_else(|e| panic!("{kind} falhou: {e}"));
        assert!(pdf.starts_with(b"%PDF-"), "{kind} nao produziu PDF");
        assert!(pdf.len() > 1000, "{kind} produziu PDF vazio");
    }
}

#[test]
fn consolidated_merges_all_parts() {
    let (case, _workspace, outcome) = common::calculated_case();
    let pdf = previus_reports::render_report(&case, "detailed", Some(&outcome)).unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
    let doc = lopdf::Document::load_mem(&pdf).unwrap();
    assert!(doc.get_pages().len() > 6, "consolidado deve ter mais de 6 paginas");
}

#[test]
fn brazilian_formatting_is_correct() {
    use previus_reports::format::{br_date, br_number, period};
    assert_eq!(br_number("1632377.53"), "1.632.377,53");
    assert_eq!(br_number("0.00"), "0,00");
    assert_eq!(br_date("2026-05-22"), "22/05/2026");
    assert_eq!(period(22, 1, 4), "22 anos, 1 mês e 4 dias");
    assert_eq!(period(1, 0, 0), "1 ano");
    assert_eq!(period(0, 0, 5), "5 dias");
}
