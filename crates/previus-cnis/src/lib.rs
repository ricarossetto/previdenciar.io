//! Local CNIS extraction, not legal validation or a benefits calculator.
//!
//! ImportResult contains sensitive information. Do not log/serialize the whole
//! result without an explicit private destination. Its summary is deliberately
//! free of names, identifiers, employers, personal dates and money values.
//! Source PDF bytes and extracted text are never written by this library.
mod pdf_layout;
use chrono::{Datelike, NaiveDate};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet}, fmt, fs, path::Path, sync::LazyLock};
const MAX_PDF_BYTES: u64 = 64 * 1024 * 1024;
const MAX_PAGES: usize = 500;
const MAX_TEXT_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Identity {
    pub full_name: Option<String>,
    pub cpf: Option<String>,
    pub nit: Option<String>,
    /// ISO YYYY-MM-DD; never inferred.
    pub birth_date: Option<String>,
    pub mother_name: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceRef {
    /// One-based page, line and logical record index.
    pub page: u32,
    pub line: usize,
    pub index: usize,
    /// Byte offset within normalized page text, not a PDF byte offset.
    pub byte_offset: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus { Unvalidated, UnresolvedPrpps }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AmountKind { Remuneration, ContributionColumnUnresolved, ConsolidatedAnnualValue }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SalaryEntry {
    pub competence: String,
    pub amount_centavos: i64,
    pub indicators: Vec<String>,
    pub source: SourceRef,
    pub link_sequence: Option<u32>,
    pub amount_kind: AmountKind,
    pub validation_status: ValidationStatus,
    pub excluded_from_validated_domain: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SalaryMonth {
    pub count: usize,
    /// Arithmetic sum in source hundredths only, not BRL across historical currencies,
    /// an RGPS salary, applied ceiling or RMI.
    pub amount_sum_centavos: i64,
    pub entry_indices: Vec<usize>,
    pub contains_unresolved_prpps: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct InferredLastSalaryEnd {
    pub competence: String,
    /// Month-end candidate, NOT a reported employment termination.
    pub date_candidate: String,
    pub source: SourceRef,
    pub provenance: String,
    pub requires_confirmation: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EmploymentLink {
    pub sequence: u32,
    pub raw_employer: Option<String>,
    pub affiliation: Option<String>,
    pub start_date: Option<String>,
    /// Missing original end stays None; never set today or a guessed end.
    pub end_date: Option<String>,
    pub reported_last_salary_month: Option<String>,
    pub inferred_last_salary_end: Option<InferredLastSalaryEnd>,
    pub indicators: Vec<String>,
    pub prpps: bool,
    pub validation_status: ValidationStatus,
    pub excluded_from_validated_domain: bool,
    pub source: SourceRef,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BenefitStatus { Denied, Unknown }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BenefitRecord {
    pub sequence: u32,
    pub species: Option<u32>,
    pub status: BenefitStatus,
    pub included_as_employment_link: bool,
    pub source: SourceRef,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Severity { Info, Warning }
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Finding {
    pub code: String,
    pub severity: Severity,
    /// Static wording only; no extracted identities, money or employers.
    pub message: String,
    pub count: usize,
    pub source: Option<SourceRef>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportMetadata {
    pub extractor: String,
    pub source_sha256: Option<String>,
    pub page_count: usize,
    pub parsed_pages: usize,
    pub legal_validation_performed: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportResult {
    pub metadata: ImportMetadata,
    pub identity: Identity,
    pub salary_entries: Vec<SalaryEntry>,
    pub salary_months: BTreeMap<String, SalaryMonth>,
    pub links: Vec<EmploymentLink>,
    pub benefits: Vec<BenefitRecord>,
    pub findings: Vec<Finding>,
    pub warnings: Vec<Finding>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportSummary {
    pub metadata: ImportMetadata,
    pub identity_fields_present: usize,
    pub salary_entry_count: usize,
    pub salary_month_count: usize,
    pub months_with_multiple_entries: usize,
    pub employment_link_count: usize,
    pub prpps_link_count: usize,
    pub unresolved_prpps_entry_count: usize,
    pub missing_end_date_count: usize,
    pub end_date_candidate_count: usize,
    pub benefit_record_count: usize,
    pub denied_benefit_record_count: usize,
    pub unassigned_entry_count: usize,
    pub warnings: Vec<Finding>,
}
impl ImportResult {
    pub fn summary(&self) -> ImportSummary {
        ImportSummary {
            metadata: self.metadata.clone(),
            identity_fields_present: [&self.identity.full_name,&self.identity.cpf,&self.identity.nit,
                &self.identity.birth_date,&self.identity.mother_name].iter().filter(|v| v.is_some()).count(),
            salary_entry_count: self.salary_entries.len(),
            salary_month_count: self.salary_months.len(),
            months_with_multiple_entries: self.salary_months.values().filter(|m| m.count > 1).count(),
            employment_link_count: self.links.len(),
            prpps_link_count: self.links.iter().filter(|l| l.prpps).count(),
            unresolved_prpps_entry_count: self.salary_entries.iter().filter(|e| e.excluded_from_validated_domain).count(),
            missing_end_date_count: self.links.iter().filter(|l| l.end_date.is_none()).count(),
            end_date_candidate_count: self.links.iter().filter(|l| l.inferred_last_salary_end.is_some()).count(),
            benefit_record_count: self.benefits.len(),
            denied_benefit_record_count: self.benefits.iter().filter(|b| b.status == BenefitStatus::Denied).count(),
            unassigned_entry_count: self.salary_entries.iter().filter(|e| e.link_sequence.is_none()).count(),
            warnings: self.warnings.clone(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImportError {
    ReadFailed, InputTooLarge, InvalidPdf, EncryptedPdf, UnsupportedPageCount,
    PageExtractionFailed { page: u32 }, NoExtractableText, NotCnis, AmountOverflow,
}
impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ReadFailed => "Unable to read local PDF (path omitted for privacy).",
            Self::InputTooLarge => "Input exceeds the extraction size limit.",
            Self::InvalidPdf => "Invalid or unsupported PDF; no partial result returned.",
            Self::EncryptedPdf => "Encrypted PDF requires explicit decryption outside this importer.",
            Self::UnsupportedPageCount => "PDF page count is empty or exceeds the limit.",
            Self::PageExtractionFailed { .. } => "PDF page extraction failed; no partial result returned.",
            Self::NoExtractableText => "No extractable text; OCR is not implemented.",
            Self::NotCnis => "Text lacks a recognizable CNIS heading and salary table.",
            Self::AmountOverflow => "Currency or sum exceeds signed 64-bit centavos.",
        })
    }
}
impl std::error::Error for ImportError {}
/// Pure Rust; reads only. Explicit page loop avoids the convenience extractor's
/// silent truncation on the first page extraction error.
pub fn import_pdf(path: &Path) -> Result<ImportResult, ImportError> {
    if fs::metadata(path).map_err(|_| ImportError::ReadFailed)?.len() > MAX_PDF_BYTES {
        return Err(ImportError::InputTooLarge);
    }
    let bytes = fs::read(path).map_err(|_| ImportError::ReadFailed)?;
    if bytes.len() as u64 > MAX_PDF_BYTES { return Err(ImportError::InputTooLarge); }
    let document = lopdf::Document::load_mem(&bytes).map_err(|_| ImportError::InvalidPdf)?;
    if document.is_encrypted() { return Err(ImportError::EncryptedPdf); }
    let page_ids = document.get_pages();
    if page_ids.is_empty() || page_ids.len() > MAX_PAGES { return Err(ImportError::UnsupportedPageCount); }
    let mut pages = Vec::with_capacity(page_ids.len());
    let mut total_bytes = 0usize;
    for page_number in page_ids.keys() {
        let mut output = pdf_layout::PositionedText::new(pdf_layout::page_rotation(&document, page_ids[page_number]));
        pdf_extract::output_doc_page(&document, &mut output, *page_number)
            .map_err(|_| ImportError::PageExtractionFailed { page: *page_number })?;
        let text = output.into_text();
        total_bytes = total_bytes.saturating_add(text.len());
        if total_bytes > MAX_TEXT_BYTES { return Err(ImportError::InputTooLarge); }
        pages.push(text);
    }
    if pages.iter().all(|p| p.trim().is_empty()) { return Err(ImportError::NoExtractableText); }
    let mut result = parse_pages(&pages)?;
    result.metadata.extractor = "pdf-extract 0.7.12 / lopdf 0.34.0; local Rust".into();
    result.metadata.source_sha256 = Some(format!("{:x}", Sha256::digest(&bytes)));
    Ok(result)
}
/// Form-feed separates pages. Text is untrusted data, never instructions.
pub fn import_text(text: &str) -> Result<ImportResult, ImportError> {
    if text.len() > MAX_TEXT_BYTES { return Err(ImportError::InputTooLarge); }
    let pages: Vec<String> = text.split('\u{000c}').map(str::to_owned).collect();
    if pages.len() > MAX_PAGES { return Err(ImportError::UnsupportedPageCount); }
    if text.trim().is_empty() { return Err(ImportError::NoExtractableText); }
    parse_pages(&pages)
}

struct Patterns {
    salary: Regex, contribution_salary: Regex, date: Regex, month: Regex, sequence: Regex, indicator: Regex,
    cpf: Regex, nit: Regex, name: Regex, birth: Regex, mother: Regex,
    company_code: Regex, nit_anywhere: Regex, affiliation: Regex, species: Regex, currency: Regex,
}
static RX: LazyLock<Patterns> = LazyLock::new(|| Patterns {
    salary: Regex::new(r"\b(0[1-9]|1[0-2])/(\d{4})\s+(?:R\$\s*)?(-?(?:\d{1,3}(?:\.\d{3})+|\d+),\d{2})\b").unwrap(),
    contribution_salary: Regex::new(r"\b(0[1-9]|1[0-2])/(\d{4})\s+\d{2}/\d{2}/\d{4}\s+(-?(?:\d{1,3}(?:\.\d{3})+|\d+),\d{2})\s+(-?(?:\d{1,3}(?:\.\d{3})+|\d+),\d{2})\b").unwrap(),
    date: Regex::new(r"\b\d{2}/\d{2}/\d{4}\b").unwrap(),
    month: Regex::new(r"\b(0[1-9]|1[0-2])/(\d{4})\b").unwrap(),
    sequence: Regex::new(r"^\s*(\d{1,3})\s+(.+)").unwrap(),
    indicator: Regex::new(r"\b[A-Z][A-Z0-9]*(?:-[A-Z0-9]+)+\b|\bPRPPS\b").unwrap(),
    cpf: Regex::new(r"(?i)\bCPF\s*:\s*([\d.\-]+)").unwrap(),
    nit: Regex::new(r"(?i)\bNIT\s*:\s*([\d.\-]+)").unwrap(),
    name: Regex::new(r"(?im)\bNome\s*:\s*([^\n]+)").unwrap(),
    birth: Regex::new(r"(?i)Data\s+de\s+nascimento\s*:\s*(\d{2}/\d{2}/\d{4})").unwrap(),
    mother: Regex::new(r"(?im)Nome\s+d[ae]\s+m[ãa]e\s*:\s*([^\n]+)").unwrap(),
    company_code: Regex::new(r"^\s*\d{2}\.\d{3}\.\d{3}(?:[/.-]\d{4}[-.]?\d{2})?\s*").unwrap(),
    nit_anywhere: Regex::new(r"\b\d{3}\.\d{5}\.\d{2}[-.]?\d\b").unwrap(),
    affiliation: Regex::new(r"(?i)Empregado\s+ou\s+Agente\s+P[úu]blico|Contribuinte\s+Individual|Facultativo|Trabalhador\s+Avulso").unwrap(),
    species: Regex::new(r"\b(\d{1,3})\s*-\s*[A-Za-zÁÉÍÓÚÂÊÔÃÕÇ]").unwrap(),
    currency: Regex::new(r"[\d.]+,\d{2}").unwrap(),
});
fn normalize(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n").replace(['\u{00a0}', '\u{202f}'], " ")
}
fn folded(text: &str) -> String {
    text.to_lowercase().replace(['á','à','ã','â'], "a").replace(['é','ê'], "e")
        .replace('í', "i").replace(['ó','ô','õ'], "o").replace('ú', "u").replace('ç', "c")
}
fn clean(s: &str) -> String { s.split_whitespace().collect::<Vec<_>>().join(" ") }
/// Cabeçalho da tabela de valores consolidados por ano civil: a posição de
/// cada mês na linha. A posição é o que permite dizer a que mês pertence cada
/// valor, mesmo quando a linha não traz os doze meses.
fn consolidated_month_header(line: &str) -> Option<Vec<(usize, u32)>> {
    const MONTHS: [&str; 12] = ["jan","fev","mar","abr","mai","jun","jul","ago","set","out","nov","dez"];
    let folded = folded(line);
    let mut header = Vec::new();
    let mut cursor = 0usize;
    for (index, name) in MONTHS.iter().enumerate() {
        let found = folded[cursor..].find(name)?;
        let start = cursor + found;
        let end = start + name.len();
        header.push((end, index as u32 + 1));
        cursor = end;
    }
    Some(header)
}
/// Linha da tabela de valores consolidados: ano e valores com a posição de
/// cada valor na linha. Rótulo e valor são alinhados à direita na coluna, o
/// que faz a comparação de borda direita ser a leitura fiel da coluna.
fn parse_consolidated_row(line: &str, header: &[(usize, u32)]) -> Option<(i32, Vec<(usize, u32, String)>)> {
    let digits = line.len() - line.trim_start().len();
    let head = &line[digits..];
    let year_end = head.find(|c: char| !c.is_ascii_digit())?;
    let year: i32 = head[..year_end].trim().parse().ok()?;
    if !(1900..=2100).contains(&year) { return None; }
    let values = RX.currency.find_iter(line).filter_map(|m| {
        let month = header.iter().rev().find(|(end, _)| *end <= m.end()).map(|(_, month)| *month)?;
        Some((m.start(), month, m.as_str().to_owned()))
    }).collect();
    Some((year, values))
}
fn digits(s: &str) -> String { s.chars().filter(char::is_ascii_digit).collect() }
fn iso_date(s: &str) -> Option<String> {
    NaiveDate::parse_from_str(s, "%d/%m/%Y").ok().map(|d| d.format("%Y-%m-%d").to_string())
}
fn codes(s: &str) -> Vec<String> {
    let mut values: Vec<String> = RX.indicator.find_iter(s).map(|m| m.as_str().to_owned()).collect();
    values.sort(); values.dedup(); values
}
fn money(s: &str) -> Result<i64, ImportError> {
    let normalized = s.replace('.', "");
    let (whole, cents) = normalized.split_once(',').ok_or(ImportError::AmountOverflow)?;
    let negative = whole.starts_with('-');
    let whole: i64 = whole.trim_start_matches('-').parse().map_err(|_| ImportError::AmountOverflow)?;
    let cents: i64 = cents.parse().map_err(|_| ImportError::AmountOverflow)?;
    let value = whole.checked_mul(100).and_then(|n| n.checked_add(cents)).ok_or(ImportError::AmountOverflow)?;
    Ok(if negative { -value } else { value })
}
fn source(page: usize, line: usize, index: usize, offset: usize) -> SourceRef {
    SourceRef { page: (page + 1) as u32, line: line + 1, index, byte_offset: offset }
}
fn finding(code: &str, severity: Severity, message: &str, count: usize, location: Option<SourceRef>) -> Finding {
    Finding { code: code.into(), severity, message: message.into(), count, source: location }
}
fn capture_text(rx: &Regex, text: &str) -> Option<String> {
    rx.captures(text).map(|c| clean(&c[1])).filter(|s| !s.is_empty())
}
fn parse_identity(page: &str) -> Identity {
    Identity {
        full_name: capture_text(&RX.name, page),
        cpf: capture_text(&RX.cpf, page).map(|s| digits(&s)).filter(|s| s.len() == 11),
        nit: capture_text(&RX.nit, page).map(|s| digits(&s)).filter(|s| s.len() == 11),
        birth_date: capture_text(&RX.birth, page).and_then(|s| iso_date(&s)),
        mother_name: capture_text(&RX.mother, page),
    }
}
fn merge_field(current: &mut Option<String>, incoming: Option<String>) -> bool {
    if let Some(new) = incoming {
        match current {
            None => { *current = Some(new); false },
            Some(old) => *old != new,
        }
    } else { false }
}
fn is_relation_heading(text: &str) -> bool {
    let lower = folded(text);
    lower.contains("origem do vinculo") || lower.contains("codigo emp.")
}
fn is_table_heading(text: &str) -> bool {
    let lower = folded(text);
    // Um cabeçalho de tabela é curto. A nota de rodapé do CNIS ("O segurado
    // somente terá reconhecida como tempo de contribuição ... competência ...")
    // cita "competência" e "contribuição" e não pode virar cabeçalho.
    if lower.trim().len() > 200 { return false; }
    lower.trim() == "remuneracoes" || lower.trim() == "contribuicoes"
        || lower.contains("competencia") && (lower.contains("remuneracao") || lower.contains("contribuicao"))
}
fn is_boundary(text: &str) -> bool {
    let lower = folded(text);
    lower.contains("consolidado por ano civil") || lower.contains("consolidados por ano civil")
        || lower.contains("legenda de indicadores") || lower.starts_with("o inss podera")
        || lower.starts_with("o segurado somente tera")
}
/// Seção que realmente encerra a tabela de remunerações. O rodapé da página
/// ("O INSS poderá rever...") não encerra: a tabela do CNIS continua na
/// página seguinte sem repetir o cabeçalho.
fn closes_salary_section(text: &str) -> bool {
    let lower = folded(text);
    lower.contains("consolidado por ano civil") || lower.contains("consolidados por ano civil")
        || lower.contains("legenda de indicadores")
}
fn parse_relation(block: &str, src: SourceRef, affiliation_hint: Option<&str>) -> Option<Result<EmploymentLink, BenefitRecord>> {
    let text = clean(block);
    let cap = RX.sequence.captures(&text)?;
    let sequence: u32 = cap[1].parse().ok()?;
    let body = &cap[2];
    let lower = folded(body);
    if lower.contains("beneficio") {
        return Some(Err(BenefitRecord {
            sequence, species: RX.species.captures(body).and_then(|c| c[1].parse().ok()),
            status: if lower.contains("indeferido") { BenefitStatus::Denied } else { BenefitStatus::Unknown },
            included_as_employment_link: false, source: src,
        }));
    }
    let affiliation = RX.affiliation.find(body);
    let affiliation_text = affiliation.as_ref().map(|m| clean(m.as_str()))
        .or_else(|| affiliation_hint.map(str::to_owned))?;
    let first_date = RX.date.find(body).map(|d| d.start()).unwrap_or(body.len());
    let employer_end = affiliation.as_ref().map(|m| m.start()).unwrap_or(first_date).min(first_date);
    let employer = RX.nit_anywhere.replace_all(&body[..employer_end], "");
    let employer = RX.company_code.replace(&employer, "");
    let employer = clean(&employer);
    let period = body;
    let dates: Vec<&str> = RX.date.find_iter(period).map(|d| d.as_str()).collect();
    let start_date = dates.first().and_then(|d| iso_date(d));
    let end_date = dates.get(1).and_then(|d| iso_date(d));
    // Full dates must not be mistaken for salary month suffixes.
    let without_dates = RX.date.replace_all(period, " ");
    let reported_last_salary_month = RX.month.captures(&without_dates).map(|m| format!("{}-{}", &m[2], &m[1]));
    let mut indicators: Vec<String> = block.lines().filter_map(|l| {
        l.find("Indicadores:").or_else(|| l.find("indicadores:")).map(|i| &l[i + "Indicadores:".len()..])
    }).flat_map(codes).collect();
    indicators.sort(); indicators.dedup();
    let prpps = indicators.iter().any(|i| i == "PRPPS");
    Some(Ok(EmploymentLink {
        sequence, raw_employer: if employer.is_empty() { None } else { Some(employer) },
        affiliation: Some(affiliation_text), start_date, end_date, reported_last_salary_month,
        inferred_last_salary_end: None, indicators, prpps,
        validation_status: if prpps { ValidationStatus::UnresolvedPrpps } else { ValidationStatus::Unvalidated },
        excluded_from_validated_domain: prpps, source: src,
    }))
}

fn parse_pages(pages: &[String]) -> Result<ImportResult, ImportError> {
    let mut result = ImportResult {
        metadata: ImportMetadata { extractor: "text / synthetic or caller-provided".into(), source_sha256: None,
            page_count: pages.len(), parsed_pages: pages.len(), legal_validation_performed: false },
        identity: Identity::default(), salary_entries: Vec::new(), salary_months: BTreeMap::new(),
        links: Vec::new(), benefits: Vec::new(), findings: Vec::new(), warnings: Vec::new(),
    };
    let mut seen_cnis = false;
    let mut table_count = 0;
    let mut current_sequence = None;
    let mut ignored_money_pairs = 0;
    let mut negative_entries = 0;
    let mut ambiguous_relations = 0;
    let mut nonadjacent_rows = 0;
    let mut split_indicator_rows = 0;
    let mut consolidated = false;
    let mut consolidated_header: Option<Vec<(usize, u32)>> = None;
    let mut consolidated_candidates: Vec<SalaryEntry> = Vec::new();
    // O contexto da tabela atravessa a quebra de página: no CNIS a tabela de
    // contribuições continua na página seguinte sem repetir o cabeçalho, e
    // perder esse contexto descartaria as linhas seguintes.
    let mut table = false;
    let mut amount_kind = AmountKind::Remuneration;
    let mut explicit_salary_column = false;
    for (page_index, raw_page) in pages.iter().enumerate() {
        let page = normalize(raw_page);
        seen_cnis |= folded(&page).contains("cnis");
        let identity = parse_identity(&page);
        let conflict = [
            merge_field(&mut result.identity.full_name, identity.full_name),
            merge_field(&mut result.identity.cpf, identity.cpf),
            merge_field(&mut result.identity.nit, identity.nit),
            merge_field(&mut result.identity.birth_date, identity.birth_date),
            merge_field(&mut result.identity.mother_name, identity.mother_name),
        ].into_iter().filter(|v| *v).count();
        if conflict > 0 {
            result.warnings.push(finding("identity_conflict", Severity::Warning,
                "Repeated identity fields conflict; first observed value retained for review.", conflict,
                Some(source(page_index, 0, 1, 0))));
        }
        let lines: Vec<&str> = page.lines().collect();
        let mut offset = 0;
        let mut relation_expected = false;
        let mut line_index = 0;
        while line_index < lines.len() {
            let line = lines[line_index];
            let lower = folded(line);
            if lower.trim() == "relacoes previdenciarias" { table = current_sequence.is_some(); }
            if is_relation_heading(line) { relation_expected = true; table = false; }
            if relation_expected && RX.sequence.is_match(line) && !RX.salary.is_match(line) {
                let first_line = line_index;
                let first_offset = offset;
                let mut end = first_line + 1;
                while end < lines.len() && !is_table_heading(lines[end])
                    && !is_relation_heading(lines[end]) && !is_boundary(lines[end])
                    && !RX.salary.is_match(lines[end]) { end += 1; }
                let block = lines[first_line..end].join("\n");
                let src = source(page_index, first_line, result.links.len() + result.benefits.len() + 1, first_offset);
                let hint = if first_line > 0 && folded(lines[first_line-1]).contains("empregado ou agente")
                    && folded(&block).contains("publico") { Some("Empregado ou Agente Público") } else { None };
                match parse_relation(&block, src, hint) {
                    Some(Ok(link)) => {
                        current_sequence = Some(link.sequence);
                        if !result.links.iter().any(|l| l.sequence == link.sequence) { result.links.push(link); }
                        else { ambiguous_relations += 1; }
                    },
                    Some(Err(benefit)) => { current_sequence = None; result.benefits.push(benefit); },
                    None => { current_sequence = None; ambiguous_relations += 1; },
                }
                relation_expected = false; table = false;
                for item in &lines[first_line..end] { offset += item.len() + 1; }
                line_index = end; continue;
            }
            if is_table_heading(line) {
                table = true; relation_expected = false; table_count += 1;
                explicit_salary_column = lower.contains("salario") && lower.contains("contribuicao");
                amount_kind = if lower.contains("contribuicao") && !lower.contains("remuneracao") {
                    AmountKind::ContributionColumnUnresolved
                } else { AmountKind::Remuneration };
            }
            // O rodapé do CNIS ("O INSS poderá rever...") fecha a página, não a
            // tabela: a continuação na página seguinte segue com o mesmo
            // cabeçalho implícito. Só uma nova seção encerra a tabela.
            if is_boundary(line) { relation_expected = false; }
            if closes_salary_section(line) { table = false; explicit_salary_column = false; }
            // Valores consolidados por ano civil: a posição do valor no texto
            // diz a que mês ele pertence. A tabela não declara vínculo,
            // empregador nem origem do pagamento: nada é atribuído por
            // inferência silenciosa aqui.
            if lower.contains("valores consolidados por ano civil") {
                consolidated = true; consolidated_header = None;
            }
            if consolidated {
                if consolidated_header.is_none() {
                    if let Some(header) = consolidated_month_header(line) { consolidated_header = Some(header); }
                } else if let Some(header) = consolidated_header.clone() {
                    if let Some((year, values)) = parse_consolidated_row(line, &header) {
                        for (index, month, text) in values {
                            let amount_centavos = money(&text)?;
                            consolidated_candidates.push(SalaryEntry{
                                competence: format!("{year}-{month:02}"), amount_centavos, indicators: vec![],
                                source: source(page_index, line_index, consolidated_candidates.len()+1, offset+index),
                                link_sequence: None, amount_kind: AmountKind::ConsolidatedAnnualValue,
                                validation_status: ValidationStatus::Unvalidated, excluded_from_validated_domain: false,
                            });
                        }
                    }
                }
            }
            // Bounded cooperative layout: explicit month, one to three
            // contractor/establishment CNPJ tokens, an optional known service
            // form ("Normal", "Cooperado", "Não Cooperado", "Terceirização")
            // and the remuneration. Any other word between the month and the
            // amount is refused: no arbitrary text/amount guessing.
            static COOPERATIVE: LazyLock<Regex> = LazyLock::new(||Regex::new(r"(?i)\b(0[1-9]|1[0-2])/(\d{4})\s+(?:\d{2}\.\d{3}\.\d{3}(?:/\d{4}-\d{2})?\s+){1,3}(?:(?:normal|cooperado|n[^\W\d_]{0,4}o(?:\s+cooperado)?|terceiriza[^\W\d_]{0,4}o)\s+){0,2}(?:R\$\s*)?(-?(?:\d{1,3}(?:\.\d{3})+|\d+),\d{2})\b").unwrap());
            let cooperative_rows:Vec<_>=if table{COOPERATIVE.captures_iter(line).collect()}else{vec![]};
            for (i,cap) in cooperative_rows.iter().enumerate(){
                let span=cap.get(0).unwrap();let end=cooperative_rows.get(i+1).map(|next|next.get(0).unwrap().start()).unwrap_or(line.len());
                let amount_centavos=money(&cap[3])?;if amount_centavos<0{negative_entries+=1;}
                result.salary_entries.push(SalaryEntry{competence:format!("{}-{}",&cap[2],&cap[1]),amount_centavos,indicators:codes(&line[span.end()..end]),source:source(page_index,line_index,result.salary_entries.len()+1,offset+span.start()),link_sequence:current_sequence,amount_kind:AmountKind::Remuneration,validation_status:ValidationStatus::Unvalidated,excluded_from_validated_domain:false});
            }
            // Only an explicitly labelled salary column can feed remuneration.
            // The paid contribution is never divided by an assumed contribution rate.
            let payment_rows: Vec<_> = if table && explicit_salary_column {
                RX.contribution_salary.captures_iter(line).collect()
            } else { vec![] };
            let payment_spans: Vec<_> = payment_rows.iter().map(|cap| cap.get(0).unwrap().range()).collect();
            for (i, cap) in payment_rows.iter().enumerate() {
                let span = cap.get(0).unwrap();
                let end = payment_rows.get(i+1).map(|next| next.get(0).unwrap().start()).unwrap_or(line.len());
                let amount_centavos = money(&cap[4])?;
                if amount_centavos < 0 { negative_entries += 1; }
                result.salary_entries.push(SalaryEntry {
                    competence: format!("{}-{}", &cap[2], &cap[1]), amount_centavos,
                    indicators: codes(&line[span.end()..end]),
                    source: source(page_index, line_index, result.salary_entries.len()+1, offset+span.start()),
                    link_sequence: current_sequence, amount_kind: AmountKind::Remuneration,
                    validation_status: ValidationStatus::Unvalidated, excluded_from_validated_domain: false,
                });
            }
            let matches: Vec<_> = RX.salary.captures_iter(line).collect();
            if !table { ignored_money_pairs += matches.len(); }
            else {
                if matches.is_empty() && payment_rows.is_empty() && cooperative_rows.is_empty() && RX.month.is_match(line) && RX.currency.is_match(line) {
                    nonadjacent_rows += 1;
                }
                for (i, cap) in matches.iter().enumerate() {
                    let span = cap.get(0).unwrap();
                    if payment_spans.iter().any(|range| range.start <= span.start() && span.start() < range.end) { continue; }
                    if span.start() > 0 && line.as_bytes()[span.start() - 1] == b'/' { continue; }
                    let tail_end = matches.get(i + 1).map(|next| next.get(0).unwrap().start()).unwrap_or(line.len());
                    let tail = &line[span.end()..tail_end];
                    let mut indicators = codes(tail);
                    // Known CNIS codes may wrap after a hyphen. Rejoin only the
                    // final entry's tail with the next line's prefix, never its amounts.
                    if i + 1 == matches.len() {
                        if let Some(next_line) = lines.get(line_index + 1) {
                            let prefix_end = RX.salary.find(next_line).map(|m| m.start()).unwrap_or(next_line.len());
                            let prefix = next_line[..prefix_end].trim();
                            if let Some(joined) = joined_indicator(tail, prefix) {
                                let partial = tail.split_whitespace().last().unwrap_or("").trim_end_matches('-');
                                indicators.retain(|c| c != partial);
                                indicators.push(joined);
                                split_indicator_rows += 1;
                            }
                        }
                    }
                    let amount_centavos = money(&cap[3])?;
                    if amount_centavos < 0 { negative_entries += 1; }
                    result.salary_entries.push(SalaryEntry {
                        competence: format!("{}-{}", &cap[2], &cap[1]), amount_centavos, indicators,
                        source: source(page_index, line_index, result.salary_entries.len() + 1, offset + span.start()),
                        link_sequence: current_sequence, amount_kind: amount_kind.clone(),
                        validation_status: ValidationStatus::Unvalidated, excluded_from_validated_domain: false,
                    });
                }
            }
            offset += line.len() + 1; line_index += 1;
        }
    }
    if !seen_cnis || table_count == 0 { return Err(ImportError::NotCnis); }
    // Valores consolidados por ano civil entram só para competências que a
    // leitura por vínculo não trouxe, e ficam sem vínculo quando mais de um
    // período os cobre: a origem não declara a origem do pagamento.
    let mut consolidated_skipped = 0u32;
    let mut consolidated_attributed = 0u32;
    let mut consolidated_ambiguous = 0u32;
    let mut covered: BTreeSet<String> = result.salary_entries.iter().map(|e| e.competence.clone()).collect();
    for mut candidate in consolidated_candidates {
        if covered.contains(&candidate.competence) { consolidated_skipped += 1; continue; }
        let month_start = format!("{}-01", candidate.competence);
        let month_end = format!("{}-28", candidate.competence);
        let covering: Vec<u32> = result.links.iter().filter(|link| {
            link.start_date.as_deref().is_some_and(|start| start <= month_end.as_str())
                && link.end_date.as_deref().is_some_and(|end| end >= month_start.as_str())
        }).map(|link| link.sequence).collect();
        match covering.as_slice() {
            [only] => { candidate.link_sequence = Some(*only); consolidated_attributed += 1; }
            _ => { consolidated_ambiguous += 1; }
        }
        covered.insert(candidate.competence.clone());
        result.salary_entries.push(candidate);
    }
    for entry in &mut result.salary_entries {
        if let Some(link) = result.links.iter().find(|l| Some(l.sequence) == entry.link_sequence) {
            if link.prpps { entry.validation_status = ValidationStatus::UnresolvedPrpps; entry.excluded_from_validated_domain = true; }
        }
        let month = result.salary_months.entry(entry.competence.clone()).or_default();
        month.count += 1;
        month.amount_sum_centavos = month.amount_sum_centavos.checked_add(entry.amount_centavos).ok_or(ImportError::AmountOverflow)?;
        month.entry_indices.push(entry.source.index);
        month.contains_unresolved_prpps |= entry.excluded_from_validated_domain;
    }
    for link in &mut result.links {
        if link.end_date.is_none() {
            let latest = result.salary_entries.iter().filter(|e| e.link_sequence == Some(link.sequence)).max_by_key(|e| &e.competence);
            let candidate = latest.map(|e| (e.competence.clone(), e.source.clone(), "observed_salary_month"))
                .or_else(|| link.reported_last_salary_month.as_ref().map(|m| (m.clone(), link.source.clone(), "reported_last_salary_month")));
            if let Some((month, location, provenance)) = candidate {
                if let Some(date_candidate) = month_end(&month) {
                    link.inferred_last_salary_end = Some(InferredLastSalaryEnd {
                        competence: month, date_candidate, source: location, provenance: provenance.into(), requires_confirmation: true,
                    });
                }
            }
        }
    }
    result.findings.push(finding("local_extraction", Severity::Info,
        "Input processed in memory locally; no backend, Python process or OCR was used.", pages.len(), None));
    result.warnings.push(finding("not_legal_validation", Severity::Warning,
        "Extraction is not entitlement, RMI, RGPS/RPPS transfer or legal validation; sums are arithmetic only.", 1, None));
    result.warnings.push(finding("currency_units_unverified", Severity::Warning,
        "Nominal amounts kept in 2-decimal source units; historical currency not decoded or converted. Do not interpret as current BRL", 1, None));
    result.warnings.push(finding("bounded_layout_support", Severity::Warning,
        "Only recognized CNIS relation/remuneration tables are parsed; other layouts or columns require review.", 1, None));
    let prpps_count = result.links.iter().filter(|l| l.prpps).count();
    if prpps_count > 0 { result.warnings.push(finding("unresolved_prpps", Severity::Warning,
        "PRPPS relations and their salary entries remain unresolved and excluded from the validated RGPS domain; CTC and review are required.", prpps_count, None)); }
    let no_end_count = result.links.iter().filter(|l| l.end_date.is_none()).count();
    if no_end_count > 0 { result.warnings.push(finding("missing_end_date", Severity::Warning,
        "Missing original end dates preserved. Last-salary month-end candidates are not verified termination dates; confirm manually.", no_end_count, None)); }
    if !result.benefits.is_empty() { result.warnings.push(finding("benefit_not_employment", Severity::Warning,
        "Benefit rows kept separate from employment; denied or unknown benefits are not treated as currently received.", result.benefits.len(), None)); }
    if ignored_money_pairs > 0 { result.warnings.push(finding("outside_salary_table", Severity::Warning,
        "Month/currency pairs outside recognized salary tables were not imported.", ignored_money_pairs, None)); }
    if ambiguous_relations > 0 { result.warnings.push(finding("ambiguous_relation", Severity::Warning,
        "Some relation headers were ambiguous or repeated; their association requires review.", ambiguous_relations, None)); }
    let unassigned = result.salary_entries.iter().filter(|e| e.link_sequence.is_none()).count();
    if unassigned > 0 { result.warnings.push(finding("unassigned_salary_entries", Severity::Warning,
        "Some salary entries could not be assigned to a recognized relation; do not infer an employer.", unassigned, None)); }
    if negative_entries > 0 { result.warnings.push(finding("negative_amounts", Severity::Warning,
        "Negative amounts retained as reported; review corrections and column semantics.", negative_entries, None)); }
    let pending = result.salary_entries.iter().filter(|e| e.amount_kind == AmountKind::ContributionColumnUnresolved).count();
    if pending > 0 { result.warnings.push(finding("contribution_column_unresolved", Severity::Warning,
        "Contribution-column amounts were not converted into remuneration and cannot feed a benefit calculation.", pending, None)); }
    if nonadjacent_rows > 0 { result.warnings.push(finding("nonadjacent_table_columns", Severity::Warning,
        "Some table rows have employer columns between month and amount; they are not imported by the adjacent month/currency parser and require review.", nonadjacent_rows, None)); }
    if split_indicator_rows > 0 { result.findings.push(finding("wrapped_indicator_rejoined", Severity::Info,
        "Recognizable hyphen-wrapped indicator tokens were rejoined without borrowing values from the next row.", split_indicator_rows, None)); }
    if consolidated_attributed > 0 { result.findings.push(finding("consolidated_annual_value_attributed", Severity::Info,
        "Competências lidas da tabela de valores consolidados por ano civil foram atribuídas ao único vínculo que cobre o mês; a tabela não declara a origem do pagamento.", consolidated_attributed as usize, None)); }
    if consolidated_ambiguous > 0 { result.warnings.push(finding("consolidated_annual_value_unassigned", Severity::Warning,
        "Competências da tabela de valores consolidados por ano civil sem vínculo único na origem: a atribuição exige conferência documental.", consolidated_ambiguous as usize, None)); }
    if consolidated_skipped > 0 { result.findings.push(finding("consolidated_annual_value_already_covered", Severity::Info,
        "Valores consolidados por ano civil não somados onde a competência já havia sido lida pelo vínculo, para não duplicar o mês.", consolidated_skipped as usize, None)); }
    if result.salary_entries.is_empty() { result.warnings.push(finding("no_salary_entries", Severity::Warning,
        "No month/currency remuneration rows found; OCR or a different layout may be needed.", 1, None)); }
    Ok(result)
}
fn month_end(month: &str) -> Option<String> {
    let first = NaiveDate::parse_from_str(&format!("{month}-01"), "%Y-%m-%d").ok()?;
    let (year, month) = if first.month() == 12 { (first.year() + 1, 1) } else { (first.year(), first.month() + 1) };
    NaiveDate::from_ymd_opt(year, month, 1)?.pred_opt().map(|d| d.format("%Y-%m-%d").to_string())
}

fn joined_indicator(tail: &str, prefix: &str) -> Option<String> {
    let partial = tail.split_whitespace().last()?;
    if !partial.ends_with('-') || !partial.chars().all(|c| c.is_ascii_uppercase() || c == '-') { return None; }
    let suffix = prefix.split_whitespace().next()?;
    if !suffix.chars().all(|c| c.is_ascii_uppercase() || c == '-') { return None; }
    let joined = format!("{partial}{suffix}");
    let known = ["PREC-MENOR-MIN", "IREM-INDPEND", "IREC-INDPEND", "PREM-EMPR", "PREM-FVIN", "PEXT", "AVRC-DEF"];
    known.contains(&joined.as_str()).then_some(joined)
}
