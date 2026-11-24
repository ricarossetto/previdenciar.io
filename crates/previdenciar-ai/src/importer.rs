use crate::error::{AiError, Result};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_FILES: usize = 20_000;
const MAX_BYTES: u64 = 1_500_000_000;
const MAX_OCR_PAGES: usize = 20;
const MAX_TEXT_CHARS: usize = 80_000;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub skills: usize,
    pub legislation: usize,
    pub natjus: usize,
    pub jurisprudence: usize,
    pub natjus_pdf_included: bool,
    pub natjus_text: usize,
    pub natjus_text_failures: usize,
    pub destination: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedPdf {
    pub text: String,
    pub used_ocr: bool,
    pub pages: usize,
}

fn validate_pdf(path: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(path)
        .map_err(|_| AiError::Data("PDF selecionado não encontrado.".into()))?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(AiError::Data(
            "O caminho selecionado não é um PDF regular.".into(),
        ));
    }
    if meta.len() > 64 * 1024 * 1024 {
        return Err(AiError::Data("O PDF excede o limite de 64 MiB.".into()));
    }
    Ok(())
}

pub fn extract_pdf_text(path: &Path) -> Result<String> {
    validate_pdf(path)?;
    let text = pdf_extract::extract_text(path).map_err(|_| {
        AiError::Data(
            "Não foi possível extrair texto deste PDF; ele pode ser escaneado ou estar protegido."
                .into(),
        )
    })?;
    let bounded: String = text.chars().take(MAX_TEXT_CHARS).collect();
    if bounded.trim().is_empty() {
        return Err(AiError::Data(
            "O PDF não retornou texto extraível; ele pode ser escaneado."
                .into(),
        ));
    }
    Ok(bounded)
}

pub fn extract_case_pdf(path: &Path) -> Result<ExtractedPdf> {
    validate_pdf(path)?;
    let text = pdf_extract::extract_text(path).unwrap_or_default();
    let bounded: String = text.chars().take(MAX_TEXT_CHARS).collect();
    let meaningful = bounded.chars().filter(|character| !character.is_whitespace()).count();
    if meaningful >= 40 {
        return Ok(ExtractedPdf {
            text: bounded,
            used_ocr: false,
            pages: 0,
        });
    }
    let (text, pages) = ocr_pdf(path, MAX_OCR_PAGES)?;
    Ok(ExtractedPdf {
        text,
        used_ocr: true,
        pages,
    })
}

fn tesseract_binary() -> Result<PathBuf> {
    let candidates = [
        "tesseract",
        "C:\\Program Files\\Tesseract-OCR\\tesseract.exe",
        "C:\\Program Files (x86)\\Tesseract-OCR\\tesseract.exe",
    ];
    for candidate in candidates {
        let usable = Command::new(candidate)
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);
        if usable {
            return Ok(PathBuf::from(candidate));
        }
    }
    Err(AiError::Data(
        "OCR local indisponível: instale o Tesseract OCR e o Poppler para ler PDFs escaneados."
            .into(),
    ))
}

fn tesseract_languages(binary: &Path) -> String {
    let Ok(output) = Command::new(binary).arg("--list-langs").output() else {
        return "por+eng".into();
    };
    let languages = String::from_utf8_lossy(&output.stdout);
    let has_por = languages.lines().any(|line| line.trim() == "por");
    let has_eng = languages.lines().any(|line| line.trim() == "eng");
    match (has_por, has_eng) {
        (true, true) => "por+eng".into(),
        (true, false) => "por".into(),
        (false, true) => "eng".into(),
        (false, false) => "por+eng".into(),
    }
}

