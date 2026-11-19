//! Audited platform boundary. No other module may use unsafe code.
//! DPAPI flags are exactly UI_FORBIDDEN: never LOCAL_MACHINE. No prompts,
//! entropy secrets, registry writes, UI, credential reads or plaintext fallback.
use crate::WorkspaceError;

#[cfg(windows)]
mod native {
    use super::WorkspaceError;
    use crate::MAX_WORKSPACE_BYTES;
    use std::{ffi::c_void, fs::{File, OpenOptions}, io,
        os::windows::{ffi::OsStrExt, fs::OpenOptionsExt}, path::Path, ptr};
    use windows_sys::Win32::{
        Foundation::{LocalFree, GENERIC_READ, GENERIC_WRITE},
        Security::Cryptography::{CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN},
        Storage::FileSystem::{MoveFileExW, DELETE, FILE_FLAG_DELETE_ON_CLOSE,
            MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH},
    };
    struct LocalBlob(CRYPT_INTEGER_BLOB);
    impl Drop for LocalBlob {
        fn drop(&mut self) {
            if !self.0.pbData.is_null() {
                // SAFETY: only DPAPI-owned output enters this guard. The API
                // allocates cbData bytes via LocalAlloc; release exactly once.
                unsafe {
                    for i in 0..self.0.cbData as usize { ptr::write_volatile(self.0.pbData.add(i), 0); }
                    LocalFree(self.0.pbData.cast::<c_void>());
                }
            }
        }
    }
    fn transform(bytes: &[u8], decrypt: bool) -> Result<Vec<u8>, WorkspaceError> {
        if bytes.is_empty() || bytes.len() > MAX_WORKSPACE_BYTES as usize { return Err(WorkspaceError::TooLarge); }
        let input = CRYPT_INTEGER_BLOB { cbData: bytes.len() as u32, pbData: bytes.as_ptr().cast_mut() };
        let mut output = LocalBlob(CRYPT_INTEGER_BLOB { cbData: 0, pbData: ptr::null_mut() });
        // SAFETY: input slice stays alive and is read-only for both APIs despite
        // the Windows blob's mutable field. Output is initialized; other pointers
        // are explicitly null. Calls are synchronous and flags prohibit prompts.
        let ok = unsafe {
            if decrypt {
                CryptUnprotectData(&input, ptr::null_mut(), ptr::null(), ptr::null(), ptr::null(), CRYPTPROTECT_UI_FORBIDDEN, &mut output.0)
            } else {
                CryptProtectData(&input, ptr::null(), ptr::null(), ptr::null(), ptr::null(), CRYPTPROTECT_UI_FORBIDDEN, &mut output.0)
            }
        };
        if ok == 0 {
            let code = io::Error::last_os_error().raw_os_error().unwrap_or(0);
            return Err(if decrypt { WorkspaceError::DecryptionFailed(code) } else { WorkspaceError::EncryptionFailed(code) });
        }
        if output.0.pbData.is_null() || output.0.cbData == 0 { return Err(WorkspaceError::InvalidEnvelope); }
        if u64::from(output.0.cbData) > MAX_WORKSPACE_BYTES { return Err(WorkspaceError::TooLarge); }
        // SAFETY: successful DPAPI returned this live allocation and length;
        // copy before LocalBlob clears and frees it. No borrowed output escapes.
        Ok(unsafe { std::slice::from_raw_parts(output.0.pbData, output.0.cbData as usize) }.to_vec())
    }
    pub fn protect(bytes: &[u8]) -> Result<Vec<u8>, WorkspaceError> { transform(bytes, false) }
    pub fn unprotect(bytes: &[u8]) -> Result<Vec<u8>, WorkspaceError> { transform(bytes, true) }
    pub fn wipe(bytes: &mut [u8]) {
        for byte in bytes {
            // SAFETY: each unique byte reference is valid; volatile writes avoid
            // elision for this buffer. Other deserialized strings remain in RAM.
            unsafe { ptr::write_volatile(byte, 0) }
        }
    }
    fn wide(path: &Path) -> Result<Vec<u16>, WorkspaceError> {
        let mut value: Vec<u16> = path.as_os_str().encode_wide().collect();
        if value.contains(&0) { return Err(WorkspaceError::InvalidPath); }
        value.push(0); Ok(value)
    }
    pub fn atomic_move(from: &Path, to: &Path, replace: bool) -> Result<(), WorkspaceError> {
        let source = wide(from)?; let destination = wide(to)?;
        let flags = MOVEFILE_WRITE_THROUGH | if replace { MOVEFILE_REPLACE_EXISTING } else { 0 };
        // SAFETY: both UTF-16 strings have one final NUL and stay live for the
        // synchronous call. Storage ensures same canonical parent, no copy flag,
        // no reboot flag, and an owned new temporary source (not a user file).
        if unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), flags) } == 0 {
            return Err(WorkspaceError::io("atomic_replace", io::Error::last_os_error()));
        }
        Ok(())
    }
    pub fn lock_new(path: &Path) -> Result<File, WorkspaceError> {
        // create_new means an unrelated pre-existing lock is NEVER overwritten
        // or removed. OS deletes only this newly created lock on handle close,
        // including process termination. No stale-lock age heuristic is used.
        OpenOptions::new().read(true).write(true).create_new(true).share_mode(0)
            .access_mode(GENERIC_READ | GENERIC_WRITE | DELETE)
            .custom_flags(FILE_FLAG_DELETE_ON_CLOSE).open(path)
            .map_err(|e| if e.kind() == io::ErrorKind::AlreadyExists || matches!(e.raw_os_error(), Some(32) | Some(5)) {
                WorkspaceError::Busy
            } else { WorkspaceError::io("acquire_lock", e) })
    }
}
#[cfg(windows)]
pub(crate) use native::*;

#[cfg(not(windows))]
pub(crate) fn protect(_: &[u8]) -> Result<Vec<u8>, WorkspaceError> { Err(WorkspaceError::Unsupported) }
#[cfg(not(windows))]
pub(crate) fn unprotect(_: &[u8]) -> Result<Vec<u8>, WorkspaceError> { Err(WorkspaceError::Unsupported) }
#[cfg(not(windows))]
pub(crate) fn wipe(bytes: &mut [u8]) { bytes.fill(0); }
#[cfg(not(windows))]
pub(crate) fn atomic_move(_: &std::path::Path, _: &std::path::Path, _: bool) -> Result<(), WorkspaceError> { Err(WorkspaceError::Unsupported) }
#[cfg(not(windows))]
pub(crate) fn lock_new(_: &std::path::Path) -> Result<std::fs::File, WorkspaceError> { Err(WorkspaceError::Unsupported) }
