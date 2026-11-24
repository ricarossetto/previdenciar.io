//! Cloud AI boundary: explicit consent, DPAPI-backed keys and read-only local
//! legal-data tools. Network transport lives in `previdenciar-ai`; this module only
//! resolves application-owned paths and bridges Tauri channels.
use serde::Serialize;
use sha2::{Digest, Sha256};
use previdenciar_ai::{
    catalog, import_audit, key_required, list_models, preset_by_id, test_connection, AiEvent,
    ChatRequest, DataStore, ImportSummary, KeyStatus, KnowledgeBlock, ProviderKind, ProviderPreset,
    SecretVault, TestResult, ToolOutcome,
};
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;
use tauri::State;

pub struct AiState {
    root: Result<PathBuf, String>,
    cancels: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl Default for AiState {
    fn default() -> Self {
        Self {
            root: app_root(),
            cancels: Mutex::new(HashMap::new()),
        }
    }
}

impl AiState {
    pub(crate) fn root(&self) -> Result<&PathBuf, String> {
        self.root.as_ref().map_err(Clone::clone)
    }

    pub(crate) fn vault(&self) -> Result<SecretVault, String> {
        Ok(SecretVault::new(self.root()?.join("ai-keys.pvai")))
    }

    fn data(&self) -> Result<DataStore, String> {
        Ok(DataStore::new(self.root()?.join("conhecimento")))
    }

    fn register(&self, request_id: &str) -> Result<Arc<AtomicBool>, String> {
        let flag = Arc::new(AtomicBool::new(false));
        self.cancels
            .lock()
            .map_err(|_| "Registro de geração indisponível.".to_owned())?
            .insert(request_id.to_owned(), Arc::clone(&flag));
        Ok(flag)
    }

