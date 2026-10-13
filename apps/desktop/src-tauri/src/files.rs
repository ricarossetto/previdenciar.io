//! Narrow user-selected file I/O. No shell, URL opener, network, or overwrites.
use crate::backend::{validate_calculation, validate_workspace};
use previus_cnis::ImportResult;
use previus_workspace::{Calculation, Workspace, MAX_WORKSPACE_BYTES};
use serde_json::{json, Value};
use std::{fs::{self, OpenOptions}, io::{Read, Write}, path::{Component, Path}};

fn safe_component(name: &std::ffi::OsStr) -> bool {
    let Some(name) = name.to_str() else { return false; };
    if name.is_empty() || name.ends_with(['.', ' '])
        || name.chars().any(|c| c.is_control() || matches!(c, ':' | '<' | '>' | '"' | '|' | '?' | '*')) { return false; }
    let stem = name.split('.').next().unwrap_or("").to_uppercase();
    if ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"].contains(&stem.as_str()) { return false; }
    !((stem.starts_with("COM") || stem.starts_with("LPT")) && stem.chars().count() == 4
        && matches!(stem.chars().last(), Some('1'..='9' | '¹' | '²' | '³')))
}
pub fn validate_local_path(path: &Path) -> Result<(), String> {
    if !path.is_absolute() { return Err("Selecione um caminho absoluto local.".into()); }
    #[cfg(windows)] {
        use std::path::Prefix;
        if !matches!(path.components().next(), Some(Component::Prefix(p)) if matches!(p.kind(), Prefix::Disk(_))) {
            return Err("Caminhos de rede, dispositivo e namespaces especiais não são permitidos.".into());
        }
    }
    for component in path.components() {
        match component {
            Component::ParentDir | Component::CurDir => return Err("Caminho com travessia de diretórios recusado.".into()),
            Component::Normal(name) if !safe_component(name) => return Err("Caminho contém nome reservado ou componente inválido.".into()),
            _ => {}
        }
    }
    // Components normalize internal '/./'; do not silently accept it.
    if path.to_string_lossy().replace('\\', "/").split('/').any(|p| p == "." || p == "..") {
        return Err("Caminho com travessia de diretórios recusado.".into());
    }
    Ok(())
}
fn check_metadata(meta: &fs::Metadata, directory: bool) -> Result<(), String> {
    #[cfg(windows)] {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 { return Err("Redirecionamento de arquivo ou diretório recusado.".into()); }
    }
    if meta.file_type().is_symlink() || (directory && !meta.is_dir()) || (!directory && !meta.is_file()) {
        return Err("Destino não regular ou redirecionado recusado.".into());
    }
    Ok(())
}
pub fn check_ancestors(parent: &Path, allow_missing: bool) -> Result<(), String> {
    validate_local_path(parent)?;
    for directory in parent.ancestors() {
        match fs::symlink_metadata(directory) {
            Ok(meta) => check_metadata(&meta, true)?,
            Err(error) if allow_missing && error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("Diretório indisponível; nenhuma operação de arquivo realizada.".into()),
        }
    }
    Ok(())
}
fn require_extension(path: &Path, extension: &str) -> Result<(), String> {
    if !path.extension().and_then(|s| s.to_str()).is_some_and(|s| s.eq_ignore_ascii_case(extension)) {
        return Err(format!("A extensão do arquivo deve ser .{extension}."));
    }
    Ok(())
}
fn checked_file(path: &Path, extension: &str) -> Result<(), String> {
    validate_local_path(path)?;
    require_extension(path, extension)?;
    check_ancestors(path.parent().ok_or("Diretório inválido.")?, false)?;
    Ok(())
}
pub fn import_path(path: &Path) -> Result<ImportResult, String> {
    checked_file(path, "pdf")?;
    let meta = fs::symlink_metadata(path).map_err(|_| "Não foi possível inspecionar o PDF selecionado.".to_owned())?;
    check_metadata(&meta, false)?;
    if meta.len() > MAX_WORKSPACE_BYTES { return Err("PDF excede o limite de 64 MiB.".into()); }
    let result = previus_cnis::import_pdf(path).map_err(|e| e.to_string())?;
    let check = Calculation { cnis: Some(result.clone()), ..Default::default() };
    validate_calculation(&check)?;
    Ok(result)
}

