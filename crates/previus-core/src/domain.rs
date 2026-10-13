use crate::{CalculationOutcome, CivilDate, Money, Warning, YearMonth};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Neutral logical records, NOT a recovered PostgreSQL/HFSQL schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Regime { Rgps, Rpps, Unknown }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Indicator {
    pub code: String,
    /// Preserved upstream notes; unresolved is intentionally true until review.
    pub unresolved: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRef {
    /// Opaque source identifier; never a filename, personal ID, token or full PDF.
    pub source_id: String,
    pub page: Option<u32>,
    pub line: Option<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SalaryMonth {
    pub competence: YearMonth,
    pub amount: Option<Money>,
    pub link_id: Option<String>,
    pub indicators: Vec<Indicator>,
    pub source: Option<SourceRef>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributionLink {
    pub id: String,
    pub start: Option<CivilDate>,
    /// None does not imply open, ended, or a manufactured end date.
    pub end: Option<CivilDate>,
    pub regime: Regime,
    pub indicators: Vec<Indicator>,
    pub source: Option<SourceRef>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportedRecords {
    pub salary_months: Vec<SalaryMonth>,
    pub links: Vec<ContributionLink>,
    pub indicators: Vec<Indicator>,
    /// Parser/reviewer warnings survive the boundary; do not discard on validation.
    pub warnings: Vec<Warning>,
}
impl ImportedRecords {
    pub fn distinct_salary_months(&self) -> usize {
        self.salary_months.iter().map(|r| r.competence).collect::<BTreeSet<_>>().len()
    }
    /// Nonmutating review: no deduplication, automatic reconciliation or CTC choice.
    /// Counts/positions rather than personal values are used in messages.
    pub fn review_warnings(&self) -> Vec<Warning> {
        let mut warnings = self.warnings.clone();
        let ids: BTreeSet<&str> = self.links.iter().map(|v| v.id.as_str()).collect();
        if ids.len() != self.links.len() {
            warnings.push(Warning::new("duplicate_link_id", "Identificadores de vínculos duplicados; conciliação pendente."));
        }
        if self.links.iter().any(|v| v.id.trim().is_empty()) {
            warnings.push(Warning::new("missing_link_id", "Vínculo sem identificador lógico."));
        }
        if self.distinct_salary_months() < self.salary_months.len() {
            warnings.push(Warning::new("repeated_competence", "Mais de um lançamento na mesma competência foi preservado; não se presume duplicidade ou concomitância resolvida."));
        }
        for link in &self.links {
            if link.regime == Regime::Rpps || link.indicators.iter().any(|i| i.code == "PRPPS") {
                warnings.push(Warning::new("rpps_review_required", "Vínculo/indicador de regime próprio preservado; CTC e aproveitamento dependem de revisão."));
            }
            if link.regime == Regime::Unknown {
                warnings.push(Warning::new("unknown_regime", "Regime não identificado; não convertido automaticamente para RGPS."));
            }
            match (link.start, link.end) {
                (Some(start), Some(end)) if end < start => warnings.push(Warning::new("reversed_link", "Vínculo com intervalo invertido; preservado para revisão.")),
                (None, _) | (_, None) => warnings.push(Warning::new("incomplete_link_dates", "Data de vínculo ausente; não preenchida por suposição.")),
                _ => {}
            }
        }
        for salary in &self.salary_months {
            match salary.amount {
                None => warnings.push(Warning::new("missing_salary", "Valor ausente preservado como desconhecido, não como zero.")),
                Some(money) if money.cents() < 0 => warnings.push(Warning::new("negative_salary", "Valor salarial negativo requer revisão; não foi corrigido.")),
                _ => {}
            }
            if salary.link_id.as_deref().is_some_and(|id| !ids.contains(id)) {
                warnings.push(Warning::new("unknown_salary_link", "Lançamento referencia vínculo não encontrado."));
            }
            if salary.link_id.is_none() {
                warnings.push(Warning::new("missing_salary_link", "Lançamento sem associação de vínculo; associação não inventada."));
            }
        }
        let indicators = self.indicators.iter()
            .chain(self.links.iter().flat_map(|v| v.indicators.iter()))
            .chain(self.salary_months.iter().flat_map(|v| v.indicators.iter()));
        for indicator in indicators {
            if indicator.code == "PRPPS" {
                warnings.push(Warning::new("prpps_preserved", "Indicador PRPPS preservado; não removido nem resolvido automaticamente."));
            }
            if indicator.unresolved {
                warnings.push(Warning::new("unresolved_indicator", "Indicador importado não resolvido; revisão humana e regra pendentes."));
            }
        }
        warnings
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnsupportedCalculationKind { BenefitEntitlement, Rmi, Ctc, ContributionTime }
/// There is deliberately no result value type for these unrecovered rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnsupportedResult {}
/// Keeps every parser/domain warning while refusing entitlement/RMI/CTC guesses.
pub fn calculate_imported_records(records: &ImportedRecords, kind: UnsupportedCalculationKind) -> CalculationOutcome<UnsupportedResult> {
    let mut warnings = records.review_warnings();
    warnings.push(Warning::new("unsupported_calculation", format!("Operação {:?} indisponível no núcleo experimental.", kind)));
    CalculationOutcome::unsupported("import_review_v1", warnings)
}
