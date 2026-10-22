mod common;
use common::*;
use previus_reports::{render_pdf,ReportKind,ReportOptions};
#[test]
fn report_families_do_not_share_an_import_inventory_cover(){
    let(case,ws,outcome)=calculated_case();
    for kind in [ReportKind::Detailed,ReportKind::Summary,ReportKind::Periods,ReportKind::Salaries,ReportKind::Memory,ReportKind::Transitions,ReportKind::Indicators,ReportKind::Quality]{
        let bytes=render_pdf(&case,&ws,ReportOptions::new(kind, false),Some(&outcome)).unwrap();
        let(doc,_)=extract(&bytes);
        let first=compact(&doc.extract_text(&[1]).unwrap());
        assert!(first.contains("Nomedosegurado:"));
        assert!(first.contains("Nascimento:"));
        assert!(first.contains("DIB:"));
        assert!(!first.contains("Inventáriodocenário"),"generic cover in {kind:?}");
        if kind==ReportKind::Memory { assert!(first.contains("Remuneraçõesmanuais")); }
        if kind==ReportKind::Transitions { assert!(first.contains("comparador")); }
    }
}
#[test]
fn detailed_report_keeps_inventory_as_a_separate_technical_appendix(){
    let(case,ws)=synthetic_case();
    let bytes=render_pdf(&case,&ws,ReportOptions::new(ReportKind::Detailed, false),None).unwrap();
    let(doc,text)=extract(&bytes);
    assert!(compact(&text).contains("Anexotécnico-dados,origemealcance"), "appendix heading: {:?}", text.split("Anexo").nth(1).unwrap_or("absent").chars().take(70).collect::<String>());
    assert!(compact(&text).contains("Inventáriodocenário"));
    for page in 1..=doc.get_pages().len()as u32 {
        let text=doc.extract_text(&[page]).unwrap();
        assert!(compact(&text).contains("Nomedosegurado:"));
        assert!(text.contains("DIB:"));
    }
}