/// create_new is mandatory even if the native dialog offered overwrite confirmation.
/// This also protects an input PDF, store, symlink, or any pre-existing file.
pub fn write_new(path: &Path, extension: &str, bytes: &[u8]) -> Result<String, String> {
    checked_file(path, extension)?;
    if bytes.len() as u64 > MAX_WORKSPACE_BYTES { return Err("Exportação excede o limite de 64 MiB.".into()); }
    let mut options = OpenOptions::new(); options.write(true).create_new(true);
    #[cfg(windows)] { use std::os::windows::fs::OpenOptionsExt; options.share_mode(0); }
    let mut file = options.open(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            "O arquivo já existe. Escolha um nome novo; sobrescrita não é permitida.".to_owned()
        } else { "Não foi possível criar o novo arquivo; caminho omitido.".to_owned() }
    })?;
    let result = file.write_all(bytes).and_then(|_| file.sync_all());
    drop(file);
    if result.is_err() {
        // Never unlink a user-selected path after closing the handle: a competing
        // process could have replaced it. Preserve any incomplete new file for review.
        return Err("Falha ao concluir a exportação; o novo arquivo pode estar incompleto. Revise o destino antes de utilizá-lo.".into());
    }
    path.to_str().map(str::to_owned).ok_or_else(|| "Arquivo gravado; caminho não representável em UTF-8.".into())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReportFormat { Html, Csv, Json, Pdf(previus_reports::ReportKind) }
impl ReportFormat {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "html" => Ok(Self::Html), "csv" => Ok(Self::Csv), "json" => Ok(Self::Json),
            "pdf" | "pdf_detailed" => Ok(Self::Pdf(previus_reports::ReportKind::Detailed)),
            "pdf_summary" => Ok(Self::Pdf(previus_reports::ReportKind::Summary)),
            "pdf_periods" => Ok(Self::Pdf(previus_reports::ReportKind::Periods)),
            "pdf_salaries" => Ok(Self::Pdf(previus_reports::ReportKind::Salaries)),
            "pdf_memory" => Ok(Self::Pdf(previus_reports::ReportKind::Memory)),
            "pdf_transitions" => Ok(Self::Pdf(previus_reports::ReportKind::Transitions)),
            "pdf_indicators" => Ok(Self::Pdf(previus_reports::ReportKind::Indicators)),
            _ => Err("Formato não suportado. Use um relatório PDF conhecido, HTML, CSV ou JSON.".into()),
        }
    }
    pub fn extension(self) -> &'static str { match self { Self::Html => "html", Self::Csv => "csv", Self::Json => "json", Self::Pdf(_) => "pdf" } }
    pub fn label(self) -> &'static str { match self { Self::Html => "Relatório HTML", Self::Csv => "Remunerações CSV", Self::Json => "Resumo técnico JSON", Self::Pdf(_) => "Relatório detalhado PDF" } }
}

/// Deliberately only counts and fixed policy labels: no names, paths, identity,
/// source hashes, free text, employer strings or arbitrary result JSON.
pub fn summary_json(calculation: &Calculation) -> Value {
    let counts = calculation.cnis.as_ref().map(|cnis| json!({
        "salary_entries": cnis.salary_entries.len(), "salary_months": cnis.salary_months.len(),
        "links": cnis.links.len(), "prpps_links": cnis.links.iter().filter(|v| v.prpps).count(),
        "missing_end_dates": cnis.links.iter().filter(|v| v.end_date.is_none()).count(),
        "benefits": cnis.benefits.len(), "warnings": cnis.warnings.len(),
        "pages": cnis.metadata.page_count,
    }));
    json!({ "schema": "previus-next-summary/v1", "legal_validation_performed": false,
        "currency_policy": "currency_units_unverified", "cnis_counts": counts,
        "manual_link_count": calculation.manual_links.len(), "manual_salary_count": calculation.manual_salaries.len(),
        "warnings": ["Resumo técnico de importação, não concessão de benefício nem validação jurídica.",
            "Indicadores, moedas históricas e regimes exigem revisão independente."] })
}

