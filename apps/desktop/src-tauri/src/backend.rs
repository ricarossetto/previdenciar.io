//! Storage boundary: fixed path, explicit saves, sticky load failure and no user-data logging.
use previus_workspace::{Calculation, Workspace, MAX_WORKSPACE_BYTES};
use serde::Serialize;
use serde_json::Value;
use std::{fs, path::{Path, PathBuf}, sync::{Arc, Mutex}};

#[derive(Serialize)]
pub struct BootResult {
    pub workspace: Workspace,
    pub storage_ready: bool,
    pub message: Option<String>,
}

pub trait WorkspaceStore {
    fn read(&self) -> Result<Option<Workspace>, String>;
    fn write(&self, workspace: &mut Workspace) -> Result<(), String>;
}

pub struct NativeStore { path: Result<PathBuf, String> }
impl NativeStore {
    pub fn fixed() -> Self {
        let path = std::env::var_os("LOCALAPPDATA")
            .ok_or_else(|| "LOCALAPPDATA indisponível; gravação bloqueada.".to_owned())
            .and_then(|base| {
                let base = PathBuf::from(base);
                crate::files::validate_local_path(&base.join("PreviusRust").join("workspace.pvr"))?;
                Ok(base.join("PreviusRust").join("workspace.pvr"))
            });
        Self { path }
    }
    pub fn path(&self) -> Result<&Path, String> {
        self.path.as_deref().map_err(Clone::clone)
    }
}
impl WorkspaceStore for NativeStore {
    fn read(&self) -> Result<Option<Workspace>, String> {
        let path = self.path()?;
        // Inspect existing ancestors even when the application directory is absent.
        crate::files::check_ancestors(path.parent().ok_or("Diretório inválido.")?, true)?;
        match fs::symlink_metadata(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err("Não foi possível inspecionar o armazenamento; gravação bloqueada.".into()),
            Ok(_) => Workspace::load(path).map(Some).map_err(|e| e.to_string()),
        }
    }
    fn write(&self, workspace: &mut Workspace) -> Result<(), String> {
        let path = self.path()?;
        let parent = path.parent().ok_or("Diretório de armazenamento inválido.")?;
        crate::files::check_ancestors(parent, true)?;
        if !parent.exists() {
            // Only one application-owned directory, not arbitrary recursive creation.
            fs::create_dir(parent).map_err(|_| "Não foi possível criar o diretório privado de armazenamento.".to_owned())?;
        }
        crate::files::check_ancestors(parent, false)?;
        workspace.save(path).map_err(|e| e.to_string())
    }
}

