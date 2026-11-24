use crate::error::{AiError, Result};
use previdenciar_workspace::local_secret;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

const MAGIC: &[u8; 5] = b"PVAI1";
const MAX_VAULT_BYTES: u64 = 1_048_576;
const MAX_KEYS: usize = 64;
const MAX_KEY_BYTES: usize = 8192;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyStatus {
    pub provider_id: String,
    pub has_key: bool,
}

#[derive(Default, Deserialize, Serialize)]
struct Vault {
    version: u32,
    keys: BTreeMap<String, String>,
}

#[derive(Clone)]
pub struct SecretVault {
    path: PathBuf,
}

impl SecretVault {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn checked_path(&self) -> Result<()> {
        if !self.path.is_absolute() {
            return Err(AiError::Vault(
                "O cofre de chaves precisa de um caminho absoluto.".into(),
            ));
        }
        for component in self.path.components() {
            match component {
                Component::ParentDir | Component::CurDir => {
                    return Err(AiError::Vault(
                        "Caminho do cofre com travessia recusado.".into(),
                    ))
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn load(&self) -> Result<Vault> {
        self.checked_path()?;
        let meta = match fs::symlink_metadata(&self.path) {
            Ok(meta) => meta,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Vault {
                    version: 1,
                    keys: BTreeMap::new(),
                })
            }
            Err(_) => {
                return Err(AiError::Vault(
                    "Não foi possível inspecionar o cofre de chaves.".into(),
                ))
            }
        };
        if meta.file_type().is_symlink() || !meta.is_file() {
            return Err(AiError::Vault(
                "O cofre de chaves não e um arquivo regular.".into(),
            ));
        }
        if meta.len() > MAX_VAULT_BYTES || meta.len() < (MAGIC.len() as u64 + 8) {
            return Err(AiError::Vault(
                "O cofre de chaves tem tamanho inválido.".into(),
            ));
        }
        let bytes = fs::read(&self.path)
            .map_err(|_| AiError::Vault("Não foi possível ler o cofre de chaves.".into()))?;
        if !bytes.starts_with(MAGIC) {
            return Err(AiError::Vault(
                "O cofre de chaves não possui o envelope esperado.".into(),
            ));
        }
        let length = u64::from_le_bytes(
            bytes[MAGIC.len()..MAGIC.len() + 8]
                .try_into()
                .map_err(|_| AiError::Vault("Envelope do cofre inválido.".into()))?,
        ) as usize;
        let ciphertext = &bytes[MAGIC.len() + 8..];
        if length != ciphertext.len() || ciphertext.is_empty() {
            return Err(AiError::Vault("Envelope do cofre inconsistente.".into()));
        }
        let mut plaintext = local_secret::unprotect(ciphertext).map_err(|_| {
            AiError::Vault("DPAPI não conseguiu abrir o cofre para este usuario Windows.".into())
        })?;
        let parsed = serde_json::from_slice::<Vault>(&plaintext);
        local_secret::wipe(&mut plaintext);
        let mut vault = parsed
            .map_err(|_| AiError::Vault("O conteúdo do cofre de chaves esta corrompido.".into()))?;
        if vault.version != 1 || vault.keys.len() > MAX_KEYS {
            return Err(AiError::Vault(
                "Versão ou tamanho do cofre de chaves inválido.".into(),
            ));
        }
        for (provider, key) in &vault.keys {
            if !valid_provider_id(provider) || key.is_empty() || key.len() > MAX_KEY_BYTES {
                return Err(AiError::Vault(
                    "O cofre contem uma entrada invalida.".into(),
                ));
            }
        }
        if vault.version == 0 {
            vault.version = 1;
        }
        Ok(vault)
    }

    fn save(&self, vault: &Vault) -> Result<()> {
        self.checked_path()?;
        if vault.keys.len() > MAX_KEYS {
            return Err(AiError::Vault(
                "O cofre excede o limite de provedores.".into(),
            ));
        }
        let parent = self
            .path
            .parent()
            .ok_or_else(|| AiError::Vault("Diretório do cofre inválido.".into()))?;
        if !parent.is_dir() {
            return Err(AiError::Vault(
                "O diretório local do aplicativo não existe.".into(),
            ));
        }
        let mut plaintext = serde_json::to_vec(vault)
            .map_err(|_| AiError::Vault("Não foi possível serializar o cofre de chaves.".into()))?;
        if plaintext.len() as u64 > MAX_VAULT_BYTES {
            return Err(AiError::Vault(
                "O cofre de chaves excede o limite local.".into(),
            ));
        }
        let protected = local_secret::protect(&plaintext).map_err(|_| {
            AiError::Vault("DPAPI não conseguiu proteger o cofre de chaves.".into())
        })?;
        local_secret::wipe(&mut plaintext);
        if protected.len() as u64 > MAX_VAULT_BYTES {
            return Err(AiError::Vault(
                "O cofre protegido excede o limite local.".into(),
            ));
        }
        let mut envelope = Vec::with_capacity(MAGIC.len() + 8 + protected.len());
        envelope.extend_from_slice(MAGIC);
        envelope.extend_from_slice(&(protected.len() as u64).to_le_bytes());
        envelope.extend_from_slice(&protected);
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|value| value.as_nanos())
            .unwrap_or(0);
        let temp = parent.join(format!(".ai-keys-{}-{nonce}.tmp", std::process::id()));
        let result = (|| -> Result<()> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)
                .map_err(|_| {
                    AiError::Vault("Não foi possível criar o arquivo temporario do cofre.".into())
                })?;
            file.write_all(&envelope)
                .and_then(|_| file.sync_all())
                .map_err(|_| AiError::Vault("Não foi possível gravar o cofre de chaves.".into()))?;
            drop(file);
            previdenciar_workspace::local_secret::atomic_replace(&temp, &self.path).map_err(|_| {
                AiError::Vault(
                    "Não foi possível substituir o cofre de chaves com seguranca.".into(),
                )
            })?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temp);
        }
        result
    }

    pub fn has_key(&self, provider_id: &str) -> Result<bool> {
        if !valid_provider_id(provider_id) {
            return Err(AiError::Invalid(
                "Identificador de provedor inválido.".into(),
            ));
        }
        Ok(self.load()?.keys.contains_key(provider_id))
    }

    pub fn statuses(&self, provider_ids: &[String]) -> Result<Vec<KeyStatus>> {
        let vault = self.load()?;
        Ok(provider_ids
            .iter()
            .map(|provider_id| KeyStatus {
                provider_id: provider_id.clone(),
                has_key: vault.keys.contains_key(provider_id),
            })
            .collect())
    }

    pub fn get_key(&self, provider_id: &str) -> Result<String> {
        if !valid_provider_id(provider_id) {
            return Err(AiError::Invalid(
                "Identificador de provedor inválido.".into(),
            ));
        }
        self.load()?
            .keys
            .get(provider_id)
            .cloned()
            .ok_or(AiError::MissingKey)
    }

    pub fn set_key(&self, provider_id: &str, api_key: &str) -> Result<()> {
        if !valid_provider_id(provider_id) {
            return Err(AiError::Invalid(
                "Identificador de provedor inválido.".into(),
            ));
        }
        let api_key = api_key.trim();
        if api_key.is_empty() {
            return Err(AiError::Invalid("Informe a chave de API.".into()));
        }
        if api_key.len() > MAX_KEY_BYTES || api_key.chars().any(char::is_whitespace) {
            return Err(AiError::Invalid(
                "A chave de API tem formato inválido.".into(),
            ));
        }
        let mut vault = self.load()?;
        vault.version = 1;
        vault
            .keys
            .insert(provider_id.to_owned(), api_key.to_owned());
        self.save(&vault)
    }

    pub fn delete_key(&self, provider_id: &str) -> Result<()> {
        if !valid_provider_id(provider_id) {
            return Err(AiError::Invalid(
                "Identificador de provedor inválido.".into(),
            ));
        }
        let mut vault = match self.load() {
            Ok(vault) => vault,
            Err(AiError::Vault(_)) if !self.path.exists() => return Ok(()),
            Err(error) => return Err(error),
        };
        if vault.keys.remove(provider_id).is_some() {
            self.save(&vault)?;
        }
        Ok(())
    }
}