fn constrained(value: &str, numeric: bool) -> String {
    if value.chars().all(|c| c.is_ascii_digit() || matches!(c, '-' | '/') || (numeric && matches!(c, '.' | ',' | '+'))) {
        value.to_owned()
    } else { String::new() }
}
fn private_case(calculation: &Calculation) -> Calculation {
    let mut case = calculation.clone();
    case.name.clear(); case.notes.clear(); case.scenario.clear(); case.result = None;
    case.dib = constrained(&case.dib, false);
    case.identity = Default::default(); case.sex = None; case.disability = None;
    case.options = Default::default();
    for link in &mut case.manual_links {
        link.employer.clear(); link.indicators.clear(); link.regime.clear();
        link.start_date = constrained(&link.start_date, false); link.end_date = constrained(&link.end_date, false);
    }
    for salary in &mut case.manual_salaries {
        salary.indicators.clear(); salary.amount = constrained(&salary.amount, true);
        salary.competence = constrained(&salary.competence, false);
    }
    if let Some(cnis) = &mut case.cnis {
        cnis.identity = Default::default(); cnis.metadata.extractor.clear(); cnis.metadata.source_sha256 = None;
        for link in &mut cnis.links {
            link.raw_employer = None; link.affiliation = None; link.indicators.clear();
            link.start_date = link.start_date.as_deref().map(|v| constrained(v, false));
            link.end_date = link.end_date.as_deref().map(|v| constrained(v, false));
            link.reported_last_salary_month = link.reported_last_salary_month.as_deref().map(|v| constrained(v, false));
            if let Some(end) = &mut link.inferred_last_salary_end {
                end.competence = constrained(&end.competence, false); end.date_candidate = constrained(&end.date_candidate, false);
                end.provenance.clear();
            }
        }
        for salary in &mut cnis.salary_entries { salary.indicators.clear(); salary.competence = constrained(&salary.competence, false); }
        for finding in cnis.findings.iter_mut().chain(cnis.warnings.iter_mut()) { finding.code = "source_finding".into(); finding.message.clear(); }
    }
    case
}

