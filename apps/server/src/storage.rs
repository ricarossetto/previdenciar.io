use aws_sdk_s3::presigning::PresigningConfig;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

#[derive(Clone)]
pub struct S3Store {
    client: aws_sdk_s3::Client,
    bucket: String,
    region: String,
}

impl S3Store {
    pub async fn new(bucket: String, region: String) -> Option<Self> {
        if bucket.trim().is_empty() {
            return None;
        }
        let shared = aws_config::load_from_env().await;
        let client = aws_sdk_s3::Client::new(&shared);
        Some(Self { client, bucket, region })
    }

    pub async fn put_bytes(&self, key: &str, bytes: Vec<u8>, content_type: &str) -> Result<(), String> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .body(bytes.into())
            .send()
            .await
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub async fn get_bytes(&self, key: &str) -> Result<Vec<u8>, String> {
        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|error| error.to_string())?;
        let bytes = response
            .body
            .collect()
            .await
            .map_err(|error| error.to_string())?
            .into_bytes()
            .to_vec();
        if bytes.len() > 64 * 1024 * 1024 {
            return Err("Arquivo PDF excede 64 MiB.".into());
        }
        Ok(bytes)
    }

    pub async fn delete(&self, key: &str) -> Result<(), String> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub async fn presigned_get(&self, key: &str, seconds: u64) -> Result<String, String> {
        let config = PresigningConfig::expires_in(Duration::from_secs(seconds.clamp(60, 3600)))
            .map_err(|error| error.to_string())?;
        let request = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .presigned(config)
            .await
            .map_err(|error| error.to_string())?;
        Ok(request.uri().to_string())
    }

    pub async fn download_knowledge(&self, key: &str, destination: &Path) -> Result<(), String> {
        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|error| error.to_string())?;
        let body = response
            .body
            .collect()
            .await
            .map_err(|error| error.to_string())?
            .into_bytes();
        if body.len() > 512 * 1024 * 1024 {
            return Err("Pacote de conhecimento excede o limite de 512 MiB.".into());
        }
        unpack_tar_gz_safe(body.as_ref(), destination)
    }

    pub fn region(&self) -> &str {
        &self.region
    }
}

#[cfg(test)]
impl S3Store {
    /// Cliente local sem rede para testar a ordem de autorização das rotas.
    /// Não é usado em produção; nenhuma chamada de presign/S3 é feita aqui.
    pub(crate) fn for_tests() -> Self {
        use aws_sdk_s3::config::Region;
        let config = aws_sdk_s3::Config::builder()
            .behavior_version(aws_config::BehaviorVersion::latest())
            .region(Region::new("sa-east-1"))
            .build();
        Self {
            client: aws_sdk_s3::Client::from_conf(config),
            bucket: "bucket-teste".to_owned(),
            region: "sa-east-1".to_owned(),
        }
    }
}

fn entry_path_is_safe(raw: &Path) -> bool {
    let mut depth: i32 = 0;
    for component in raw.components() {
        match component {
            Component::Normal(part) => {
                if part.to_string_lossy().contains('\u{0}') {
                    return false;
                }
                depth += 1;
            }
            Component::CurDir => {}
            Component::ParentDir => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            Component::RootDir | Component::Prefix(_) => return false,
        }
    }
    true
}

fn safe_target_path(destination: &Path, raw: &Path) -> Option<PathBuf> {
    if !entry_path_is_safe(raw) {
        return None;
    }
    let relative = raw
        .components()
        .filter(|component| !matches!(component, Component::CurDir))
        .collect::<PathBuf>();
    if relative.as_os_str().is_empty() {
        return None;
    }
    let target = destination.join(&relative);
    let root = destination.canonicalize().ok()?;
    let mut cursor = root.clone();
    for component in relative.components() {
        if !matches!(component, Component::Normal(_)) {
            return None;
        }
        cursor.push(component.as_os_str());
        if cursor.is_symlink() {
            return None;
        }
        if cursor.exists() {
            if let Ok(resolved) = cursor.canonicalize() {
                if !resolved.starts_with(&root) {
                    return None;
                }
            }
        }
    }
    Some(target)
}