fn valid_provider_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

#[cfg(windows)]
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn dpapi_vault_roundtrip_never_writes_plaintext_key() {
        let dir = tempfile::tempdir().unwrap();
        let vault = SecretVault::new(dir.path().join("ai-keys.pvai"));
        vault
            .set_key("opencode-go", "sk-synthetic-test-key")
            .unwrap();
        assert!(vault.has_key("opencode-go").unwrap());
        assert_eq!(
            vault.get_key("opencode-go").unwrap(),
            "sk-synthetic-test-key"
        );
        let bytes = fs::read(vault.path()).unwrap();
        assert!(bytes.starts_with(MAGIC));
        assert!(!bytes.windows(8).any(|window| window == b"sk-synth"));
        vault.delete_key("opencode-go").unwrap();
        assert!(!vault.has_key("opencode-go").unwrap());
    }

    #[test]
    fn corrupt_vault_blocks_writes_instead_of_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ai-keys.pvai");
        fs::write(&path, b"not-a-vault").unwrap();
        let vault = SecretVault::new(path.clone());
        assert!(vault.get_key("openai").is_err());
        assert!(vault.set_key("openai", "sk-synthetic").is_err());
        assert_eq!(fs::read(&path).unwrap(), b"not-a-vault");
    }

    #[test]
    fn provider_ids_and_whitespace_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let vault = SecretVault::new(dir.path().join("ai-keys.pvai"));
        assert!(vault.set_key("../openai", "sk-synthetic").is_err());
        assert!(vault.set_key("openai", "sk with spaces").is_err());
        assert!(vault.set_key("openai", "   ").is_err());
    }
}