pub fn render_report(calculation: &Calculation, workspace: &Workspace, format: ReportFormat, include_personal: bool) -> Result<Vec<u8>, String> {
    validate_workspace(workspace)?; validate_calculation(calculation)?;
    if let ReportFormat::Pdf(kind) = format {
        // PDF always describes the current reviewed input, not arbitrary stored JSON.
        // Missing data/unsupported policies produce a conferência report with no fake RMI.
        let outcome = match crate::backend::validate_assistant_for_calculation(calculation) {
            Ok(()) => match previus_rgps::calculate_case(calculation) {
                Ok(value) => value,
                Err(_) => serde_json::json!({"status":"blocked","data":null,"legal_validation_performed":false,
                    "warnings":["Não foi possível recalcular a hipótese; nenhum resultado armazenado foi aceito."],
                    "blockers":[{"code":"report_input_unresolved","message":"Dados ou política ainda não permitem executar o motor."}]}),
            },
            Err(message) => serde_json::json!({"status":"blocked","data":null,"legal_validation_performed":false,
                "warnings":["Este PDF registra conferência de dados, não benefício calculado."],
                "blockers":[{"code":"assistant_review_pending","message":message}]}),
        };
        let bytes = previus_reports::render_pdf(calculation, workspace,
            previus_reports::ReportOptions { kind, include_personal }, Some(&outcome))?;
        if bytes.len() as u64 > MAX_WORKSPACE_BYTES { return Err("PDF excede o limite de 64 MiB.".into()); }
        return Ok(bytes);
    }
    let text = match format {
        ReportFormat::Html => {
            let redacted;
            let case = if include_personal { calculation } else { redacted = private_case(calculation); &redacted };
            let mut html = previus_workspace::report_html(case, workspace, include_personal);
            if include_personal {
                // Recompute from reviewed inputs; never trust arbitrary stored result JSON.
                // This explicit identified report is not the private counts-only summary.
                let extra = recomputed_rgps_report(calculation);
                html = html.replace("</body>", &format!("{extra}</body>"));
            }
            html
        },
        ReportFormat::Csv => {
            if include_personal { previus_workspace::export_salary_csv(calculation) }
            else {
                // Source indicator/manual strings can contain arbitrary user text.
                // Private CSV keeps only constrained numeric/date cells and fixed labels.
                let redacted = private_case(calculation);
                previus_workspace::export_salary_csv(&redacted)
            }
        }
        ReportFormat::Json => serde_json::to_string_pretty(&summary_json(calculation)).map_err(|_| "Falha ao gerar resumo.".to_owned())?,
        ReportFormat::Pdf(_) => unreachable!("PDF is generated through the binary renderer before text formats"),
    };
    if text.len() as u64 > MAX_WORKSPACE_BYTES { return Err("Relatório excede o limite de 64 MiB.".into()); }
    Ok(text.into_bytes())
}

pub fn encrypted_snapshot(source: &Path) -> Result<Vec<u8>, String> {
    checked_file(source, "pvr")?;
    let meta = fs::symlink_metadata(source).map_err(|_| "Não há workspace cifrado disponível para backup.".to_owned())?;
    check_metadata(&meta, false)?;
    let mut options = OpenOptions::new(); options.read(true);
    // While the handle is alive, Windows refuses competing writes/deletes.
    #[cfg(windows)] { use std::os::windows::fs::OpenOptionsExt; options.share_mode(1); }
    let file = options.open(source).map_err(|_| "Workspace indisponível para cópia consistente.".to_owned())?;
    if file.metadata().map_err(|_| "Não foi possível conferir o snapshot.")?.len() > MAX_WORKSPACE_BYTES {
        return Err("Snapshot excede o limite.".into());
    }
    let mut bytes = Vec::new(); (&file).take(MAX_WORKSPACE_BYTES + 1).read_to_end(&mut bytes)
        .map_err(|_| "Falha ao ler o snapshot cifrado.".to_owned())?;
    if bytes.len() as u64 > MAX_WORKSPACE_BYTES || !bytes.starts_with(b"PVRS2") { return Err("Snapshot cifrado inválido.".into()); }
    let loaded = Workspace::load(source).map_err(|e| e.to_string())?;
    validate_workspace(&loaded)?;
    drop(file); Ok(bytes)
}

pub fn dialog_import() -> Result<Option<ImportResult>, String> {
    let selected = rfd::FileDialog::new().set_title("Selecionar CNIS local — não grava automaticamente")
        .add_filter("CNIS em PDF", &["pdf"]).pick_file();
    selected.map(|path| import_path(&path)).transpose()
}
/// Preview and export share the exact renderer. No file/dialog is opened here.
pub fn preview_pdf(calculation: &Calculation, workspace: &Workspace, format: &str, include_personal: bool) -> Result<String, String> {
    use base64::Engine;
    let format = ReportFormat::parse(format)?;
    if !matches!(format, ReportFormat::Pdf(_)) { return Err("A prévia está disponível para os formatos PDF.".into()); }
    let bytes = render_report(calculation, workspace, format, include_personal)?;
    if bytes.len() > 12 * 1024 * 1024 { return Err("PDF maior que 12 MiB. Use Exportar arquivo para salvar a versão completa.".into()); }
    Ok(base64::engine::general_purpose::STANDARD.encode(bytes))
}