fn ocr_pdf(path: &Path, max_pages: usize) -> Result<(String, usize)> {
    if Command::new("pdftoppm").arg("-v").output().is_err() {
        return Err(AiError::Data(
            "OCR local indisponível: instale o Poppler (pdftoppm) junto com o Tesseract.".into(),
        ));
    }
    let tesseract = tesseract_binary()?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or(0);
    let temp = std::env::temp_dir().join(format!("previdenciar-ocr-{}-{stamp}", std::process::id()));
    fs::create_dir_all(&temp)
        .map_err(|_| AiError::Data("Não foi possível preparar a pasta temporária do OCR.".into()))?;
    let prefix = temp.join("page");
    let rendered = Command::new("pdftoppm")
        .args(["-r", "200", "-png", "-f", "1", "-l"])
        .arg(max_pages.to_string())
        .arg(path)
        .arg(&prefix)
        .output();
    match rendered {
        Ok(output) if output.status.success() => {}
        Ok(output) => {
            let _ = fs::remove_dir_all(&temp);
            let message = String::from_utf8_lossy(&output.stderr);
            return Err(AiError::Data(format!(
                "O OCR não conseguiu converter o PDF em imagens: {}",
                message.trim().chars().take(240).collect::<String>()
            )));
        }
        Err(_) => {
            let _ = fs::remove_dir_all(&temp);
            return Err(AiError::Data(
                "O OCR não conseguiu executar o pdftoppm.".into(),
            ));
        }
    }
    let mut images: Vec<(usize, PathBuf)> = fs::read_dir(&temp)
        .map_err(|_| AiError::Data("Não foi possível listar as páginas do OCR.".into()))?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("png") {
                return None;
            }
            let page = path
                .file_stem()
                .and_then(|value| value.to_str())
                .and_then(|value| value.rsplit('-').next())
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(0);
            Some((page, path))
        })
        .collect();
    images.sort_by_key(|(page, _)| *page);
    if images.is_empty() {
        let _ = fs::remove_dir_all(&temp);
        return Err(AiError::Data("O OCR não gerou páginas para leitura.".into()));
    }
    let languages = tesseract_languages(&tesseract);
    let mut text = String::new();
    let mut pages = 0usize;
    for (page_number, image) in images {
        let output = Command::new(&tesseract)
            .arg(&image)
            .arg("stdout")
            .args(["-l", &languages, "--psm", "3"])
            .output();
        if let Ok(output) = output {
            if output.status.success() {
                let page_text = String::from_utf8_lossy(&output.stdout);
                if !page_text.trim().is_empty() {
                    text.push_str(&format!("[Página {page_number}]\n\n"));
                    text.push_str(page_text.trim());
                    text.push_str("\n\n");
                    pages += 1;
                }
            }
        }
    }
    let _ = fs::remove_dir_all(&temp);
    let text: String = text.chars().take(MAX_TEXT_CHARS).collect();
    if text.trim().is_empty() {
        return Err(AiError::Data(
            "O OCR local não encontrou texto legível neste PDF.".into(),
        ));
    }
    Ok((text, pages))
}
pub fn import_audit(
    source: &Path,
    destination: &Path,
    include_natjus_pdf: bool,
) -> Result<ImportSummary> {
    let source = source
        .canonicalize()
        .map_err(|_| AiError::Data("A pasta de auditoria selecionada não existe.".into()))?;
    if !source.is_dir() {
        return Err(AiError::Data(
            "A origem selecionada não e uma pasta.".into(),
        ));
    }
    let destination = absolute_destination(destination)?;
    let comparison = canonical_for_comparison(&destination);
    if comparison == source || comparison.starts_with(&source) {
        return Err(AiError::Data(
            "O destino não pode ficar dentro da pasta de origem.".into(),
        ));
    }
    fs::create_dir_all(&destination)
        .map_err(|_| AiError::Data("Não foi possível criar a pasta local do Copiloto.".into()))?;
    let mut budget = Budget::default();
    let skills_source = prefixed_dir(&source, "HABILIDADES", "habilidades")?;
    let legislation_source = prefixed_dir(&source, "LEGISLACAO", "legislacao")?;
    let natjus_source = prefixed_dir(&source, "NATJUS", "NatJus")?;

    copy_required_file(
        &skills_source.join("index.json"),
        &destination.join("skills").join("index.json"),
        &mut budget,
    )?;
    copy_required_tree(
        &skills_source.join("md"),
        &destination.join("skills").join("md"),
        &mut budget,
    )?;
    copy_required_file(
        &legislation_source.join("index.json"),
        &destination.join("legislation").join("index.json"),
        &mut budget,
    )?;
    copy_required_tree(
        &legislation_source.join("normas"),
        &destination.join("legislation").join("normas"),
        &mut budget,
    )?;
    copy_required_file(
        &natjus_source.join("revisoes.json"),
        &destination.join("natjus").join("revisoes.json"),
        &mut budget,
    )?;
    copy_required_tree(
        &natjus_source.join("revisoes"),
        &destination.join("natjus").join("revisoes"),
        &mut budget,
    )?;
    if include_natjus_pdf {
        copy_required_tree(
            &natjus_source.join("pdf"),
            &destination.join("natjus").join("pdf"),
            &mut budget,
        )?;
    }
    let (natjus_text, natjus_text_failures) = if include_natjus_pdf {
        extract_natjus_text(&destination.join("natjus"))
    } else {
        (0, 0)
    };

    let jurisprudence = normalize_jurisprudence(&source.join("JURISPRUDENCIA-API"))?;
    write_json(
        &destination.join("jurisprudence.json"),
        &Value::Array(jurisprudence.clone()),
    )?;

    let skills = count_index(&destination.join("skills").join("index.json"))?;
    let legislation = count_index(&destination.join("legislation").join("index.json"))?;
    let natjus = count_index(&destination.join("natjus").join("revisoes.json"))?;
    let manifest = json!({
        "importedAt": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|value| value.as_secs())
            .unwrap_or(0),
        "source": source.to_string_lossy(),
        "destination": destination.to_string_lossy(),
        "skills": skills,
        "legislation": legislation,
        "natjus": natjus,
        "jurisprudence": jurisprudence.len(),
        "natjusPdfIncluded": include_natjus_pdf,
        "natjusText": natjus_text,
        "natjusTextFailures": natjus_text_failures,
        "note": "Extratos públicos da base de conhecimento; a base local não certifica vigência, inteiro teor ou aplicação ao caso."
    });
    write_json(&destination.join("manifest.json"), &manifest)?;

    Ok(ImportSummary {
        skills,
        legislation,
        natjus,
        jurisprudence: jurisprudence.len(),
        natjus_pdf_included: include_natjus_pdf,
        natjus_text,
        natjus_text_failures,
        destination: destination.to_string_lossy().to_string(),
    })
}

