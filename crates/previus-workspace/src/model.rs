use chrono::NaiveDate;
use previus_cnis::{Identity, ImportResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

pub const SCHEMA_VERSION: u32 = 2;

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Client {
    pub id: u64,
    pub name: String,
    pub cpf: String,
    pub email: String,
    pub phone: String,
    pub birth_date: String,
    pub sex: Option<String>,
    pub notes: String,
    pub archived: bool,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Calculation {
    pub id: u64,
    pub name: String,
    pub client_id: Option<u64>,
    pub kind: String,
    pub status: String,
    pub identity: Identity,
    pub cnis: Option<ImportResult>,
    pub sex: Option<String>,
    pub disability: Option<String>,
    pub dib: String,
    pub scenario: String,
    /// Interview answers only; missing legacy data stays null, never guessed.
    pub assistant: Option<AssistantInterview>,
    pub manual_links: Vec<ManualLink>,
    pub manual_salaries: Vec<ManualSalary>,
    pub options: CalcOptions,
    pub result: Option<Value>,
    pub notes: String,
    pub archived: bool,
}

/// Answers recorded by the assistant, not proof of entitlement or legal review.
/// Option<bool> deliberately distinguishes an unanswered question from "no".
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AssistantInterview {
    pub mode: String,
    pub current_benefit_received: Option<bool>,
    pub import_requested: Option<bool>,
    pub process_kind: Option<String>,
    pub goal: Option<String>,
    pub review_confirmed: bool,
    pub completed_steps: Vec<String>,
}
impl AssistantInterview {
    /// Structural checks only: incomplete interviews remain valid drafts.
    /// Messages never echo operator answers, imported data or private paths.
    pub fn validate(&self) -> Result<(), String> {
        if !matches!(self.mode.as_str(), "assisted" | "manual") {
            return Err("Modo do assistente inválido; use assisted ou manual.".into());
        }
        const PROCESS_KINDS: [&str; 6] = [
            "Simulação do benefício", "Revisão e valor da causa", "Planejamento futuro",
            "Processo administrativo", "Execução judicial", "Contribuições em atraso",
        ];
        const GOALS: [&str; 3] = [
            "Calcular um novo benefício", "Calcular somente o tempo de contribuição",
            "Restabelecer um benefício",
        ];
        if self.process_kind.as_ref().is_some_and(|value|
            value.chars().count() > 128 || !PROCESS_KINDS.contains(&value.as_str())) {
            return Err("Tipo de processo do assistente inválido ou excede o limite.".into());
        }
        if self.goal.as_ref().is_some_and(|value|
            value.chars().count() > 128 || !GOALS.contains(&value.as_str())) {
            return Err("Finalidade do assistente inválida ou excede o limite.".into());
        }
        const STEPS: [&str; 11] = ["intro", "benefit", "import", "process", "goal",
            "parameters", "periods", "salaries", "policy", "review", "result"];
        if self.completed_steps.len() > 16 {
            return Err("Quantidade de etapas do assistente excede o limite.".into());
        }
        let mut seen = HashSet::new();
        for step in &self.completed_steps {
            if !STEPS.contains(&step.as_str()) || !seen.insert(step.as_str()) {
                return Err("Etapa do assistente inválida ou duplicada.".into());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ManualLink {
    pub id: u64,
    pub employer: String,
    pub start_date: String,
    pub end_date: String,
    pub regime: String,
    pub indicators: String,
    pub confirmed: bool,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ManualSalary {
    pub id: u64,
    pub competence: String,
    pub amount: String,
    pub indicators: String,
    pub confirmed: bool,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CalcOptions {
    pub correction_index: String,
    pub interest_rate: String,
    pub interest_kind: String,
    pub fees_percent: String,
    pub fees_fixed: String,
    pub prescription_date: String,
    pub reference_base: String,
    pub include_thirteenth: bool,
    pub rule_policy: String,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ReferenceTable {
    pub id: u64,
    pub name: String,
    pub unit: String,
    pub periodicity: String,
    pub entries: Vec<ReferenceEntry>,
    pub provenance: String,
    pub verified: bool,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ReferenceEntry {
    pub period: String,
    pub value: String,
    pub note: String,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DocumentTemplate {
    pub id: u64,
    pub name: String,
    pub category: String,
    pub body: String,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Appointment {
    pub id: u64,
    pub title: String,
    pub date: String,
    pub time: String,
    pub client_id: Option<u64>,
    pub notes: String,
    pub completed: bool,
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub office_name: String,
    pub report_footer: String,
    pub light_theme: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Workspace {
    pub schema_version: u32,
    pub revision: u64,
    pub next_id: u64,
    pub clients: Vec<Client>,
    pub calculations: Vec<Calculation>,
    pub reference_tables: Vec<ReferenceTable>,
    pub documents: Vec<DocumentTemplate>,
    pub appointments: Vec<Appointment>,
    pub settings: Settings,
}
impl Default for Workspace {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION, revision: 0, next_id: 1,
            clients: Vec::new(), calculations: Vec::new(), reference_tables: Vec::new(),
            documents: Vec::new(), appointments: Vec::new(), settings: Settings::default(),
        }
    }
}
impl Workspace {
    /// Allocates globally across top-level entities and manual rows. Existing
    /// public-vector edits are scanned so a stale next_id cannot silently collide.
    /// Exhaustion panics *before* mutation; no zero or recycled ID is returned.
    pub fn id(&mut self) -> u64 {
        let max = self.entity_ids().max().unwrap_or(0);
        let id = self.next_id.max(1).max(max.checked_add(1).expect("Workspace ID space exhausted"));
        let next = id.checked_add(1).expect("Workspace ID space exhausted");
        self.next_id = next;
        id
    }
    fn entity_ids(&self) -> impl Iterator<Item = u64> + '_ {
        self.clients.iter().map(|v| v.id)
            .chain(self.calculations.iter().map(|v| v.id))
            .chain(self.reference_tables.iter().map(|v| v.id))
            .chain(self.documents.iter().map(|v| v.id))
            .chain(self.appointments.iter().map(|v| v.id))
            .chain(self.calculations.iter().flat_map(|c| c.manual_links.iter().map(|v| v.id)))
            .chain(self.calculations.iter().flat_map(|c| c.manual_salaries.iter().map(|v| v.id)))
    }
    /// Optional structural review; drafts may be stored even while incomplete.
    /// No names, identifiers, money amounts or user text enter these messages.
    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        let mut ids = HashSet::new();
        for id in self.entity_ids() {
            if id == 0 { warnings.push("Entidade sem ID alocado.".into()); }
            else if !ids.insert(id) { warnings.push("ID duplicado no workspace.".into()); }
        }
        if self.schema_version != SCHEMA_VERSION {
            warnings.push("Versão de schema não suportada.".into());
        }
        let clients: HashSet<_> = self.clients.iter().map(|c| c.id).collect();
        for id in self.calculations.iter().filter_map(|c| c.client_id)
            .chain(self.appointments.iter().filter_map(|a| a.client_id)) {
            if !clients.contains(&id) { warnings.push("Referência a cliente inexistente.".into()); }
        }
        for case in &self.calculations {
            if let Some(assistant) = &case.assistant {
                if let Err(message) = assistant.validate() { warnings.push(message); }
            }
        }
        warnings.sort(); warnings.dedup(); warnings
    }
}

fn text_present(v: &Option<String>) -> bool { v.as_deref().is_some_and(|s| !s.trim().is_empty()) }
fn iso_date(s: &str) -> Option<NaiveDate> {
    if s.len() != 10 || s.as_bytes()[4] != b'-' || s.as_bytes()[7] != b'-' { return None; }
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}
fn pending_date(date: &str, label: &str, out: &mut Vec<String>) -> Option<NaiveDate> {
    let parsed = iso_date(date);
    if parsed.is_none() { out.push(format!("{label} ausente ou inválida (usar AAAA-MM-DD).")); }
    parsed
}

/// Review blockers only, never a decision on financial eligibility, RGPS
/// membership, entitlement, duration of contribution or RMI. PRPPS is not
/// silently removed even if a separate manual row is marked confirmed.
pub fn blockers(case: &Calculation) -> Vec<String> {
    let mut out = Vec::new();
    if !text_present(&case.identity.full_name) { out.push("Identidade: nome não confirmado.".into()); }
    if !text_present(&case.identity.cpf) { out.push("Identidade: CPF não confirmado.".into()); }
    let birth = pending_date(case.identity.birth_date.as_deref().unwrap_or(""), "Data de nascimento", &mut out);
    if !text_present(&case.sex) { out.push("Sexo cadastral não confirmado; não inferir pelo nome.".into()); }
    if !text_present(&case.disability) { out.push("Situação de deficiência não confirmada; não presumir ausência.".into()); }
    let dib = pending_date(&case.dib, "DIB", &mut out);
    if matches!((birth, dib), (Some(b), Some(d)) if b > d) {
        out.push("Nascimento posterior à DIB; revisar datas.".into());
    }
    if case.cnis.is_none() && case.manual_links.is_empty() && case.manual_salaries.is_empty() {
        out.push("Sem CNIS importado ou registros manuais para revisão.".into());
    }
    if let Some(cnis) = &case.cnis {
        let pairs = [(&case.identity.full_name, &cnis.identity.full_name),
            (&case.identity.cpf, &cnis.identity.cpf), (&case.identity.birth_date, &cnis.identity.birth_date)];
        if pairs.iter().any(|(a,b)| text_present(a) && text_present(b) && a != b) {
            out.push("Identidade do cálculo diverge da identidade importada; confirmar sem sobrescrever.".into());
        }
        if cnis.links.iter().any(|l| l.prpps || l.excluded_from_validated_domain) {
            out.push("CNIS contém PRPPS/RPPS ou vínculos excluídos do domínio validado; exigir revisão de regime/CTC.".into());
        }
        if cnis.links.iter().any(|l| l.end_date.is_none()) {
            out.push("CNIS contém vínculos sem data final; último salário não prova término do vínculo.".into());
        }
        if cnis.links.iter().any(|l| l.inferred_last_salary_end.as_ref().is_some_and(|e| e.requires_confirmation)) {
            out.push("Datas finais candidatas do CNIS ainda exigem confirmação.".into());
        }
        if !cnis.warnings.is_empty() || cnis.links.iter().any(|l| !l.indicators.is_empty())
            || cnis.salary_entries.iter().any(|s| !s.indicators.is_empty()) {
            out.push("Indicadores/avisos do CNIS preservados; revisar antes de qualquer cálculo financeiro.".into());
        }
        if cnis.salary_entries.iter().any(|s| s.link_sequence.is_none()) {
            out.push("Existem remunerações sem vínculo reconhecido.".into());
        }
        out.push("Valores do CNIS são centésimos nominais da unidade fonte; moedas históricas, tetos e conversão não validados.".into());
    }
    for link in &case.manual_links {
        if !link.confirmed { out.push("Há vínculos manuais não confirmados.".into()); }
        let start = pending_date(&link.start_date, "Início de vínculo manual", &mut out);
        let end = pending_date(&link.end_date, "Fim de vínculo manual", &mut out);
        if matches!((start, end), (Some(s), Some(e)) if s > e) { out.push("Intervalo de vínculo manual invertido.".into()); }
        let regime = link.regime.trim().to_uppercase();
        if regime != "RGPS" { out.push("Regime de vínculo manual não validado para RGPS.".into()); }
        if !link.indicators.trim().is_empty() { out.push("Indicadores em vínculo manual exigem revisão.".into()); }
    }
    for salary in &case.manual_salaries {
        if !salary.confirmed { out.push("Há remunerações manuais não confirmadas.".into()); }
        if salary.amount.trim().is_empty() { out.push("Valor nominal manual ausente.".into()); }
        if salary.competence.len() != 7 || iso_date(&format!("{}-01", salary.competence)).is_none() {
            out.push("Competência manual ausente ou inválida (usar AAAA-MM).".into());
        }
        if !salary.indicators.trim().is_empty() { out.push("Indicadores em remuneração manual exigem revisão.".into()); }
    }
    out.push("Direito a benefício, tempo de contribuição, RMI e regras financeiras permanecem pendentes de implementação/validação.".into());
    let mut seen = HashSet::new(); out.retain(|v| seen.insert(v.clone())); out
}