pub fn dialog_export(calculation: Calculation, workspace: Workspace, format: String, include_personal: bool) -> Result<Option<String>, String> {
    let format = ReportFormat::parse(&format)?;
    let bytes = render_report(&calculation, &workspace, format, include_personal)?;
    let filename = format!("previus-relatorio.{}", format.extension());
    let selected = rfd::FileDialog::new().set_title("Exportar para NOVO arquivo (não sobrescreve)")
        .add_filter(format.label(), &[format.extension()]).set_file_name(filename).save_file();
    selected.map(|path| write_new(&path, format.extension(), &bytes)).transpose()
}
pub fn dialog_document(text: String) -> Result<Option<String>, String> {
    if text.len() as u64 > MAX_WORKSPACE_BYTES { return Err("Documento excede o limite de 64 MiB.".into()); }
    let selected = rfd::FileDialog::new().set_title("Salvar documento em NOVO arquivo TXT")
        .add_filter("Texto UTF-8", &["txt"]).set_file_name("previus-documento.txt").save_file();
    selected.map(|path| write_new(&path, "txt", text.as_bytes())).transpose()
}
pub fn dialog_backup(bytes: Vec<u8>) -> Result<Option<String>, String> {
    let selected = rfd::FileDialog::new().set_title("Exportar backup cifrado — somente novo arquivo")
        .add_filter("Backup cifrado PVRS2", &["pvr"]).set_file_name("previus-backup.pvr").save_file();
    selected.map(|path| write_new(&path, "pvr", &bytes)).transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use previus_workspace::ManualSalary;
    use std::{path::PathBuf, sync::atomic::{AtomicU64, Ordering}};
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    struct TestDir(PathBuf);
    impl TestDir {
        fn new() -> Self {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join("synthetic-file-tests")
                .join(format!("{}-{}", std::process::id(), SEQUENCE.fetch_add(1, Ordering::Relaxed)));
            fs::create_dir_all(&path).unwrap(); Self(path)
        }
    }
    impl Drop for TestDir { fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); } }
    #[test]
    fn export_never_overwrites_existing_file_or_input_pdf() {
        let dir = TestDir::new(); let existing = dir.0.join("existing.txt"); fs::write(&existing, b"original").unwrap();
        assert!(write_new(&existing, "txt", b"replaced").is_err()); assert_eq!(fs::read(&existing).unwrap(), b"original");
        let input = dir.0.join("cnis.pdf"); fs::write(&input, b"original-pdf").unwrap();
        assert!(write_new(&input, "txt", b"wrong").is_err()); assert_eq!(fs::read(&input).unwrap(), b"original-pdf");
    }
    #[test]
    fn new_export_is_written_once() {
        let dir = TestDir::new(); let output = dir.0.join("new.txt");
        assert!(write_new(&output, "txt", "ação".as_bytes()).is_ok());
        assert_eq!(fs::read_to_string(&output).unwrap(), "ação"); assert!(write_new(&output, "txt", b"second").is_err());
    }
    #[test]
    fn rejects_traversal_device_ads_and_wrong_extensions() {
        let dir = TestDir::new();
        assert!(write_new(&dir.0.join("..\\escape.txt"), "txt", b"x").is_err());
        for name in ["CON.txt", "NUL.txt", "COM1.txt", "LPT1.txt", "file.txt:secret", "trailing.txt."] {
            assert!(write_new(&dir.0.join(name), "txt", b"x").is_err(), "{name}");
        }
        assert!(write_new(Path::new("relative.txt"), "txt", b"x").is_err());
        assert!(write_new(&dir.0.join("wrong.pdf"), "html", b"x").is_err());
        #[cfg(windows)] assert!(write_new(Path::new(r"\\server\share\out.txt"), "txt", b"x").is_err());
    }
    #[test]
    fn accepts_native_pdf_formats_and_refuses_unsupported_docx() {
        assert_eq!(ReportFormat::parse("pdf").unwrap().extension(), "pdf"); assert!(ReportFormat::parse("docx").is_err());
        assert_eq!(ReportFormat::parse("html").unwrap(), ReportFormat::Html);
        assert_eq!(ReportFormat::parse("csv").unwrap(), ReportFormat::Csv);
        assert_eq!(ReportFormat::parse("json").unwrap(), ReportFormat::Json);
    }
    #[test]
    fn private_summary_drops_all_free_text_and_arbitrary_results() {
        let case = Calculation { name: "PRIVATE-NAME".into(), notes: "PRIVATE-NOTE".into(),
            result: Some(json!({"cpf": "PRIVATE-CPF"})), ..Default::default() };
        let text = serde_json::to_string(&summary_json(&case)).unwrap();
        assert!(!text.contains("PRIVATE-")); assert!(text.contains("currency_units_unverified"));
        assert_eq!(summary_json(&case)["legal_validation_performed"], false);
    }
    #[test]
    fn html_escapes_personal_content_and_private_mode_hides_it() {
        let mut w = Workspace::default(); let id = w.id();
        let case = Calculation { id, name: "<script>alert(1)</script>".into(), notes: "PRIVATE-NOTE".into(), ..Default::default() };
        let html = String::from_utf8(render_report(&case, &w, ReportFormat::Html, true).unwrap()).unwrap();
        assert!(!html.contains("<script>"));
        let private = String::from_utf8(render_report(&case, &w, ReportFormat::Html, false).unwrap()).unwrap();
        assert!(!private.contains("PRIVATE-NOTE"));
    }
    #[test]
    fn csv_protects_formula_injection_and_private_mode_omits_free_text() {
        let case = Calculation { manual_salaries: vec![ManualSalary { amount: "=HYPERLINK(\"secret\")".into(),
            indicators: "PRIVATE-INDICATOR".into(), competence: "PRIVATE-COMPETENCE".into(), ..Default::default() }], ..Default::default() };
        let w = Workspace::default();
        let csv = String::from_utf8(render_report(&case, &w, ReportFormat::Csv, true).unwrap()).unwrap();
        assert!(csv.contains("'=HYPERLINK"));
        let private = String::from_utf8(render_report(&case, &w, ReportFormat::Csv, false).unwrap()).unwrap();
        assert!(!private.contains("PRIVATE-")); assert!(!private.contains("HYPERLINK"));
    }
    #[test]
    fn invalid_pdf_is_read_only_and_no_artifact_is_created() {
        let dir = TestDir::new(); let input = dir.0.join("invalid.pdf"); fs::write(&input, b"not a pdf").unwrap();
        assert!(import_path(&input).is_err()); assert_eq!(fs::read(&input).unwrap(), b"not a pdf");
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 1);
    }
    #[cfg(windows)]
    #[test]
    fn real_dpapi_snapshot_is_ciphertext_and_corruption_is_preserved() {
        let dir = TestDir::new(); let path = dir.0.join("workspace.pvr"); let mut w = Workspace::default();
        w.settings.office_name = "SYNTHETIC-PRIVATE-TEXT".into(); w.save(&path).unwrap();
        let bytes = encrypted_snapshot(&path).unwrap(); assert_eq!(bytes, fs::read(&path).unwrap());
        assert!(!String::from_utf8_lossy(&bytes).contains("SYNTHETIC-PRIVATE-TEXT"));
        let output = dir.0.join("backup.pvr"); write_new(&output, "pvr", &bytes).unwrap();
        assert_eq!(Workspace::load(&output).unwrap().settings.office_name, "SYNTHETIC-PRIVATE-TEXT");
        fs::write(&path, b"PVRS2corrupt").unwrap(); assert!(encrypted_snapshot(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"PVRS2corrupt");
    }
    #[test]
    fn private_html_and_csv_drop_arbitrary_indicator_option_and_date_strings() {
        let mut case = Calculation::default();
        case.dib = "PRIVATE-DATE".into(); case.options.correction_index = "PRIVATE-INDEX".into();
        case.options.interest_rate = "PRIVATE-RATE".into();
        case.manual_salaries.push(ManualSalary { id: 1, competence: "PRIVATE-MONTH".into(), amount: "PRIVATE-AMOUNT".into(), indicators: "PRIVATE-INDICATOR".into(), ..Default::default() });
        for format in [ReportFormat::Html, ReportFormat::Csv, ReportFormat::Json] {
            let output = String::from_utf8(render_report(&case, &Workspace::default(), format, false).unwrap()).unwrap();
            assert!(!output.contains("PRIVATE-"));
        }
    }

    fn synthetic_cnis_pdf() -> Vec<u8> {
        let stream = "BT /F1 12 Tf 50 780 Td (CNIS) Tj 0 -20 Td (Competencia Remuneracao Indicadores) Tj 0 -20 Td (01/2020 100,00) Tj ET";
        let objects = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>".to_owned(),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_owned(),
            format!("<< /Length {} >>\nstream\n{}\nendstream", stream.len(), stream),
        ];
        let mut pdf = b"%PDF-1.4\n".to_vec(); let mut offsets = vec![0usize];
        for (index, object) in objects.iter().enumerate() {
            offsets.push(pdf.len());
            pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
        }
        let start = pdf.len(); pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len()).as_bytes());
        for offset in offsets.iter().skip(1) { pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes()); }
        pdf.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n", offsets.len(), start).as_bytes());
        pdf
    }
    #[test]
    fn actual_async_import_reads_synthetic_pdf_without_autosave_or_copy() {
        let dir = TestDir::new(); let path = dir.0.join("synthetic-cnis.pdf");
        let original = synthetic_cnis_pdf(); fs::write(&path, &original).unwrap();
        let result = tauri::async_runtime::block_on(crate::import_cnis_path(path.to_str().unwrap().to_owned())).unwrap();
        assert_eq!(result.salary_entries.len(), 1); assert_eq!(result.salary_months.len(), 1);
        assert_eq!(result.salary_entries[0].amount_centavos, 10_000);
        assert!(!result.metadata.legal_validation_performed); assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(fs::read_dir(&dir.0).unwrap().count(), 1);
    }

}