#[derive(Clone, Copy)]
enum LoadState { Pending, Ready(u64), Blocked }
pub struct LoadGuard<S> { store: S, state: LoadState }
impl<S: WorkspaceStore> LoadGuard<S> {
    pub fn new(store: S) -> Self { Self { store, state: LoadState::Pending } }
    fn blocked(message: String) -> BootResult {
        BootResult { workspace: Workspace::default(), storage_ready: false, message: Some(message) }
    }
    pub fn bootstrap(&mut self) -> BootResult {
        if matches!(self.state, LoadState::Blocked) {
            return Self::blocked("Armazenamento bloqueado nesta sessão após erro de integridade. Preserve os arquivos e reinicie somente após revisão; não há restauração automática.".into());
        }
        match self.store.read() {
            Ok(workspace) => {
                let workspace = workspace.unwrap_or_default();
                if let Err(message) = validate_workspace(&workspace) {
                    self.state = LoadState::Blocked;
                    return Self::blocked(message);
                }
                self.state = LoadState::Ready(workspace.revision);
                BootResult { workspace, storage_ready: true, message: None }
            }
            Err(message) => {
                self.state = LoadState::Blocked;
                Self::blocked(format!("Falha na leitura do workspace. Gravações bloqueadas; nenhum arquivo substituído. {message}"))
            }
        }
    }
    pub fn save(&mut self, mut workspace: Workspace) -> Result<Workspace, String> {
        let revision = match self.state {
            LoadState::Pending => return Err("Execute bootstrap antes de gravar.".into()),
            LoadState::Blocked => return Err("Gravação bloqueada por falha anterior na leitura; arquivo preservado.".into()),
            LoadState::Ready(revision) => revision,
        };
        validate_workspace(&workspace)?;
        if workspace.revision != revision {
            return Err("Revisão da interface desatualizada; recarregue antes de gravar.".into());
        }
        let disk = match self.store.read() {
            Ok(Some(disk)) => {
                if let Err(message) = validate_workspace(&disk) {
                    self.state = LoadState::Blocked;
                    return Err(message);
                }
                disk.revision
            }
            Ok(None) => 0,
            Err(message) => {
                self.state = LoadState::Blocked;
                return Err(format!("Armazenamento mudou ou não pode ser lido; gravação bloqueada. {message}"));
            }
        };
        if disk != revision {
            return Err("Workspace alterado por outra instância ou removido; recarregue antes de gravar.".into());
        }
        self.store.write(&mut workspace)?;
        self.state = LoadState::Ready(workspace.revision);
        Ok(workspace)
    }
    pub fn ready(&self) -> Result<(), String> {
        if matches!(self.state, LoadState::Ready(_)) { Ok(()) }
        else { Err("Armazenamento não carregado ou bloqueado; operação recusada.".into()) }
    }
}
pub struct BackendState(pub Arc<Mutex<LoadGuard<NativeStore>>>);
impl Default for BackendState {
    fn default() -> Self {
        #[cfg(debug_assertions)]
        if let Some(profile) = std::env::args().find_map(|s| s.strip_prefix("--verification-profile=").map(str::to_owned)) {
            // Debug-only, fixed application-owned test root. Never accepts an arbitrary path,
            // changes OS settings, weakens encryption or creates a production demo fallback.
            let path = if !profile.is_empty() && profile.len() <= 32 && profile.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
                let base = std::env::temp_dir().join("PreviusRust-native-validation");
                let dir = base.join(profile);
                match crate::files::check_ancestors(&dir, true) {
                    Ok(()) => match std::fs::create_dir_all(&dir) {
                        Ok(()) => crate::files::check_ancestors(&dir, false).map(|_| dir.join("workspace.pvr")),
                        Err(_) => Err("Não foi possível criar o armazenamento sintético de teste.".into()),
                    },
                    Err(e) => Err(e),
                }
            } else { Err("Perfil de validação inválido; nenhuma base foi aberta.".into()) };
            return Self(Arc::new(Mutex::new(LoadGuard::new(NativeStore { path }))));
        }
        Self(Arc::new(Mutex::new(LoadGuard::new(NativeStore::fixed()))))
    }
}
impl BackendState {
    pub fn lock(&self) -> Result<std::sync::MutexGuard<'_, LoadGuard<NativeStore>>, String> {
        self.0.lock().map_err(|_| "Estado privado indisponível; reinicie sem substituir arquivos.".into())
    }
}

/// The external Workspace model uses serde(default). Refuse omitted collections
/// or fields rather than turning a partial invoke payload into a destructive save.
pub fn validate_full_payload(raw: &Value, workspace: &Workspace) -> Result<(), String> {
    let canonical = serde_json::to_value(workspace)
        .map_err(|_| "Workspace não serializável.".to_owned())?;
    if raw != &canonical {
        return Err("Envie o workspace completo recebido de bootstrap, preservando todos os campos e vetores. Campos omitidos ou desconhecidos foram recusados.".into());
    }
    Ok(())
}

