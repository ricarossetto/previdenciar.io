//! JSON boundary for desktop integration. Does not duplicate any calculation.
//! Errors are deliberately generic and contain no supplied values or personal data.
use crate::{dates::{count_inclusive_calendar_days, InclusiveIntervalInput},
    domain::{calculate_imported_records, ImportedRecords, UnsupportedCalculationKind},
    pension::{calculate_pension_accumulation, PensionAccumulationInput},
    quality::{calculate_facultative_quality, QualityOfInsuredInput}};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

/// Stable tool IDs and native input structs:
/// - "inclusive_dates": InclusiveIntervalInput {start,end} (ISO strings)
/// - "facultative_quality": QualityOfInsuredInput
/// - "pension_accumulation": PensionAccumulationInput
/// - "import_review": {records: ImportedRecords, kind: UnsupportedCalculationKind}
/// Outputs are CalculationOutcome envelopes; UI MUST retain/show their warnings.
/// No defaulting, fuzzy tool selection or argument guessing occurs here.
pub fn run_tool(tool: &str, input: Value) -> Result<Value, String> {
    match tool {
        "update_values" | "irrf_refund" | "irrf_oficial" | "undue_discounts" | "financial_formula"
        | "currency_conversion" | "benefit_adjustment" => crate::finance::run_finance(tool, input),
        "inclusive_dates" => {
            let input: InclusiveIntervalInput = decode(input)?;
            encode(count_inclusive_calendar_days(&input).map_err(|e| e.to_string())?)
        },
        "bpc_income" => {
            let input: crate::bpc::BpcIncomeInput = decode(input)?;
            encode(crate::bpc::calculate_bpc_income(&input).map_err(|error| error.to_string())?)
        },
        "arrears" => {
            let input: crate::prev_tools::ArrearsInput = decode(input)?;
            encode(crate::prev_tools::arrears(&input).map_err(|error| error.to_string())?)
        },
        "pension_split" => {
            let input: crate::prev_tools::PensionSplitInput = decode(input)?;
            encode(crate::prev_tools::pension_split(&input).map_err(|error| error.to_string())?)
        },
        "incapacity" => {
            let input: crate::prev_tools::IncapacityInput = decode(input)?;
            encode(crate::prev_tools::incapacity(&input).map_err(|error| error.to_string())?)
        },
        "special_time" => {
            let input: crate::prev_tools::SpecialInput = decode(input)?;
            encode(crate::prev_tools::special_time(&input).map_err(|error| error.to_string())?)
        },
        "revision_deadline" => {
            let input: crate::prev_tools::RevisionDeadlineInput = decode(input)?;
            encode(crate::prev_tools::revision_deadline(&input).map_err(|error| error.to_string())?)
        },
        "planning_gap" => {
            let input: crate::prev_tools::PlanningInput = decode(input)?;
            encode(crate::prev_tools::planning(&input).map_err(|error| error.to_string())?)
        },
        "triage" => {
            let input: crate::prev_tools::TriageInput = decode(input)?;
            encode(crate::prev_tools::triage(&input).map_err(|error| error.to_string())?)
        },
        "quality_preview" => {
            let input: crate::quality_preview::QualityPreviewInput = decode(input)?;
            encode(crate::quality_preview::quality_preview(&input).map_err(|error| error.to_string())?)
        },
        "facultative_quality" => {
            let input: QualityOfInsuredInput = decode(input)?;
            encode(calculate_facultative_quality(&input).map_err(|e| e.to_string())?)
        },
        "pension_accumulation" => {
            let input: PensionAccumulationInput = decode(input)?;
            encode(calculate_pension_accumulation(&input).map_err(|e| e.to_string())?)
        },
        "import_review" => {
            let input: ImportReviewInput = decode(input)?;
            encode(calculate_imported_records(&input.records, input.kind))
        },
        _ => Err("Ferramenta desconhecida ou não implementada.".to_owned()),
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportReviewInput { records: ImportedRecords, kind: UnsupportedCalculationKind }
fn decode<T: DeserializeOwned>(value: Value) -> Result<T, String> {
    serde_json::from_value(value).map_err(|_| "Entrada inválida ou incompleta para a ferramenta selecionada.".to_owned())
}
fn encode<T: Serialize>(value: T) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|_| "Não foi possível serializar o resultado.".to_owned())
}