fn html_escape_value(s:&str)->String{s.replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;").replace('"',"&quot;").replace('\'',"&#39;")}
fn recomputed_rgps_report(case:&Calculation)->String{
    let outcome=match previus_rgps::calculate_case(case){Ok(v)=>v,Err(_)=>return "<h2>Simulação técnica</h2><p>Não foi possível recalcular a hipótese. Nenhuma RMI armazenada foi aceita como resultado verificado.</p>".into()};
    if outcome["status"]!="experimental"{return "<h2>Simulação técnica bloqueada</h2><p>Parâmetros ou origem não revisados. Nenhum valor financeiro foi gerado neste relatório.</p>".into()}
    let Some(summary)=outcome.pointer("/data/native_result/resumo")else{return "<p>O motor não retornou um resumo de cálculo completo.</p>".into()};
    let mut html=String::from("<h2>Simulação técnica recalculada em Rust</h2><p class=\"warning\">EXPERIMENTAL: resultado do motor próprio com parâmetros explícitos. Não certifica legislação, enquadramento ou concessão. Conferência profissional obrigatória.</p><table><tr><th>Grandeza</th><th>Valor do motor</th></tr>");
    for(key,label)in[("rmi","RMI da hipótese"),("rmi_bruta","RMI antes de limites"),("salario_beneficio","Salário de benefício"),("coeficiente","Coeficiente informado"),("competencias","Competências"),("carencia_meses","Carência calculada em meses")] {
        if let Some(v)=summary.get(key){let value=v.as_str().map(str::to_owned).unwrap_or_else(||v.to_string());html.push_str(&format!("<tr><td>{label}</td><td>{}</td></tr>",html_escape_value(&value)));}
    }
    html.push_str("</table><h3>Memória e avisos</h3><ul>");
    if let Some(warnings)=outcome["warnings"].as_array(){for v in warnings{if let Some(s)=v.as_str(){html.push_str(&format!("<li>{}</li>",html_escape_value(s)));}}}
    html.push_str("</ul>");html
}
#[cfg(test)]mod recomputed_report_tests{
    use super::*;
    #[test]fn reviewed_report_recomputes_native_result_and_ignores_stored_json(){
        let mut case:Calculation=serde_json::from_str(include_str!("../../../../crates/previus-rgps/tests/fixtures/calculation-synthetic.json")).unwrap();
        case.result=Some(serde_json::json!({"rmi":"<script>privateFake()</script>"}));
        let html=recomputed_rgps_report(&case);assert!(html.contains("1600.00"));assert!(html.contains("EXPERIMENTAL"));assert!(!html.contains("privateFake"));
    }
    #[test]fn incomplete_report_generates_no_fake_amount(){let case=Calculation::default();let html=recomputed_rgps_report(&case);assert!(html.contains("bloqueada"));assert!(!html.contains("RMI da hipótese"));}
    #[test]fn html_escape_is_literal(){assert_eq!(html_escape_value("<script>&\"'"),"&lt;script&gt;&amp;&quot;&#39;");}
}


