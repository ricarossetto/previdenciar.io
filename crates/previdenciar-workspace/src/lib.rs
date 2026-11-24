//! A private workspace, not a benefit-entitlement or RMI calculation engine.
//! No autosave, network, PDF copying, credential lookup or personal-data logging.
#![deny(unsafe_code)]
mod model;
pub mod contributions;
pub mod evidence;
pub mod link_review;
mod report;
mod storage;
// Only this small FFI boundary is allowed to contain audited unsafe operations.
#[allow(unsafe_code)]
mod crypt;

pub use model::*;
pub use report::{export_salary_csv, report_html};
pub use storage::{WorkspaceError, MAX_WORKSPACE_BYTES};

/// Narrow DPAPI boundary for small, non-workspace secrets (for example API
/// keys). It reuses the audited crypt module and never adds a plaintext
/// fallback. Callers remain responsible for keeping secret lifetimes short.
pub mod local_secret {
    use super::WorkspaceError;
    use std::path::Path;

    pub fn protect(plaintext: &[u8]) -> Result<Vec<u8>, WorkspaceError> {
        super::crypt::protect(plaintext)
    }

    pub fn unprotect(ciphertext: &[u8]) -> Result<Vec<u8>, WorkspaceError> {
        super::crypt::unprotect(ciphertext)
    }

    pub fn wipe(bytes: &mut [u8]) {
        super::crypt::wipe(bytes)
    }

    /// Same-directory atomic replacement with the audited Windows flags.
    pub fn atomic_replace(from: &Path, to: &Path) -> Result<(), WorkspaceError> {
        super::crypt::atomic_move(from, to, true)
    }
}
