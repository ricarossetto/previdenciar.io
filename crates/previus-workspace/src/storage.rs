use crate::{crypt, Workspace, SCHEMA_VERSION};
use std::{ffi::OsString, fmt, fs::{self, File, OpenOptions}, io::{self, Read, Write},
    path::{Component, Path, PathBuf}, sync::atomic::{AtomicU64, Ordering}, time::{SystemTime, UNIX_EPOCH}};

pub const MAX_WORKSPACE_BYTES: u64 = 64 * 1024 * 1024;
const MAGIC: &[u8; 5] = b"PVRS2";
const HEADER_BYTES: usize = 13;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Errors omit paths, personal data, JSON snippets and operating-system text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceError {
    Unsupported,
    InvalidPath,
    UnsafePath,
    TooLarge,
    InvalidEnvelope,
    InvalidData,
    UnsupportedSchema(u32),
    EncryptionFailed(i32),
    DecryptionFailed(i32),
    Io { operation: &'static str, code: Option<i32> },
    Busy,
    StaleRevision { expected: u64, actual: u64 },
    ChangedDuringSave,
    RevisionOverflow,
}
impl WorkspaceError {
    pub(crate) fn io(operation: &'static str, error: io::Error) -> Self { Self::Io { operation, code: error.raw_os_error() } }
}
impl fmt::Display for WorkspaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported => f.write_str("Armazenamento exige Windows DPAPI; não existe fallback em texto puro."),
            Self::InvalidPath => f.write_str("Selecionar caminho absoluto local válido, com diretório existente."),
            Self::UnsafePath => f.write_str("Caminho com redirecionamento/reparse point ou destino não regular foi recusado."),
            Self::TooLarge => f.write_str("Workspace excede o limite de 64 MiB."),
            Self::InvalidEnvelope => f.write_str("Envelope PVRS2 ausente, truncado ou inválido; arquivo não alterado."),
            Self::InvalidData => f.write_str("Conteúdo cifrado não é um workspace válido; arquivo não alterado."),
            Self::UnsupportedSchema(v) => write!(f, "Versão de workspace não suportada: {v}."),
            Self::EncryptionFailed(code) => write!(f, "DPAPI não conseguiu cifrar (código {code}); nenhum fallback."),
            Self::DecryptionFailed(code) => write!(f, "DPAPI não conseguiu decifrar para este usuário ou o arquivo está corrompido (código {code})."),
            Self::Io { operation, code } => write!(f, "Falha de armazenamento em {operation} (código {code:?}; caminho omitido)."),
            Self::Busy => f.write_str("Outra gravação está em andamento ou o arquivo de bloqueio está indisponível."),
            Self::StaleRevision { expected, actual } => write!(f, "Revisão desatualizada ({expected}); disco contém {actual}. Recarregar antes de gravar."),
            Self::ChangedDuringSave => f.write_str("Arquivo mudou durante a gravação; substituição recusada."),
            Self::RevisionOverflow => f.write_str("Espaço de revisões esgotado; gravação recusada."),
        }
    }
}
impl std::error::Error for WorkspaceError {}

struct Secret(Vec<u8>);
impl Drop for Secret { fn drop(&mut self) { crypt::wipe(&mut self.0); } }
struct LimitedBuffer { data: Secret, exceeded: bool }
impl Write for LimitedBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > MAX_WORKSPACE_BYTES as usize - self.data.0.len() {
            self.exceeded = true; return Err(io::Error::other("workspace_size_limit"));
        }
        self.data.0.extend_from_slice(bytes); Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}
fn encode(workspace: &Workspace) -> Result<Vec<u8>, WorkspaceError> {
    check_schema(workspace)?;
    let mut writer = LimitedBuffer { data: Secret(Vec::new()), exceeded: false };
    if serde_json::to_writer(&mut writer, workspace).is_err() {
        return Err(if writer.exceeded { WorkspaceError::TooLarge } else { WorkspaceError::InvalidData });
    }
    let encrypted = crypt::protect(&writer.data.0)?;
    if encrypted.len() > MAX_WORKSPACE_BYTES as usize - HEADER_BYTES { return Err(WorkspaceError::TooLarge); }
    let mut envelope = Vec::with_capacity(HEADER_BYTES + encrypted.len());
    envelope.extend_from_slice(MAGIC);
    envelope.extend_from_slice(&(encrypted.len() as u64).to_le_bytes());
    envelope.extend_from_slice(&encrypted); Ok(envelope)
}
fn decode(envelope: &[u8]) -> Result<Workspace, WorkspaceError> {
    if envelope.len() > MAX_WORKSPACE_BYTES as usize { return Err(WorkspaceError::TooLarge); }
    if envelope.len() <= HEADER_BYTES || &envelope[..MAGIC.len()] != MAGIC { return Err(WorkspaceError::InvalidEnvelope); }
    let mut size = [0u8; 8]; size.copy_from_slice(&envelope[5..HEADER_BYTES]);
    if u64::from_le_bytes(size) != (envelope.len() - HEADER_BYTES) as u64 { return Err(WorkspaceError::InvalidEnvelope); }
    let clear = Secret(crypt::unprotect(&envelope[HEADER_BYTES..])?);
    let workspace: Workspace = serde_json::from_slice(&clear.0).map_err(|_| WorkspaceError::InvalidData)?;
    check_schema(&workspace)?; Ok(workspace)
}
fn check_schema(workspace: &Workspace) -> Result<(), WorkspaceError> {
    if workspace.schema_version != SCHEMA_VERSION { Err(WorkspaceError::UnsupportedSchema(workspace.schema_version)) } else { Ok(()) }
}

