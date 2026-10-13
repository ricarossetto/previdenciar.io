//! A private workspace, not a benefit-entitlement or RMI calculation engine.
//! No autosave, network, PDF copying, credential lookup or personal-data logging.
#![deny(unsafe_code)]
mod model;
pub mod contributions;
pub mod link_review;
mod report;
mod storage;
// Only this small FFI boundary is allowed to contain audited unsafe operations.
#[allow(unsafe_code)]
mod crypt;

pub use model::*;
pub use report::{export_salary_csv, report_html};
pub use storage::{WorkspaceError, MAX_WORKSPACE_BYTES};
