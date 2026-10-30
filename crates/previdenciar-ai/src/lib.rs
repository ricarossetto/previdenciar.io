#![deny(unsafe_code)]

mod agent;
mod error;
mod importer;
mod providers;
mod secrets;
mod tools;

pub use agent::{run, AiEvent, ChatMessage, ChatRequest, MinutaTask, Usage};
pub use error::{AiError, Result};
pub use importer::{extract_case_pdf, extract_pdf_text, import_audit, ExtractedPdf, ImportSummary};
pub use providers::{
    catalog, chat_endpoint, key_required, list_models, models_endpoint, preset_by_id,
    test_connection, ProviderKind, ProviderPreset, TestResult,
};
pub use secrets::{KeyStatus, SecretVault};
pub use tools::{DataStatus, DataStore, KnowledgeBlock, SourceRef, ToolDef, ToolOutcome};