pub fn validate_workspace(workspace: &Workspace) -> Result<(), String> {
    let structural = workspace.validate();
    if !structural.is_empty() { return Err(format!("Workspace estruturalmente inválido: {}", structural.join(" "))); }
    // JSON numbers travelling through JavaScript must be exactly representable.
    const MAX_JS_ID: u64 = 9_007_199_254_740_991;
    let ids = workspace.clients.iter().map(|v| v.id)
        .chain(workspace.calculations.iter().map(|v| v.id))
        .chain(workspace.reference_tables.iter().map(|v| v.id))
        .chain(workspace.documents.iter().map(|v| v.id))
        .chain(workspace.appointments.iter().map(|v| v.id))
        .chain(workspace.calculations.iter().flat_map(|v| v.manual_links.iter().map(|v| v.id)))
        .chain(workspace.calculations.iter().flat_map(|v| v.manual_salaries.iter().map(|v| v.id)));
    let max_id = ids.max().unwrap_or(0);
    if max_id >= MAX_JS_ID || workspace.next_id <= max_id || workspace.next_id > MAX_JS_ID
        || workspace.revision >= MAX_JS_ID {
        return Err("IDs, próximo ID ou revisão fora do intervalo seguro da interface.".into());
    }
    if workspace.clients.len() > 100_000 || workspace.calculations.len() > 100_000
        || workspace.reference_tables.len() > 10_000 || workspace.documents.len() > 100_000
        || workspace.appointments.len() > 100_000 {
        return Err("Quantidade de entidades excede o limite local.".into());
    }
    for calculation in &workspace.calculations { validate_calculation(calculation)?; }
    // Bounded streaming serialization avoids allocating a second cleartext buffer.
    struct SizeLimit(u64);
    impl std::io::Write for SizeLimit {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self.0.saturating_add(bytes.len() as u64);
            if self.0 > MAX_WORKSPACE_BYTES { return Err(std::io::Error::other("size_limit")); }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
    }
    serde_json::to_writer(SizeLimit(0), workspace)
        .map_err(|_| "Workspace excede o limite de 64 MiB ou não é serializável.".into())
}

pub fn validate_calculation(calculation: &Calculation) -> Result<(), String> {
    if calculation.options.rule_policy.len()>2*1024*1024{return Err("Política/snapshot excede o limite de 2 MiB.".into());}
    if calculation.options.rule_policy.trim().starts_with('{'){
        let policy:previus_rgps::contracts::CasePolicy=serde_json::from_str(&calculation.options.rule_policy).map_err(|_|"Política de cálculo inválida; não foi salva nem calculada.")?;
        if let Some(pcd)=policy.lc142{if let Some(snapshot)=pcd.snapshot{snapshot.validate()?;}}
    }
    previus_workspace::link_review::validate_source_links(calculation)?;
    if let Some(review) = &calculation.contribution_review { review.validate(calculation)?; }
    if let Some(assistant) = &calculation.assistant { assistant.validate()?; }
    if calculation.manual_links.len() > 100_000 || calculation.manual_salaries.len() > 100_000 {
        return Err("Cálculo excede o limite de registros manuais.".into());
    }
    if let Some(cnis) = &calculation.cnis {
        if cnis.salary_entries.len() > 1_000_000 || cnis.links.len() > 100_000
            || cnis.metadata.page_count > 500 || cnis.metadata.parsed_pages > cnis.metadata.page_count {
            return Err("CNIS excede os limites de importação ou possui metadados inconsistentes.".into());
        }
        let mut sequences = std::collections::HashSet::new();
        for link in &cnis.links {
            if link.sequence == 0 || !sequences.insert(link.sequence) {
                return Err("CNIS contém sequência de vínculo inválida ou duplicada.".into());
            }
        }
        let mut referenced = vec![false; cnis.salary_entries.len()];
        for (competence, month) in &cnis.salary_months {
            if month.count != month.entry_indices.len() || month.count == 0 {
                return Err("CNIS contém agrupamento de competências inconsistente.".into());
            }
            let mut sum = 0i64;
            for &source_index in &month.entry_indices {
                // Importer grouping stores ONE-BASED source indices, not Vec offsets.
                let index = source_index.checked_sub(1).ok_or("CNIS contém índice de origem zero.")?;
                let entry = cnis.salary_entries.get(index)
                    .ok_or("CNIS contém referência a remuneração inexistente.")?;
                if entry.source.index != source_index || entry.competence != *competence || referenced[index] {
                    return Err("CNIS contém referência duplicada ou competência divergente.".into());
                }
                referenced[index] = true;
                sum = sum.checked_add(entry.amount_centavos).ok_or("Soma nominal do CNIS excedeu o limite.")?;
            }
            if sum != month.amount_sum_centavos {
                return Err("Soma nominal do CNIS diverge dos registros de origem.".into());
            }
        }
        if referenced.iter().any(|found| !found)
            || cnis.salary_entries.iter().any(|entry| entry.link_sequence.is_some_and(|id| !sequences.contains(&id))) {
            return Err("CNIS contém remunerações ausentes do agrupamento ou referências inválidas.".into());
        }
    }
    Ok(())
}