fn extract_natjus_text(natjus: &Path) -> (usize, usize) {
    let index = fs::read(natjus.join("revisoes.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default();
    let text_dir = natjus.join("pdf-text");
    if fs::create_dir_all(&text_dir).is_err() {
        return (0, index.len());
    }
    let mut extracted = 0usize;
    let mut failures = 0usize;
    for entry in index {
        let Some(id) = entry.get("idParecerTecnico").and_then(Value::as_i64) else {
            failures += 1;
            continue;
        };
        let Some(pdf_file) = entry.get("pdf_file").and_then(Value::as_str) else {
            failures += 1;
            continue;
        };
        let relative = Path::new(pdf_file);
        if relative.is_absolute()
            || relative.components().any(|component| {
                matches!(
                    component,
                    std::path::Component::ParentDir | std::path::Component::CurDir
                )
            })
        {
            failures += 1;
            continue;
        }
        let path = natjus.join(relative);
        if !path.is_file() {
            failures += 1;
            continue;
        }
        match pdf_extract::extract_text(&path) {
            Ok(text) if !text.trim().is_empty() => {
                let bounded: String = text.chars().take(2_000_000).collect();
                match fs::write(text_dir.join(format!("{id}.txt")), bounded) {
                    Ok(()) => extracted += 1,
                    Err(_) => failures += 1,
                }
            }
            _ => failures += 1,
        }
    }
    (extracted, failures)
}

fn prefixed_dir(root: &Path, prefix: &str, label: &str) -> Result<PathBuf> {
    let mut matches = fs::read_dir(root)
        .map_err(|_| AiError::Data(format!("Pasta de origem ausente ({label}).")))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .filter(|entry| entry.file_name().to_string_lossy().to_uppercase().starts_with(prefix))
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    matches.sort();
    matches.into_iter().next().ok_or_else(|| {
        AiError::Data(format!("Pasta de origem ausente ({label})."))
    })
}

fn canonical_for_comparison(path: &Path) -> PathBuf {
    if let Ok(canonical) = path.canonicalize() {
        return canonical;
    }
    let Some(parent) = path.parent() else {
        return path.to_path_buf();
    };
    let Some(name) = path.file_name() else {
        return path.to_path_buf();
    };
    match parent.canonicalize() {
        Ok(canonical) => canonical.join(name),
        Err(_) => path.to_path_buf(),
    }
}

fn absolute_destination(path: &Path) -> Result<PathBuf> {
    if !path.is_absolute() {
        return Err(AiError::Data(
            "O destino da base local precisa ser absoluto.".into(),
        ));
    }
    for component in path.components() {
        if matches!(
            component,
            std::path::Component::ParentDir | std::path::Component::CurDir
        ) {
            return Err(AiError::Data(
                "Caminho de destino com travessia recusado.".into(),
            ));
        }
    }
    Ok(path.to_path_buf())
}

#[derive(Default)]
struct Budget {
    files: usize,
    bytes: u64,
}

fn charge(budget: &mut Budget, bytes: u64) -> Result<()> {
    budget.files = budget.files.saturating_add(1);
    budget.bytes = budget.bytes.saturating_add(bytes);
    if budget.files > MAX_FILES || budget.bytes > MAX_BYTES {
        return Err(AiError::Data(
            "A base de origem excede o limite de importacao local.".into(),
        ));
    }
    Ok(())
}

fn copy_required_file(from: &Path, to: &Path, budget: &mut Budget) -> Result<()> {
    let meta = fs::symlink_metadata(from)
        .map_err(|_| AiError::Data(format!("Arquivo obrigatorio ausente: {}.", from.display())))?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(AiError::Data("Arquivo de origem não e regular.".into()));
    }
    charge(budget, meta.len())?;
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).map_err(|_| {
            AiError::Data("Não foi possível criar a estrutura da base local.".into())
        })?;
    }
    fs::copy(from, to)
        .map_err(|_| AiError::Data("Não foi possível copiar um arquivo da base local.".into()))?;
    Ok(())
}