fn regular_or_missing(path: &Path) -> Result<bool, WorkspaceError> {
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            #[cfg(windows)] {
                use std::os::windows::fs::MetadataExt;
                // FILE_ATTRIBUTE_REPARSE_POINT; refuse junctions as well as links.
                if meta.file_attributes() & 0x400 != 0 { return Err(WorkspaceError::UnsafePath); }
            }
            if !meta.is_file() || meta.file_type().is_symlink() { return Err(WorkspaceError::UnsafePath); }
            Ok(true)
        },
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(WorkspaceError::io("inspect_destination", e)),
    }
}
fn checked_path(path: &Path) -> Result<PathBuf, WorkspaceError> {
    if !path.is_absolute() { return Err(WorkspaceError::InvalidPath); }
    // No alternate data streams, device names, UNC shares, or traversal.
    if path.components().any(|c| matches!(c, Component::ParentDir | Component::CurDir)) { return Err(WorkspaceError::InvalidPath); }
    #[cfg(windows)] {
        use std::path::Prefix;
        if !matches!(path.components().next(), Some(Component::Prefix(p)) if matches!(p.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))) {
            return Err(WorkspaceError::InvalidPath);
        }
    }
    let name = path.file_name().ok_or(WorkspaceError::InvalidPath)?;
    let text = name.to_str().ok_or(WorkspaceError::InvalidPath)?;
    if text.is_empty() || text.ends_with(['.', ' ']) || text.chars().any(|c| c.is_control() || matches!(c, ':' | '<' | '>' | '"' | '|' | '?' | '*')) {
        return Err(WorkspaceError::InvalidPath);
    }
    let stem = text.split('.').next().unwrap_or("").to_uppercase();
    if ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
        || (stem.len() == 4 && (stem.starts_with("COM") || stem.starts_with("LPT")) && matches!(stem.as_bytes()[3], b'1'..=b'9')) {
        return Err(WorkspaceError::InvalidPath);
    }
    let parent = path.parent().ok_or(WorkspaceError::InvalidPath)?;
    for directory in parent.ancestors() {
        let meta = fs::symlink_metadata(directory).map_err(|e| WorkspaceError::io("inspect_parent", e))?;
        #[cfg(windows)] {
            use std::os::windows::fs::MetadataExt;
            if meta.file_attributes() & 0x400 != 0 { return Err(WorkspaceError::UnsafePath); }
        }
        if !meta.is_dir() || meta.file_type().is_symlink() { return Err(WorkspaceError::UnsafePath); }
    }
    let canonical = fs::canonicalize(parent).map_err(|e| WorkspaceError::io("resolve_parent", e))?.join(name);
    regular_or_missing(&canonical)?; Ok(canonical)
}
fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name: OsString = path.file_name().expect("checked path").to_os_string();
    name.push(suffix); path.with_file_name(name)
}
fn read_bounded(path: &Path) -> Result<Vec<u8>, WorkspaceError> {
    if !regular_or_missing(path)? { return Err(WorkspaceError::io("open_workspace", io::Error::from(io::ErrorKind::NotFound))); }
    let file = File::open(path).map_err(|e| WorkspaceError::io("open_workspace", e))?;
    if file.metadata().map_err(|e| WorkspaceError::io("inspect_workspace", e))?.len() > MAX_WORKSPACE_BYTES { return Err(WorkspaceError::TooLarge); }
    let mut bytes = Vec::new();
    file.take(MAX_WORKSPACE_BYTES + 1).read_to_end(&mut bytes).map_err(|e| WorkspaceError::io("read_workspace", e))?;
    if bytes.len() as u64 > MAX_WORKSPACE_BYTES { return Err(WorkspaceError::TooLarge); } Ok(bytes)
}

