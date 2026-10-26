//! Native A4 PDF reconstruction, not an identical copy of the proprietary report.
//! The supplied outcome must be freshly recomputed by the caller after review guards.
//! Stored Calculation::result is deliberately never consulted.
mod model;
mod layout;
mod typst_engine;
mod format;
mod report_data;
use previus_workspace::{Calculation, Workspace};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReportKind { Detailed, Summary, Periods, Salaries, Memory, Transitions, Indicators, Quality, Analysis }
impl ReportKind {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "detailed" => Ok(Self::Detailed), "summary" => Ok(Self::Summary),
            "periods" => Ok(Self::Periods), "salaries" => Ok(Self::Salaries),
            "memory" => Ok(Self::Memory), "transitions" => Ok(Self::Transitions),
            "indicators" => Ok(Self::Indicators), "quality" => Ok(Self::Quality),
            "analysis" => Ok(Self::Analysis),
            _ => Err("Tipo de relatório PDF inválido.".into()),
        }
    }
    fn title(self) -> &'static str {        match self {
            Self::Detailed => "Relatório detalhado do cálculo previdenciário",
            Self::Summary => "Resumo do cálculo RGPS",
            Self::Periods => "Relatório do Tempo de Contribuição",
            Self::Salaries => "Salários de contribuição",
            Self::Memory => "Cálculo da R.M.I.",
            Self::Transitions => "Análise das Regras de Transição - EC 103/2019",
            Self::Indicators => "Indicadores e pendências de conferência",
            Self::Quality => "Cálculo da Qualidade de Segurado",
            Self::Analysis => "Análise da Situação Atual do Benefício",
        }
    }
}
impl ReportKind {
    /// Identificador estável usado pelo motor de templates.
    pub fn slug(self) -> &'static str {
        match self {
            Self::Detailed => "detailed",
            Self::Summary => "summary",
            Self::Periods => "periods",
            Self::Salaries => "salaries",
            Self::Memory => "memory",
            Self::Transitions => "transitions",
            Self::Indicators => "indicators",
            Self::Quality => "quality",
            Self::Analysis => "analysis",
        }
    }
    /// Se este tipo já tem template Typst no novo formato.
    pub fn has_typst_template(self) -> bool {
        matches!(self, Self::Analysis | Self::Periods | Self::Transitions)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReportOptions { pub kind: ReportKind, pub include_personal: bool, pub presentation: Presentation }
impl ReportOptions {
    pub fn new(kind: ReportKind, include_personal: bool) -> Self {
        Self { kind, include_personal, presentation: Presentation::default() }
    }
}
impl Default for ReportOptions { fn default() -> Self { Self::new(ReportKind::Detailed, false) } }
/// Presentation choices of the generated document. They change what is
/// printed, never what is calculated: every excluded section stays available
/// in the other report types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Presentation { pub include_appendix: bool, pub include_quality: bool, pub include_planning: bool }
impl Default for Presentation {
    fn default() -> Self { Self { include_appendix: true, include_quality: true, include_planning: true } }
}

/// Returns the actual PDF bytes, with embedded report fonts (installed Trebuchet or licensed Inter fallback), no runtime printer.
/// Never writes files, mutates a case, calculates entitlements or trusts saved results.
/// Unknown/blocked outcomes produce a review report, not a zero-valued RMI.
pub fn render_pdf(
    case: &Calculation, workspace: &Workspace, options: ReportOptions,
    outcome: Option<&Value>,
) -> Result<Vec<u8>, String> {
    let report = model::build(case, workspace, options, outcome)?;
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| layout::render(report)))
        .map_err(|_| "Falha interna no gerador PDF; nenhum relatório válido foi produzido.".to_owned())?
}

/// Renderiza um relatório a partir dos templates Typst (novo layout).
pub fn render_typst(kind: &str, data: &Value) -> Result<Vec<u8>, String> {
    typst_engine::render(kind, data)
}

/// Renderiza um relatório do novo formato a partir do caso e do resultado nativo.
pub fn render_report(case: &Calculation, kind: &str, outcome: Option<&Value>) -> Result<Vec<u8>, String> {
    let data = report_data::build(kind, case, outcome)?;
    typst_engine::render(kind, &data)
}