fn copy_required_tree(from: &Path, to: &Path, budget: &mut Budget) -> Result<()> {
    let meta = fs::symlink_metadata(from)
        .map_err(|_| AiError::Data(format!("Pasta obrigatoria ausente: {}.", from.display())))?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Err(AiError::Data("Pasta de origem não e regular.".into()));
    }
    fs::create_dir_all(to)
        .map_err(|_| AiError::Data("Não foi possível criar uma pasta da base local.".into()))?;
    for entry in fs::read_dir(from)
        .map_err(|_| AiError::Data("Não foi possível ler a pasta de origem.".into()))?
    {
        let entry = entry.map_err(|_| AiError::Data("Entrada de origem invalida.".into()))?;
        let path = entry.path();
        let meta = fs::symlink_metadata(&path)
            .map_err(|_| AiError::Data("Não foi possível inspecionar a origem.".into()))?;
        if meta.file_type().is_symlink() {
            continue;
        }
        let target = to.join(entry.file_name());
        if meta.is_dir() {
            copy_required_tree(&path, &target, budget)?;
        } else if meta.is_file() {
            charge(budget, meta.len())?;
            fs::copy(&path, &target).map_err(|_| {
                AiError::Data("Não foi possível copiar um arquivo da base local.".into())
            })?;
        }
    }
    Ok(())
}