/// Owns ONLY a path successfully opened using create_new. It never removes a
/// destination/backup/unknown file. Failed, encrypted scratch files are cleaned
/// up best-effort; a crash may leave encrypted temporaries, never cleartext.
struct OwnedTemp { path: PathBuf, consumed: bool }
impl Drop for OwnedTemp {
    fn drop(&mut self) { if !self.consumed { let _ = fs::remove_file(&self.path); } }
}
fn prepare_temp(destination: &Path, bytes: &[u8]) -> Result<OwnedTemp, WorkspaceError> {
    let ticks = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    for _ in 0..32 {
        let seq = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = sibling(destination, &format!(".tmp-{}-{ticks:x}-{seq:x}", std::process::id()));
        let mut options = OpenOptions::new(); options.write(true).create_new(true);
        #[cfg(windows)] { use std::os::windows::fs::OpenOptionsExt; options.share_mode(0); }
        let mut file = match options.open(&path) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(WorkspaceError::io("create_encrypted_temp", e)),
        };
        let guard = OwnedTemp { path, consumed: false };
        // Explicit close BEFORE guard cleanup on error (share mode denies delete).
        let result = file.write_all(bytes).and_then(|_| file.sync_all());
        drop(file);
        result.map_err(|e| WorkspaceError::io("sync_encrypted_temp", e))?;
        return Ok(guard);
    }
    Err(WorkspaceError::Busy)
}
fn unchanged(path: &Path, expected: Option<&[u8]>) -> Result<(), WorkspaceError> {
    match expected {
        Some(bytes) if regular_or_missing(path)? && read_bounded(path)? == bytes => Ok(()),
        None if !regular_or_missing(path)? => Ok(()),
        _ => Err(WorkspaceError::ChangedDuringSave),
    }
}
impl Workspace {
    /// Reads only. Missing/corrupt files are errors, never defaulted or repaired.
    pub fn load(path: &Path) -> Result<Self, WorkspaceError> {
        if !cfg!(windows) { return Err(WorkspaceError::Unsupported); }
        let path = checked_path(path)?; decode(&read_bounded(&path)?)
    }
    /// Explicit save; same-directory encrypted scratch + sync + Windows rename.
    /// Existing file MUST decrypt/parse before backup changes. A stale revision,
    /// corrupt backup, unsupported schema or lock conflict refuses the save.
    /// revision changes in memory ONLY after successful final replacement.
    pub fn save(&mut self, path: &Path) -> Result<(), WorkspaceError> {
        if !cfg!(windows) { return Err(WorkspaceError::Unsupported); }
        check_schema(self)?;
        let next_revision = self.revision.checked_add(1).ok_or(WorkspaceError::RevisionOverflow)?;
        let path = checked_path(path)?;
        let backup = sibling(&path, ".backup");
        let lock_path = sibling(&path, ".lock");
        let _lock = crypt::lock_new(&lock_path)?;
        let original = if regular_or_missing(&path)? {
            let bytes = read_bounded(&path)?;
            let existing = decode(&bytes)?;
            if existing.revision != self.revision {
                return Err(WorkspaceError::StaleRevision { expected: self.revision, actual: existing.revision });
            }
            Some(bytes)
        } else {
            if self.revision != 0 { return Err(WorkspaceError::StaleRevision { expected: self.revision, actual: 0 }); }
            None
        };
        let old_backup = if regular_or_missing(&backup)? {
            let bytes = read_bounded(&backup)?; decode(&bytes)?; Some(bytes)
        } else { None };
        let mut candidate = self.clone(); candidate.revision = next_revision;
        let encrypted = encode(&candidate)?;
        let mut temp = prepare_temp(&path, &encrypted)?;
        if let Some(bytes) = original.as_deref() {
            let mut backup_temp = prepare_temp(&backup, bytes)?;
            unchanged(&path, original.as_deref())?;
            unchanged(&backup, old_backup.as_deref())?;
            crypt::atomic_move(&backup_temp.path, &backup, old_backup.is_some())?;
            backup_temp.consumed = true;
        }
        unchanged(&path, original.as_deref())?;
        crypt::atomic_move(&temp.path, &path, original.is_some())?;
        temp.consumed = true;
        self.revision = next_revision; Ok(())
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn ciphertext_length_header_and_schema_rejected() {
        let good = encode(&Workspace::default()).unwrap();
        let mut wrong = good.clone(); wrong[5] ^= 1;
        assert!(matches!(decode(&wrong), Err(WorkspaceError::InvalidEnvelope)));
        let mut bad_schema = Workspace::default(); bad_schema.schema_version = 99;
        assert!(matches!(encode(&bad_schema), Err(WorkspaceError::UnsupportedSchema(99))));
        let plain = Secret(serde_json::to_vec(&bad_schema).unwrap());
        let cipher = crypt::protect(&plain.0).unwrap();
        let mut envelope = MAGIC.to_vec(); envelope.extend_from_slice(&(cipher.len() as u64).to_le_bytes()); envelope.extend(cipher);
        assert!(matches!(decode(&envelope), Err(WorkspaceError::UnsupportedSchema(99))));
    }
    #[test]
    fn invalid_json_is_not_defaulted() {
        let cipher = crypt::protect(b"not-a-workspace-json").unwrap();
        let mut envelope = MAGIC.to_vec(); envelope.extend_from_slice(&(cipher.len() as u64).to_le_bytes()); envelope.extend(cipher);
        assert!(matches!(decode(&envelope), Err(WorkspaceError::InvalidData)));
    }
    #[test]
    fn writer_limit_refuses_excess_and_wipes_on_drop() {
        let mut writer = LimitedBuffer { data: Secret(Vec::new()), exceeded: false };
        writer.data.0.resize(MAX_WORKSPACE_BYTES as usize, b'0');
        assert!(writer.write_all(b"x").is_err()); assert!(writer.exceeded);
    }
}
