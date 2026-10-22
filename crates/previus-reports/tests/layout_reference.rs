mod common;
use common::{compact, extract, synthetic_case};
use lopdf::{content::{Content, Operation}, Document, Object};
use previus_reports::{render_pdf, ReportKind, ReportOptions};
use previus_workspace::{Calculation, Workspace};


fn options(kind: ReportKind) -> ReportOptions { ReportOptions::new(kind, false) }
fn operations(doc: &Document, id: lopdf::ObjectId) -> Vec<Operation> {
    Content::decode(&doc.get_page_content(id).unwrap()).unwrap().operations
}
fn numbers(op: &Operation) -> Vec<f32> { op.operands.iter().map(|v| v.as_float().unwrap()).collect() }
fn close(left: f32, right: f32) -> bool { (left - right).abs() < 0.05 }
fn color(op: &Operation, rgb: [u8;3]) -> bool {
    op.operator == "rg" && op.operands.len() == 3 && numbers(op).iter().zip(rgb)
        .all(|(a,b)| (*a - f32::from(b)/255.0).abs() < 0.0001)
}

// Public coverage uses synthetic renderer fixtures, not private-reference hashes.

#[test]
fn compact_reference_style() {
    let bytes = render_pdf(&Calculation::default(), &Workspace::default(), options(ReportKind::Summary), None).unwrap();
    let (doc, text) = extract(&bytes);
    assert!(text.contains("DADOS PENDENTES"));
    assert!(text.contains("IDENTIFICAÇÃO OCULTA"));
    for id in doc.get_pages().values() {
        let ops = operations(&doc, *id);
        assert!(ops.iter().any(|op| op.operator == "Td" && numbers(op).len() == 2 && close(numbers(op)[1], 785.0)), "compact title baseline");
        let sizes: Vec<f32> = ops.iter().filter(|op| op.operator == "Tf").map(|op| op.operands[1].as_float().unwrap()).collect();
        assert!(sizes.contains(&12.0));
        assert!(sizes.iter().all(|v| *v <= 12.0), "no oversized redesign cover");
        assert!(ops.iter().filter(|op| op.operator == "w").all(|op| (numbers(op)[0] - 0.283).abs() < 0.00001));
    }
    for object in doc.objects.values() {
        if let Ok(dict) = object.as_dict() {
            if dict.get(b"Type").and_then(Object::as_name).ok() == Some(b"FontDescriptor") {
                assert!(dict.has(b"FontFile2") || dict.has(b"FontFile3"), "licensed fonts stay embedded");
            }
        }
    }
    // Optional in-memory PDFium preview; no report or image is written to disk.
    if std::env::var_os("PREVIUS_LAYOUT_PREVIEW").is_some() {
        println!("SYNTHETIC_PDF_HEX={}", bytes.iter().map(|b| format!("{b:02x}")).collect::<String>());
    }
}

#[test]
fn reference_bands_are_filled_behind_text_and_inside_body() {
    let (mut case, ws) = synthetic_case();
    case.cnis.as_mut().unwrap().salary_entries.truncate(16);
    let bytes = render_pdf(&case, &ws, options(ReportKind::Periods), None).unwrap();
    let (doc, _) = extract(&bytes);
    let mut blue_bands = 0;
    let mut gray_bands = 0;
    for id in doc.get_pages().values() {
        let ops = operations(&doc, *id);
        let first_text = ops.iter().position(|op| op.operator == "BT").unwrap();
        for (index, op) in ops.iter().enumerate() {
            if color(op, [51,67,93]) || color(op, [246,247,249]) {
                if color(op, [51,67,93]) { blue_bands += 1; } else { gray_bands += 1; }
                assert!(index < first_text, "bands must not cover glyphs");
                let rect = &ops[index+1];
                assert_eq!(rect.operator, "re");
                assert_eq!(ops[index+2].operator, "f");
                let r = numbers(rect);
                assert!(close(r[0].max(42.52), r[0]), "left margin");
                assert!(r[0]+r[2] <= 569.1, "right margin");
                assert!(r[1] >= (297.0-278.0)*72.0/25.4-0.1, "footer reserve");
                assert!(r[1]+r[3] <= (297.0-24.5)*72.0/25.4+0.1, "header reserve");
            }
        }
    }
    assert!(blue_bands > 0, "reference blue table header must actually be filled");
    assert!(gray_bands > 0, "reference alternate rows must actually be filled");
}

#[test]
fn all_synthetic_source_rows_survive_continuations_with_private_headers() {
    let (case, ws) = synthetic_case();
    let bytes = render_pdf(&case, &ws, options(ReportKind::Salaries), None).unwrap();
    let (doc, text) = extract(&bytes);
    let flat = compact(&text);
    for amount in 200000..200560 {
        let expected = format!("{}.{:03},{:02}", amount/100000, amount/100%1000, amount%100);
        assert!(flat.contains(&compact(&expected)), "missing synthetic source amount {amount}");
    }
    assert!(doc.get_pages().len() > 2);
    assert!(flat.matches("Competência").count() > 10, "actual continuation headers on many pages");
    assert_eq!(flat.matches("IDENTIFICAÇÃOOCULTA").count(), doc.get_pages().len());
    for (page, id) in doc.get_pages() {
        let ops = operations(&doc, id);
        if ops.iter().any(|op| color(op, [51,67,93])) {
            assert!(ops.iter().any(|op| color(op, [255,255,255])), "filled headers retain white text");
        }
        assert!(flat.contains(&format!("Página{page}")));
    }
    assert!(!flat.contains("CPFMARCADOR"));
    assert!(!flat.contains("EMPREGADORMARCADOR"));
    assert!(!flat.contains("ESCRITORIOMARCADOR"));
    assert!(!flat.contains("RODAPEMARCADOR"));
}