pub fn unpack_tar_gz_safe(input: &[u8], destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    let decoder = flate2::read::GzDecoder::new(input);
    let mut archive = tar::Archive::new(decoder);
    archive.set_preserve_permissions(false);
    archive.set_preserve_mtime(false);
    for entry in archive
        .entries()
        .map_err(|error| format!("Nao foi possivel ler as entradas do pacote: {error}"))?
    {
        let mut entry = entry.map_err(|error| format!("Entrada invalida no pacote: {error}"))?;
        let entry_type = entry.header().entry_type();
        if entry_type.is_symlink() || entry_type.is_hard_link() {
            return Err("Entrada com link simbolico ou permanente foi recusada.".into());
        }
        if !entry_type.is_file() && !entry_type.is_dir() {
            return Err("Entrada com tipo de arquivo nao suportado foi recusada.".into());
        }
        let raw_path = entry.path().map_err(|error| error.to_string())?.into_owned();
        let Some(target) = safe_target_path(destination, &raw_path) else {
            return Err("Entrada com caminho fora do destino foi recusada.".into());
        };
        if entry_type.is_dir() {
            fs::create_dir_all(&target).map_err(|error| error.to_string())?;
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        entry
            .unpack_in(destination)
            .map_err(|error| format!("Falha ao extrair entrada do pacote: {error}"))?;
        if target.is_symlink() {
            return Err("Entrada gerou link simbolico no destino e foi recusada.".into());
        }
    }
    Ok(())
}

fn raw_tar_entry(path: &str, entry_type: u8, size: u64) -> Vec<u8> {
    let mut header = [0u8; 512];
    let name_bytes = path.as_bytes();
    header[..name_bytes.len()].copy_from_slice(name_bytes);
    let octal = format!("{:011o}\0", size);
    header[124..124 + 12].copy_from_slice(octal.as_bytes());
    header[156] = entry_type;
    header[257..257 + 5].copy_from_slice(b"ustar");
    header[263..263 + 2].copy_from_slice(b"00");
    for byte in header[148..156].iter_mut() {
        *byte = b' ';
    }
    let mut checksum: u32 = 0;
    for byte in header.iter() {
        checksum += u32::from(*byte);
    }
    let checksum_octal = format!("{:06o}\0 ", checksum);
    header[148..148 + 8].copy_from_slice(checksum_octal.as_bytes());
    header.to_vec()
}

fn write_tar_with_entries(entries: &[(&str, u8, &[u8])]) -> Vec<u8> {
    let mut raw = Vec::new();
    for (path, entry_type, contents) in entries {
        let mut header = raw_tar_entry(path, *entry_type, contents.len() as u64);
        raw.append(&mut header);
        raw.extend_from_slice(contents);
        let padding = (512 - (contents.len() % 512)) % 512;
        raw.extend(std::iter::repeat(0u8).take(padding));
    }
    raw.extend(std::iter::repeat(0u8).take(1024));
    let mut gz = Vec::new();
    {
        let mut encoder = flate2::write::GzEncoder::new(&mut gz, flate2::Compression::default());
        std::io::Write::write_all(&mut encoder, &raw).expect("tar comprimido");
        encoder.finish().expect("gzip finalizado");
    }
    gz
}

fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("diretorio temporario criado")
}

fn assert_rejects(entries: &[(&str, u8, &[u8])]) {
    let destination = temp_dir();
    let package = write_tar_with_entries(entries);
    let result = unpack_tar_gz_safe(&package, destination.path());
    assert!(result.is_err(), "entrada maliciosa deveria ser recusada");
    assert!(
        !destination.path().join("passwd").exists(),
        "arquivo fora do destino foi criado"
    );
}

const TYPE_FILE: u8 = b'0';
const TYPE_SYMLINK: u8 = b'2';
const TYPE_HARDLINK: u8 = b'1';
const TYPE_DIR: u8 = b'5';

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recusa_caminho_com_parent_dir() {
        assert_rejects(&[("../../etc/passwd", TYPE_FILE, b"malicioso")]);
    }

    #[test]
    fn recusa_caminho_absoluto() {
        assert_rejects(&[("/etc/passwd", TYPE_FILE, b"malicioso")]);
    }

    #[test]
    fn recusa_link_simbolico() {
        assert_rejects(&[("alvo", TYPE_SYMLINK, b"")]);
    }

    #[test]
    fn recusa_link_permanente() {
        assert_rejects(&[("alvo", TYPE_HARDLINK, b"")]);
    }

    #[test]
    fn extrai_arquivo_regular_no_destino() {
        let destination = temp_dir();
        let package = write_tar_with_entries(&[
            ("base/", TYPE_DIR, b""),
            ("base/conhecimento.txt", TYPE_FILE, b"conteudo"),
        ]);
        unpack_tar_gz_safe(&package, destination.path()).expect("extracao valida");
        let extracted = destination.path().join("base/conhecimento.txt");
        assert_eq!(fs::read(&extracted).expect("arquivo lido"), b"conteudo");
    }

    #[test]
    fn recusa_entrada_com_link_apos_extrair() {
        let destination = temp_dir();
        let package = write_tar_with_entries(&[
            ("arquivo.txt", TYPE_FILE, b"conteudo"),
            ("../../fora.txt", TYPE_FILE, b"malicioso"),
        ]);
        let result = unpack_tar_gz_safe(&package, destination.path());
        assert!(result.is_err());
        assert!(!destination.path().join("fora.txt").exists());
    }
}