    fn finish(&self, request_id: &str) {
        if let Ok(mut cancels) = self.cancels.lock() {
            cancels.remove(request_id);
        }
    }
}

fn app_root() -> Result<PathBuf, String> {
    let base = std::env::var_os("LOCALAPPDATA").ok_or_else(|| {
        "LOCALAPPDATA indisponível; o Copiloto não pode abrir o cofre local.".to_owned()
    })?;
    let root = PathBuf::from(base).join("Previdenciar");
    crate::files::validate_local_path(&root)?;
    Ok(root)
}

pub(crate) fn ensure_root(root: &Path) -> Result<(), String> {
    crate::files::check_ancestors(root, true)?;
    if !root.exists() {
        std::fs::create_dir(root)
            .map_err(|_| "Não foi possível criar o diretório local do aplicativo.".to_owned())?;
    }
    crate::files::check_ancestors(root, false)
}

fn key_for(vault: &SecretVault, provider_id: &str) -> Result<String, String> {
    match vault.get_key(provider_id) {
        Ok(key) => Ok(key),
        Err(previdenciar_ai::AiError::MissingKey) if !key_required(provider_id) => Ok(String::new()),
        Err(error) => Err(error.to_string()),
    }
}

#[tauri::command]
pub fn ai_provider_catalog() -> Vec<ProviderPreset> {
    catalog()
}

#[tauri::command]
pub async fn ai_key_status(state: State<'_, AiState>) -> Result<Vec<KeyStatus>, String> {
    let vault = state.vault()?;
    let ids = catalog()
        .into_iter()
        .map(|item| item.id)
        .collect::<Vec<_>>();
    tauri::async_runtime::spawn_blocking(move || {
        vault.statuses(&ids).map_err(|error| error.to_string())
    })
    .await
    .map_err(|_| "Leitura do cofre interrompida.".to_owned())?
}

#[tauri::command]
pub async fn ai_save_key(
    provider_id: String,
    api_key: String,
    state: State<'_, AiState>,
) -> Result<(), String> {
    let root = state.root()?.to_path_buf();
    let vault = SecretVault::new(root.join("ai-keys.pvai"));
    tauri::async_runtime::spawn_blocking(move || {
        ensure_root(&root)?;
        vault
            .set_key(&provider_id, &api_key)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|_| "Gravação do cofre interrompida.".to_owned())?
}

#[tauri::command]
pub async fn ai_delete_key(provider_id: String, state: State<'_, AiState>) -> Result<(), String> {
    let vault = state.vault()?;
    tauri::async_runtime::spawn_blocking(move || {
        vault
            .delete_key(&provider_id)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|_| "Remoção da chave interrompida.".to_owned())?
}

#[tauri::command]
pub async fn ai_test(
    provider_id: String,
    kind: ProviderKind,
    base_url: String,
    model: String,
    state: State<'_, AiState>,
) -> Result<TestResult, String> {
    if preset_by_id(&provider_id).is_none() {
        return Err("Provedor desconhecido.".into());
    }
    let vault = state.vault()?;
    let key = key_for(&vault, &provider_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        test_connection(kind, &base_url, &model, &key).map_err(|error| error.to_string())
    })
    .await
    .map_err(|_| "Teste do provedor interrompido.".to_owned())?
}

#[tauri::command]
pub async fn ai_models(
    provider_id: String,
    kind: ProviderKind,
    base_url: String,
    state: State<'_, AiState>,
) -> Result<Vec<String>, String> {
    if preset_by_id(&provider_id).is_none() {
        return Err("Provedor desconhecido.".into());
    }
    let vault = state.vault()?;
    let key = key_for(&vault, &provider_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        list_models(kind, &base_url, &key).map_err(|error| error.to_string())
    })
    .await
    .map_err(|_| "Consulta de modelos interrompida.".to_owned())?
}

#[tauri::command]
pub async fn ai_data_status(state: State<'_, AiState>) -> Result<previdenciar_ai::DataStatus, String> {
    let data = state.data()?;
    tauri::async_runtime::spawn_blocking(move || Ok(data.status()))
        .await
        .map_err(|_| "Leitura da base local interrompida.".to_owned())?
}

#[tauri::command]
pub async fn ai_import_data(
    include_natjus_pdf: bool,
    state: State<'_, AiState>,
) -> Result<Option<ImportSummary>, String> {
    let destination = state.root()?.join("conhecimento");
    let source = tauri::async_runtime::spawn_blocking(|| {
        rfd::FileDialog::new()
            .set_title("Selecionar a pasta base-conhecimento")
            .pick_folder()
    })
    .await
    .map_err(|_| "Seleção de pasta interrompida.".to_owned())?;
    let Some(source) = source else {
        return Ok(None);
    };
    tauri::async_runtime::spawn_blocking(move || {
        import_audit(&source, &destination, include_natjus_pdf)
            .map(Some)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|_| "Importação da base local interrompida.".to_owned())?
}

#[tauri::command]
pub async fn ai_knowledge_search(
    block: KnowledgeBlock,
    query: String,
    limit: Option<u32>,
    state: State<'_, AiState>,
) -> Result<ToolOutcome, String> {
    let data = state.data()?;
    let limit = limit.unwrap_or(8).clamp(1, 8) as usize;
    tauri::async_runtime::spawn_blocking(move || {
        data.search_block(block, &query, limit)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|_| "Busca na base local interrompida.".to_owned())?
}

#[tauri::command]
pub async fn ai_knowledge_read(
    block: KnowledgeBlock,
    id: String,
    state: State<'_, AiState>,
) -> Result<ToolOutcome, String> {
    let data = state.data()?;
    tauri::async_runtime::spawn_blocking(move || {
        data.read_block(block, &id)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|_| "Leitura da base local interrompida.".to_owned())?
}


#[tauri::command]
pub async fn save_export_file(
    data: Vec<u8>,
    extension: String,
    suggested_name: String,
) -> Result<Option<String>, String> {
    let extension = match extension.as_str() {
        "docx" | "txt" | "md" => extension,
        _ => return Err("Formato de exportação não suportado.".into()),
    };
    if data.is_empty() || data.len() as u64 > previdenciar_workspace::MAX_WORKSPACE_BYTES {
        return Err("Arquivo vazio ou acima do limite de 64 MiB.".into());
    }
    let safe_name: String = suggested_name
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || matches!(character, ' ' | '-' | '_' | '(' | ')'))
        .take(80)
        .collect();
    let name = if safe_name.trim().is_empty() {
        format!("minuta.{}", extension)
    } else {
        format!("{}.{}", safe_name.trim(), extension)
    };
    tauri::async_runtime::spawn_blocking(move || {
        let selected = rfd::FileDialog::new()
            .set_title("Salvar arquivo em novo destino")
            .add_filter(extension.to_uppercase(), &[extension.as_str()])
            .set_file_name(&name)
            .save_file();
        selected
            .map(|path| crate::files::write_new(&path, &extension, &data))
            .transpose()
    })
    .await
    .map_err(|_| "Exportação interrompida.".to_owned())?
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CasePdfText {
    pub file_name: String,
    pub text: String,
    pub used_ocr: bool,
    pub pages: usize,
    pub hash_sha256: String,
}

fn file_sha256(path: &Path) -> Result<String, String> {
    let mut file = File::open(path)
        .map_err(|_| "Não foi possível abrir o PDF para calcular a integridade.".to_owned())?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher)
        .map_err(|_| "Não foi possível calcular a integridade do PDF.".to_owned())?;
    Ok(format!("{:x}", hasher.finalize()))
}

#[tauri::command]
pub async fn pick_case_pdf_text() -> Result<Option<CasePdfText>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let selected = rfd::FileDialog::new()
            .set_title("Anexar PDF do caso com texto ou OCR local")
            .add_filter("PDF", &["pdf"])
            .pick_file();
        let Some(path) = selected else { return Ok(None) };
        let extracted = previdenciar_ai::extract_case_pdf(&path).map_err(|error| error.to_string())?;
        let hash_sha256 = file_sha256(&path)?;
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("documento.pdf")
            .to_owned();
        Ok(Some(CasePdfText {
            file_name,
            text: extracted.text,
            used_ocr: extracted.used_ocr,
            pages: extracted.pages,
            hash_sha256,
        }))
    })
    .await
    .map_err(|_| "Leitura do PDF interrompida.".to_owned())?
}
#[tauri::command]
pub async fn ai_chat(
    request: ChatRequest,
    on_event: Channel<AiEvent>,
    state: State<'_, AiState>,
) -> Result<(), String> {
    if request.request_id.is_empty() || request.request_id.len() > 80 {
        return Err("Identificador de solicitação inválido.".into());
    }
    let request_id = request.request_id.clone();
    let cancel = state.register(&request_id)?;
    let vault = match state.vault() {
        Ok(vault) => vault,
        Err(error) => {
            state.finish(&request_id);
            return Err(error);
        }
    };
    let data = match state.data() {
        Ok(data) => data,
        Err(error) => {
            state.finish(&request_id);
            return Err(error);
        }
    };
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        previdenciar_ai::run(request, &vault, &data, &cancel, |event| {
            let _ = on_event.send(event);
        })
        .map_err(|error| error.to_string())
    })
    .await;
    state.finish(&request_id);
    outcome.map_err(|_| "Geracao interrompida; nenhum resultado foi confirmado.".to_owned())?
}

#[tauri::command]
pub fn ai_cancel(request_id: String, state: State<'_, AiState>) -> Result<(), String> {
    let cancels = state
        .cancels
        .lock()
        .map_err(|_| "Registro de geração indisponível.".to_owned())?;
    if let Some(flag) = cancels.get(&request_id) {
        flag.store(true, Ordering::Relaxed);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_root_has_no_traversal_and_is_absolute_on_windows() {
        if let Ok(root) = app_root() {
            assert!(root.is_absolute());
            crate::files::validate_local_path(&root).unwrap();
            assert!(root.ends_with("Previdenciar"));
        }
    }

    #[test]
    fn key_policy_is_explicit_for_custom_and_hosted_presets() {
        assert!(key_required("opencode-go"));
        assert!(!key_required("custom"));
        assert!(!key_required("ollama"));
    }
}