/// Merge multiple PDF documents into a single output, preserving page order.
/// Each input must be a valid PDF; empty input list is an error.
pub fn merge_pdfs(parts: &[Vec<u8>]) -> Result<Vec<u8>, String> {
    if parts.is_empty() {
        return Err("Nenhum PDF para mesclar.".into());
    }
    if parts.len() == 1 {
        return Ok(parts[0].clone());
    }
    let mut base = lopdf::Document::load_mem(&parts[0])
        .map_err(|e| format!("Falha ao carregar PDF parte 0: {e}"))?;
    for (idx, bytes) in parts.iter().enumerate().skip(1) {
        if !bytes.starts_with(b"%PDF-") {
            return Err(format!("Parte {idx} nao e um PDF valido."));
        }
        let src = lopdf::Document::load_mem(bytes)
            .map_err(|e| format!("Falha ao carregar PDF parte {idx}: {e}"))?;
        let offset = base.reference_table.max_id() + 1;
        let mut mapping: std::collections::BTreeMap<lopdf::ObjectId, lopdf::ObjectId> = Default::default();
        for &old_id in src.objects.keys() {
            mapping.insert(old_id, (old_id.0 + offset, old_id.1));
        }
        for (&old_id, obj) in &src.objects {
            let new_id = mapping[&old_id];
            let mut relocated = obj.clone();
            relocate_references(&mut relocated, &mapping);
            base.set_object(new_id, relocated);
            if new_id.0 > base.max_id { base.max_id = new_id.0; }
        }
        // Find the base Pages node.
        let root_ref = base.trailer.get(b"Root").map_err(|_| "PDF base sem Root.")?
            .as_reference().map_err(|_| "Root nao e referencia.")?;
        let root_dict = base.get_object(root_ref).map_err(|_| "Root invalido.")?
            .as_dict().map_err(|_| "Root nao e dicionario.")?;
        let pages_ref = root_dict.get(b"Pages").map_err(|_| "Sem Pages.")?
            .as_reference().map_err(|_| "Pages nao e referencia.")?;
        // Copy source pages into base page tree.
        let src_page_ids: Vec<lopdf::ObjectId> = src.get_pages().values().copied().collect();
        for pid in &src_page_ids {
            let new_pid = mapping[pid];
            if let Some(page_obj) = base.objects.get_mut(&new_pid) {
                if let Ok(dict) = page_obj.as_dict_mut() {
                    dict.set("Parent", lopdf::Object::Reference(pages_ref));
                }
            }
        }
        // Append source page references to Kids and update Count.
        {
            let pages_obj = base.get_object_mut(pages_ref)
                .map_err(|_| "Paginas base invalidas.")?;
            let pages_dict = pages_obj.as_dict_mut()
                .map_err(|_| "Paginas base nao e dicionario.")?;
            let count = pages_dict.get(b"Count").ok().and_then(|c| c.as_i64().ok()).unwrap_or(0);
            pages_dict.set("Count", lopdf::Object::Integer(count + src_page_ids.len() as i64));
            let kids = pages_dict.get_mut(b"Kids")
                .map_err(|_| "Sem Kids.")?
                .as_array_mut()
                .map_err(|_| "Kids nao e array.")?;
            for pid in &src_page_ids {
                kids.push(lopdf::Object::Reference(mapping[pid]));
            }
        }
    }
    let mut out = std::io::Cursor::new(Vec::new());
    base.save_to(&mut out).map_err(|e| format!("Falha ao salvar PDF mesclado: {e}"))?;
    Ok(out.into_inner())
}

fn relocate_references(obj: &mut lopdf::Object, mapping: &std::collections::BTreeMap<lopdf::ObjectId, lopdf::ObjectId>) {
    match obj {
        lopdf::Object::Reference(id) => {
            if let Some(&new_id) = mapping.get(id) {
                *id = new_id;
            }
        }
        lopdf::Object::Array(arr) => {
            for item in arr.iter_mut() {
                relocate_references(item, mapping);
            }
        }
        lopdf::Object::Dictionary(dict) => {
            for (_key, value) in dict.iter_mut() {
                relocate_references(value, mapping);
            }
        }
        _ => {}
    }
}
#[cfg(test)]
mod merge_tests {
    use super::*;

    fn minimal_pdf(pages: u32) -> Vec<u8> {
        let mut doc = lopdf::Document::with_version("1.7");
        let mut kids = Vec::new();
        for _ in 0..pages {
            let page_id = doc.add_object(lopdf::Object::Dictionary({
                let mut d = lopdf::Dictionary::new();
                d.set("Type", lopdf::Object::Name(b"Page".to_vec()));
                d.set("MediaBox", lopdf::Object::Array(vec![
                    lopdf::Object::Integer(0), lopdf::Object::Integer(0),
                    lopdf::Object::Integer(595), lopdf::Object::Integer(842),
                ]));
                d
            }));
            kids.push(lopdf::Object::Reference(page_id));
        }
        let pages_id = doc.add_object(lopdf::Object::Dictionary({
            let mut d = lopdf::Dictionary::new();
            d.set("Type", lopdf::Object::Name(b"Pages".to_vec()));
            d.set("Kids", lopdf::Object::Array(kids));
            d.set("Count", lopdf::Object::Integer(pages as i64));
            d
        }));
        let root_id = doc.add_object(lopdf::Object::Dictionary({
            let mut d = lopdf::Dictionary::new();
            d.set("Type", lopdf::Object::Name(b"Catalog".to_vec()));
            d.set("Pages", lopdf::Object::Reference(pages_id));
            d
        }));
        doc.trailer.set("Root", lopdf::Object::Reference(root_id));
        let mut out = std::io::Cursor::new(Vec::new());
        doc.save_to(&mut out).unwrap();
        out.into_inner()
    }

    #[test]
    fn merge_empty_is_error() {
        assert!(merge_pdfs(&[]).is_err());
    }

    #[test]
    fn merge_single_returns_same_bytes() {
        let pdf = minimal_pdf(1);
        let result = merge_pdfs(&[pdf.clone()]).unwrap();
        assert_eq!(result, pdf);
    }

    #[test]
    fn merge_two_pdfs_combines_pages() {
        let a = minimal_pdf(2);
        let b = minimal_pdf(3);
        let merged = merge_pdfs(&[a, b]).unwrap();
        assert!(merged.starts_with(b"%PDF-"));
        let doc = lopdf::Document::load_mem(&merged).unwrap();
        let mut page_count = 0;
        for obj in doc.objects.values() {
            if let Ok(d) = obj.as_dict() {
                if d.get(b"Type").ok().and_then(|t| t.as_name().ok()).map(|n| n == b"Page").unwrap_or(false) {
                    page_count += 1;
                }
            }
        }
        assert_eq!(page_count, 5);
    }

    #[test]
    fn merge_rejects_non_pdf() {
        let valid = minimal_pdf(1);
        assert!(merge_pdfs(&[valid, b"not a pdf".to_vec()]).is_err());
    }
}