/// Execution gate, separate from draft validation: saving never completes an
/// interview or approves a rule. Legacy/manual flows keep the engine's own gates.
pub fn validate_assistant_for_calculation(calculation: &Calculation) -> Result<(), String> {
    let Some(assistant) = &calculation.assistant else { return Ok(()); };
    assistant.validate()?;
    if assistant.mode != "assisted" { return Ok(()); }
    if assistant.current_benefit_received.is_none() || assistant.process_kind.is_none()
        || assistant.goal.is_none() || !assistant.review_confirmed {
        return Err("Complete as perguntas sobre benefício atual, tipo de processo e finalidade; confirme a revisão do assistente antes de calcular.".into());
    }
    if assistant.current_benefit_received == Some(true) {
        return Err("Benefício atual informado: revisão ou restabelecimento não são suportados por este motor de novo benefício. Nenhuma RMI foi calculada.".into());
    }
    if assistant.goal.as_deref() != Some("Calcular um novo benefício") {
        return Err("Este motor suporta somente calcular um novo benefício; cálculo exclusivo de tempo e restabelecimento ainda não são suportados. Nenhuma RMI foi calculada.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use previus_workspace::{Client, ManualSalary};
    use std::cell::{Cell, RefCell};
    struct FakeStore {
        disk: RefCell<Option<Workspace>>, corrupt: Cell<bool>, writes: Cell<usize>, fail_write: Cell<bool>,
    }
    impl FakeStore {
        fn empty() -> Self { Self { disk: RefCell::new(None), corrupt: Cell::new(false), writes: Cell::new(0), fail_write: Cell::new(false) } }
    }
    impl WorkspaceStore for FakeStore {
        fn read(&self) -> Result<Option<Workspace>, String> {
            if self.corrupt.get() { Err("synthetic_corruption".into()) } else { Ok(self.disk.borrow().clone()) }
        }
        fn write(&self, workspace: &mut Workspace) -> Result<(), String> {
            if self.fail_write.get() { return Err("synthetic_write_failure".into()); }
            workspace.revision += 1;
            self.writes.set(self.writes.get() + 1);
            self.disk.replace(Some(workspace.clone())); Ok(())
        }
    }
    #[test]
    fn missing_boot_returns_default_without_write() {
        let mut guard = LoadGuard::new(FakeStore::empty());
        let boot = guard.bootstrap(); assert!(boot.storage_ready); assert_eq!(boot.workspace.revision, 0);
        assert!(boot.message.is_none()); assert_eq!(guard.store.writes.get(), 0);
    }
    #[test]
    fn save_before_boot_is_refused() {
        let mut guard = LoadGuard::new(FakeStore::empty());
        assert!(guard.save(Workspace::default()).is_err()); assert_eq!(guard.store.writes.get(), 0);
    }
    #[test]
    fn corrupt_boot_is_sticky_and_never_saves() {
        let store = FakeStore::empty(); store.corrupt.set(true);
        let mut guard = LoadGuard::new(store); let boot = guard.bootstrap();
        assert!(!boot.storage_ready); assert!(boot.message.is_some()); assert!(boot.workspace.clients.is_empty());
        guard.store.corrupt.set(false); assert!(!guard.bootstrap().storage_ready);
        assert!(guard.save(Workspace::default()).is_err()); assert_eq!(guard.store.writes.get(), 0);
    }
    #[test]
    fn invalid_loaded_model_blocks_saves() {
        let store = FakeStore::empty(); let mut w = Workspace::default(); w.clients.push(Client::default());
        store.disk.replace(Some(w)); let mut guard = LoadGuard::new(store);
        assert!(!guard.bootstrap().storage_ready); assert!(guard.save(Workspace::default()).is_err());
    }
    #[test]
    fn successful_save_returns_incremented_revision() {
        let mut guard = LoadGuard::new(FakeStore::empty()); let boot = guard.bootstrap();
        let saved = guard.save(boot.workspace).unwrap(); assert_eq!(saved.revision, 1);
        assert_eq!(guard.save(saved).unwrap().revision, 2); assert_eq!(guard.store.writes.get(), 2);
    }
    #[test]
    fn stale_frontend_and_external_revision_are_refused() {
        let mut guard = LoadGuard::new(FakeStore::empty()); let boot = guard.bootstrap();
        let saved = guard.save(boot.workspace.clone()).unwrap(); assert!(guard.save(boot.workspace).is_err());
        let mut changed = saved.clone(); changed.revision = 2; guard.store.disk.replace(Some(changed));
        assert!(guard.save(saved).is_err()); assert_eq!(guard.store.writes.get(), 1);
    }
    #[test]
    fn corrupt_between_boot_and_save_blocks_session() {
        let mut guard = LoadGuard::new(FakeStore::empty()); let boot = guard.bootstrap();
        guard.store.corrupt.set(true); assert!(guard.save(boot.workspace).is_err());
        guard.store.corrupt.set(false); assert!(!guard.bootstrap().storage_ready); assert_eq!(guard.store.writes.get(), 0);
    }
    #[test]
    fn failed_save_does_not_advance_guard_revision() {
        let mut guard = LoadGuard::new(FakeStore::empty()); let boot = guard.bootstrap();
        guard.store.fail_write.set(true); assert!(guard.save(boot.workspace.clone()).is_err());
        guard.store.fail_write.set(false); assert_eq!(guard.save(boot.workspace).unwrap().revision, 1);
    }
    #[test]
    fn duplicate_global_ids_and_dangling_clients_rejected() {
        let mut w = Workspace::default(); w.clients.push(Client { id: 1, ..Default::default() }); w.next_id = 2;
        w.calculations.push(Calculation { id: 1, ..Default::default() }); assert!(validate_workspace(&w).is_err());
        w.calculations[0].id = 2; w.next_id = 3; w.calculations[0].client_id = Some(999);
        assert!(validate_workspace(&w).is_err());
    }
    #[test]
    fn manual_ids_and_js_safe_range_rejected() {
        let mut w = Workspace::default(); w.calculations.push(Calculation { id: 1, manual_salaries: vec![ManualSalary { id: 1, ..Default::default() }], ..Default::default() });
        w.next_id = 2; assert!(validate_workspace(&w).is_err());
        w.calculations.clear(); w.next_id = u64::MAX; assert!(validate_workspace(&w).is_err());
    }
    #[test]
    fn complete_roundtrip_required_including_nested_fields() {
        let mut w = Workspace::default(); w.clients.push(Client { id: 1, ..Default::default() }); w.next_id = 2;
        let full = serde_json::to_value(&w).unwrap(); assert!(validate_full_payload(&full, &w).is_ok());
        let mut partial = full.clone(); partial.as_object_mut().unwrap().remove("calculations");
        let decoded: Workspace = serde_json::from_value(partial.clone()).unwrap(); assert!(validate_full_payload(&partial, &decoded).is_err());
        let mut nested = full.clone(); nested["clients"][0].as_object_mut().unwrap().remove("notes");
        let decoded: Workspace = serde_json::from_value(nested.clone()).unwrap(); assert!(validate_full_payload(&nested, &decoded).is_err());
        let mut unknown = full; unknown["unexpected"] = Value::Bool(true); assert!(validate_full_payload(&unknown, &w).is_err());
    }
    #[test]
    fn incomplete_draft_can_save_but_not_claim_legal_validation() {
        let mut guard = LoadGuard::new(FakeStore::empty()); let mut w = guard.bootstrap().workspace;
        let id = w.id(); w.calculations.push(Calculation { id, name: "Synthetic draft".into(), ..Default::default() });
        assert!(guard.save(w).is_ok());
    }
    #[cfg(windows)]
    #[test]
    fn native_guard_uses_only_synthetic_ciphertext_and_preserves_corruption() {
        let parent = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join(format!("synthetic-guard-{}", std::process::id()));
        fs::create_dir_all(&parent).unwrap();
        let path = parent.join("workspace.pvr");
        assert!(!path.exists());
        let mut guard = LoadGuard::new(NativeStore { path: Ok(path.clone()) });
        let workspace = guard.bootstrap().workspace; assert!(!path.exists());
        let saved = guard.save(workspace).unwrap(); assert_eq!(saved.revision, 1);
        assert!(fs::read(&path).unwrap().starts_with(b"PVRS2"));
        fs::write(&path, b"synthetic-corrupt-envelope").unwrap();
        assert!(guard.save(saved).is_err()); assert!(!guard.bootstrap().storage_ready);
        assert_eq!(fs::read(&path).unwrap(), b"synthetic-corrupt-envelope");
        fs::remove_dir_all(parent).unwrap();
    }

    #[test]
    fn synthetic_import_preserves_one_based_groups_and_rejects_zero_or_duplicates() {
        let parsed = previus_cnis::import_text(include_str!("../../../../crates/previus-cnis/tests/fixtures/synthetic.cnis")).unwrap();
        let case = Calculation { cnis: Some(parsed.clone()), ..Default::default() };
        validate_calculation(&case).unwrap();
        let mut bad = case.clone();
        bad.cnis.as_mut().unwrap().salary_months.values_mut().next().unwrap().entry_indices[0] = 0;
        assert!(validate_calculation(&bad).is_err());
        let mut bad = case.clone();
        let cnis = bad.cnis.as_mut().unwrap();
        cnis.salary_months.values_mut().next().unwrap().amount_sum_centavos += 1;
        assert!(validate_calculation(&bad).is_err());
        assert_eq!(parsed.salary_entries.len(), 6);
        assert_eq!(parsed.links.iter().filter(|v| v.prpps).count(), 1);
    }

    fn assistant_case() -> Calculation {
        Calculation { assistant: Some(previus_workspace::AssistantInterview {
            mode: "assisted".into(), current_benefit_received: Some(false), import_requested: Some(true),
            process_kind: Some("Simulação do benefício".into()), goal: Some("Calcular um novo benefício".into()),
            review_confirmed: true, completed_steps: vec!["intro".into(), "benefit".into(), "review".into()],
        }), ..Default::default() }
    }
    #[test]
    fn assisted_draft_saves_unanswered_without_implicit_completion_or_guessed_parameters() {
        let mut guard = LoadGuard::new(FakeStore::empty()); let mut workspace = guard.bootstrap().workspace;
        let id = workspace.id(); workspace.calculations.push(Calculation { id,
            assistant: Some(previus_workspace::AssistantInterview { mode: "assisted".into(), ..Default::default() }),
            ..Default::default() });
        let saved = guard.save(workspace).unwrap(); let case = &saved.calculations[0];
        let assistant = case.assistant.as_ref().unwrap();
        assert!(assistant.current_benefit_received.is_none()); assert!(assistant.import_requested.is_none());
        assert!(assistant.process_kind.is_none()); assert!(assistant.goal.is_none());
        assert!(!assistant.review_confirmed); assert!(assistant.completed_steps.is_empty());
        assert!(case.sex.is_none()); assert!(case.disability.is_none()); assert!(case.result.is_none());
        assert!(validate_assistant_for_calculation(case).is_err());
    }
    #[test]
    fn assistant_null_is_required_in_canonical_save_payload_but_old_bootstrap_migrates_in_memory() {
        let mut workspace = Workspace::default(); let id = workspace.id();
        workspace.calculations.push(Calculation { id, ..Default::default() });
        let canonical = serde_json::to_value(&workspace).unwrap();
        assert!(canonical["calculations"][0].as_object().unwrap().contains_key("assistant"));
        assert!(canonical["calculations"][0]["assistant"].is_null());
        validate_full_payload(&canonical, &workspace).unwrap();
        let mut old = canonical.clone(); old["calculations"][0].as_object_mut().unwrap().remove("assistant");
        let loaded: Workspace = serde_json::from_value(old.clone()).unwrap();
        assert!(loaded.calculations[0].assistant.is_none()); assert!(validate_full_payload(&old, &loaded).is_err());
        validate_full_payload(&serde_json::to_value(&loaded).unwrap(), &loaded).unwrap();
    }
    #[test]
    fn assistant_full_payload_requires_all_nested_answers_even_when_defaults_are_safe() {
        let mut workspace = Workspace::default(); let id = workspace.id();
        workspace.calculations.push(Calculation { id, ..assistant_case() });
        let canonical = serde_json::to_value(&workspace).unwrap(); validate_full_payload(&canonical, &workspace).unwrap();
        for field in ["mode", "current_benefit_received", "import_requested", "process_kind", "goal", "review_confirmed", "completed_steps"] {
            let mut partial = canonical.clone(); partial["calculations"][0]["assistant"].as_object_mut().unwrap().remove(field);
            let decoded: Workspace = serde_json::from_value(partial.clone()).unwrap();
            assert!(validate_full_payload(&partial, &decoded).is_err());
        }
    }
    #[test]
    fn assistant_false_answers_persist_as_false_not_null() {
        let mut guard = LoadGuard::new(FakeStore::empty()); let mut workspace = guard.bootstrap().workspace;
        let id = workspace.id(); let mut case = assistant_case(); case.id = id;
        case.assistant.as_mut().unwrap().import_requested = Some(false); workspace.calculations.push(case);
        let saved = guard.save(workspace).unwrap(); let value = serde_json::to_value(saved).unwrap();
        assert_eq!(value["calculations"][0]["assistant"]["current_benefit_received"], serde_json::json!(false));
        assert_eq!(value["calculations"][0]["assistant"]["import_requested"], serde_json::json!(false));
    }
    #[test]
    fn structural_validator_accepts_every_closed_process_and_goal_as_draft() {
        for process in ["Simulação do benefício", "Revisão e valor da causa", "Planejamento futuro",
            "Processo administrativo", "Execução judicial", "Contribuições em atraso"] {
            for goal in ["Calcular um novo benefício", "Calcular somente o tempo de contribuição", "Restabelecer um benefício"] {
                let mut case = assistant_case(); let assistant = case.assistant.as_mut().unwrap();
                assistant.process_kind = Some(process.into()); assistant.goal = Some(goal.into());
                assistant.review_confirmed = false; validate_calculation(&case).unwrap();
            }
        }
    }
    #[test]
    fn structural_validator_rejects_unknown_metadata_and_sticky_loaded_corruption() {
        let mut case = assistant_case(); case.assistant.as_mut().unwrap().completed_steps.push("intro".into());
        assert!(validate_calculation(&case).is_err());
        let store = FakeStore::empty(); let mut workspace = Workspace::default(); let id = workspace.id(); case.id = id;
        workspace.calculations.push(case); store.disk.replace(Some(workspace)); let mut guard = LoadGuard::new(store);
        assert!(!guard.bootstrap().storage_ready); assert!(guard.save(Workspace::default()).is_err());
        assert_eq!(guard.store.writes.get(), 0);
    }
    #[test]
    fn execution_gate_requires_each_answer_and_explicit_review_not_step_ids() {
        let base = assistant_case(); validate_assistant_for_calculation(&base).unwrap();
        for missing in 0..4 {
            let mut case = base.clone(); let assistant = case.assistant.as_mut().unwrap();
            assistant.completed_steps = vec!["intro", "benefit", "import", "process", "goal", "parameters", "periods", "salaries", "policy", "review", "result"].into_iter().map(str::to_owned).collect();
            match missing { 0 => assistant.current_benefit_received = None, 1 => assistant.process_kind = None,
                2 => assistant.goal = None, _ => assistant.review_confirmed = false }
            validate_calculation(&case).unwrap(); assert!(validate_assistant_for_calculation(&case).unwrap_err().contains("Complete"));
        }
    }
    #[test]
    fn execution_gate_refuses_current_benefit_and_unsupported_goals_without_private_text() {
        let mut case = assistant_case(); case.notes = "SYNTHETIC PRIVATE MARKER".into();
        case.assistant.as_mut().unwrap().current_benefit_received = Some(true);
        let error = validate_assistant_for_calculation(&case).unwrap_err();
        assert!(error.contains("Benefício atual informado")); assert!(!error.contains("SYNTHETIC PRIVATE MARKER"));
        case.assistant.as_mut().unwrap().current_benefit_received = Some(false);
        for goal in ["Calcular somente o tempo de contribuição", "Restabelecer um benefício"] {
            case.assistant.as_mut().unwrap().goal = Some(goal.into()); validate_calculation(&case).unwrap();
            let error = validate_assistant_for_calculation(&case).unwrap_err(); assert!(error.contains("somente calcular um novo benefício"));
            assert!(!error.contains("SYNTHETIC PRIVATE MARKER"));
        }
    }
    #[test]
    fn manual_and_legacy_paths_keep_existing_engine_review_gates() {
        validate_assistant_for_calculation(&Calculation::default()).unwrap();
        let case = Calculation { assistant: Some(previus_workspace::AssistantInterview { mode: "manual".into(), ..Default::default() }), ..Default::default() };
        validate_calculation(&case).unwrap(); validate_assistant_for_calculation(&case).unwrap();
        let result = tauri::async_runtime::block_on(crate::calculate_case(case)).unwrap();
        assert_eq!(result["status"], "blocked"); assert!(result["data"].is_null()); assert_eq!(result["legal_validation_performed"], false);
    }
    #[test]
    fn actual_async_case_command_rejects_incomplete_assistant_before_rgps() {
        let mut case = assistant_case(); case.assistant.as_mut().unwrap().review_confirmed = false;
        assert!(tauri::async_runtime::block_on(crate::calculate_case(case)).unwrap_err().contains("Complete"));
    }
    #[test]
    fn actual_async_case_command_cannot_calculate_new_rmi_for_existing_benefit() {
        let mut case = assistant_case(); case.assistant.as_mut().unwrap().current_benefit_received = Some(true);
        assert!(tauri::async_runtime::block_on(crate::calculate_case(case)).unwrap_err().contains("Benefício atual informado"));
    }
    #[test]
    fn actual_async_case_command_accepts_completed_synthetic_assistant_without_legal_claim() {
        let mut case: Calculation = serde_json::from_str(include_str!("../../../../crates/previus-rgps/tests/fixtures/calculation-synthetic.json")).unwrap();
        case.assistant = assistant_case().assistant;
        let result = tauri::async_runtime::block_on(crate::calculate_case(case)).unwrap();
        assert_eq!(result["status"], "experimental"); assert_eq!(result["legal_validation_performed"], false);
        assert_eq!(result["data"]["native_result"]["resumo"]["rmi"], "1600.00");
        assert!(!result["warnings"].as_array().unwrap().is_empty());
    }

}