fn count_index(path: &Path) -> Result<usize> {
    let bytes = fs::read(path).map_err(|_| AiError::Data("Índice importado ausente.".into()))?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|_| AiError::Data("Índice importado inválido.".into()))?;
    value
        .as_array()
        .map(Vec::len)
        .ok_or_else(|| AiError::Data("Índice importado não e uma lista.".into()))
}

fn normalize_jurisprudence(source: &Path) -> Result<Vec<Value>> {
    if !source.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = fs::read_dir(source)
        .map_err(|_| AiError::Data("Não foi possível ler a pasta de jurisprudência.".into()))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with("search-") && name.ends_with(".json"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    files.sort();
    let buckets = [
        "results",
        "reranked_results",
        "juris",
        "rg",
        "sumulas",
        "sumulas_vinc",
        "repetitivos",
        "puil",
        "iacs",
    ];
    let mut seen = HashSet::new();
    let mut key_counts = HashMap::<String, usize>::new();
    let mut output = Vec::new();
    for file in files {
        let tribunal = file
            .file_stem()
            .and_then(|value| value.to_str())
            .map(|value| value.trim_start_matches("search-").to_lowercase())
            .unwrap_or_else(|| "tribunal".into());
        let bytes = fs::read(&file)
            .map_err(|_| AiError::Data("Falha ao ler jurisprudência de origem.".into()))?;
        let document: Value = serde_json::from_slice(&bytes)
            .map_err(|_| AiError::Data("Jurisprudencia de origem invalida.".into()))?;
        for bucket in buckets {
            let Some(items) = document.get(bucket).and_then(Value::as_array) else {
                continue;
            };
            for item in items {
                let Some(ementa) = text_field(item, "texto_ementa")
                    .or_else(|| text_field(item, "ementa"))
                    .or_else(|| text_field(item, "texto"))
                else {
                    continue;
                };
                let id = text_or_number(item, "id")
                    .or_else(|| text_or_number(item, "normalizedId"))
                    .unwrap_or_else(|| format!("sem-id-{}", output.len() + 1));
                let base_key = format!("{tribunal}-{id}");
                let fingerprint = format!(
                    "{base_key}|{}",
                    ementa.chars().take(160).collect::<String>()
                );
                if !seen.insert(fingerprint) {
                    continue;
                }
                let count = key_counts.entry(base_key.clone()).or_insert(0);
                *count += 1;
                let key = if *count == 1 {
                    base_key
                } else {
                    format!("{base_key}-{count}")
                };
                let classe = text_field(item, "classe_processual");
                let numero = text_field(item, "numero_processo");
                let titulo = text_field(item, "titulo").unwrap_or_else(|| {
                    let parts = [classe.as_deref(), numero.as_deref()]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>();
                    if parts.is_empty() {
                        format!("{tribunal} {id}")
                    } else {
                        parts.join(" ")
                    }
                });
                let url = text_field(item, "link_pdf")
                    .or_else(|| text_field(item, "link_consulta"))
                    .or_else(|| text_field(item, "link_processo"))
                    .or_else(|| text_field(item, "sourceUrl"));
                output.push(json!({
                    "key": key,
                    "id": id,
                    "tribunal": tribunal,
                    "tipo": text_field(item, "__kind").or_else(|| text_field(item, "type")),
                    "titulo": titulo,
                    "classe": classe,
                    "processo": text_field(item, "numero_processo_cnj").or_else(|| text_field(item, "numero_processo")),
                    "relator": text_field(item, "relator"),
                    "orgao": text_field(item, "orgao_julgador"),
                    "data": text_field(item, "data_julgamento"),
                    "url": url,
                    "ementa": ementa,
                    "bucket": bucket
                }));
            }
        }
    }
    Ok(output)
}

fn text_or_number(value: &Value, field: &str) -> Option<String> {
    text_field(value, field).or_else(|| {
        value.get(field).and_then(|item| {
            item.as_i64()
                .map(|number| number.to_string())
                .or_else(|| item.as_u64().map(|number| number.to_string()))
        })
    })
}

fn text_field(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn write_json(path: &Path, value: &Value) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|_| AiError::Data("Não foi possível serializar a base local.".into()))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|_| AiError::Data("Não foi possível criar a pasta da base local.".into()))?;
    }
    fs::write(path, bytes)
        .map_err(|_| AiError::Data("Não foi possível gravar a base local.".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn local_ocr_can_be_checked_with_an_external_pdf() {
        let Ok(path) = std::env::var("PREVIDENCIAR_OCR_TEST_PDF") else {
            return;
        };
        let (text, pages) = ocr_pdf(Path::new(&path), 1).unwrap();
        assert!(pages >= 1);
        assert!(text.contains("[Página 1]"));
        assert!(!text.trim().is_empty());
    }

    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("audit");
        fs::create_dir_all(source.join("HABILIDADES-BASE/md")).unwrap();
        fs::create_dir_all(source.join("LEGISLACAO-BASE/normas")).unwrap();
        fs::create_dir_all(source.join("NATJUS-BASE/revisoes")).unwrap();
        fs::create_dir_all(source.join("JURISPRUDENCIA-API")).unwrap();
        fs::write(
            source.join("HABILIDADES-BASE/index.json"),
            r#"[{"stableId":"a","title":"A","md_path":"md/a.md"}]"#,
        )
        .unwrap();
        fs::write(source.join("HABILIDADES-BASE/md/a.md"), "# A").unwrap();
        fs::write(
            source.join("LEGISLACAO-BASE/index.json"),
            r#"[{"legislacao_id":1,"titulo":"Lei","md_path":"normas/l.md"}]"#,
        )
        .unwrap();
        fs::write(source.join("LEGISLACAO-BASE/normas/l.md"), "# Lei").unwrap();
        fs::write(
            source.join("NATJUS-BASE/revisoes.json"),
            r#"[{"idParecerTecnico":1,"intervencao":"Tecnologia","md_path":"revisoes/n.md"}]"#,
        )
        .unwrap();
        fs::write(source.join("NATJUS-BASE/revisoes/n.md"), "# NatJus").unwrap();
        fs::write(source.join("JURISPRUDENCIA-API/search-stj.json"), r#"{"results":[{"id":1,"texto_ementa":"Ementa sintetica","numero_processo":"REsp 1"}],"reranked_results":[{"id":1,"texto_ementa":"Ementa sintetica","numero_processo":"REsp 1"}]}"#).unwrap();
        dir
    }

    #[test]
    fn imports_known_directories_and_normalizes_jurisprudence() {
        let dir = fixture();
        let destination = dir.path().join("data");
        let summary = import_audit(&dir.path().join("audit"), &destination, false).unwrap();
        assert_eq!(
            (
                summary.skills,
                summary.legislation,
                summary.natjus,
                summary.jurisprudence
            ),
            (1, 1, 1, 1)
        );
        let jurisprudence: Value =
            serde_json::from_slice(&fs::read(destination.join("jurisprudence.json")).unwrap())
                .unwrap();
        assert_eq!(jurisprudence[0]["key"], "stj-1");
        assert!(destination.join("skills/md/a.md").is_file());
    }

    #[test]
    fn rejects_destination_inside_source() {
        let dir = fixture();
        assert!(import_audit(
            &dir.path().join("audit"),
            &dir.path().join("audit/data"),
            false
        )
        .is_err());
    }
}





