//! Native A4 PDF reconstruction, not an identical copy of the proprietary report.
//! The supplied outcome must be freshly recomputed by the caller after review guards.
//! Stored Calculation::result is deliberately never consulted.
mod model;
mod layout;
use previus_workspace::{Calculation, Workspace};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReportKind { Detailed, Summary, Periods, Salaries, Memory, Transitions, Indicators, Quality }
impl ReportKind {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "detailed" => Ok(Self::Detailed), "summary" => Ok(Self::Summary),
            "periods" => Ok(Self::Periods), "salaries" => Ok(Self::Salaries),
            "memory" => Ok(Self::Memory), "transitions" => Ok(Self::Transitions),
            "indicators" => Ok(Self::Indicators), "quality" => Ok(Self::Quality),
            _ => Err("Tipo de relatório PDF inválido.".into()),
        }
    }
    fn title(self) -> &'static str {
        match self {
            Self::Detailed => "Relatório detalhado do cálculo previdenciário", Self::Summary => "Resumo do cálculo RGPS",
            Self::Periods => "Relatório do Tempo de Contribuição", Self::Salaries => "Salários de contribuição",
            Self::Memory => "Cálculo da R.M.I.", Self::Transitions => "Análise das regras de transição - EC 103/2019",
            Self::Indicators => "Indicadores e pendências de conferência",
            Self::Quality => "Cálculo da Qualidade de Segurado",
        }
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