#[cfg(test)]mod native_pdf_backend_tests{
 use super::*;
 #[test]fn binary_pdf_formats_are_exact_known_variants(){for input in["pdf","pdf_detailed","pdf_summary","pdf_periods","pdf_salaries","pdf_memory","pdf_transitions","pdf_indicators"]{let format=ReportFormat::parse(input).unwrap();assert_eq!(format.extension(),"pdf");assert!(format.label().contains("PDF"));}assert!(ReportFormat::parse("pdf_unrecognized").is_err());}
 #[test]fn incomplete_assistant_can_generate_conferencia_not_a_fake_financial_result(){
  let c=Calculation{kind:"RGPS".into(),assistant:Some(previus_workspace::AssistantInterview{mode:"assisted".into(),..Default::default()}),result:Some(json!({"rmi":"999999.00"})),..Default::default()};
  let original=serde_json::to_vec(&c).unwrap();let bytes=render_report(&c,&Workspace::default(),ReportFormat::parse("pdf").unwrap(),false).unwrap();assert!(bytes.starts_with(b"%PDF-"));assert!(bytes.len()>500);assert_eq!(original,serde_json::to_vec(&c).unwrap());
 }
 #[test]fn reviewed_synthetic_case_generates_real_binary_memory_pdf(){
  let c:Calculation=serde_json::from_str(include_str!("../../../../crates/previus-rgps/tests/fixtures/calculation-synthetic.json")).unwrap();let bytes=render_report(&c,&Workspace::default(),ReportFormat::parse("pdf_memory").unwrap(),false).unwrap();assert!(bytes.starts_with(b"%PDF-"));assert!(bytes.len()>500);
 }
}

#[cfg(test)]
mod preview_tests {
    use super::*;
    use base64::Engine;
    #[test]
    fn pdf_preview_is_bounded_binary_output_without_a_dialog() {
        let encoded = preview_pdf(&Calculation::default(), &Workspace::default(), "pdf_summary", false).unwrap();
        let bytes = base64::engine::general_purpose::STANDARD.decode(encoded).unwrap();
        assert!(bytes.starts_with(b"%PDF-"));
        assert!(bytes.len() <= 12 * 1024 * 1024);
    }
    #[test]
    fn non_pdf_formats_cannot_be_used_as_a_pdf_preview() {
        for format in ["json", "csv", "html", "missing"] {
            assert!(preview_pdf(&Calculation::default(), &Workspace::default(), format, false).is_err());
        }
    }
}
