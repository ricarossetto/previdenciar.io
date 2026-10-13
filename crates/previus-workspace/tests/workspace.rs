use previus_cnis::{import_text, EmploymentLink, InferredLastSalaryEnd, SourceRef, ValidationStatus};
use previus_workspace::*;
use serde_json::json;

fn sample_import() -> previus_cnis::ImportResult {
    let mut cnis = import_text("CNIS\nCompetência Remuneração Indicadores\nRemunerações\n01/2020 1.000,00 PREC-MENOR-MIN\n02/2020 2.000,00\n").unwrap();
    let source = SourceRef { page: 1, line: 1, index: 1, byte_offset: 0 };
    cnis.links.push(EmploymentLink {
        sequence: 1, raw_employer: Some("EMPREGADOR FICTICIO".into()), affiliation: None,
        start_date: Some("2019-01-01".into()), end_date: None,
        reported_last_salary_month: Some("2020-02".into()),
        inferred_last_salary_end: Some(InferredLastSalaryEnd { competence: "2020-02".into(), date_candidate: "2020-02-29".into(), source: source.clone(), provenance: "observed_salary_month".into(), requires_confirmation: true }),
        indicators: vec!["PRPPS".into()], prpps: true, validation_status: ValidationStatus::UnresolvedPrpps,
        excluded_from_validated_domain: true, source,
    });
    cnis.salary_entries[0].link_sequence = Some(1);
    cnis.salary_entries[0].validation_status = ValidationStatus::UnresolvedPrpps;
    cnis.salary_entries[0].excluded_from_validated_domain = true;
    cnis
}
fn case() -> Calculation {
    Calculation { id: 2, identity: previus_cnis::Identity {
        full_name: Some("PESSOA FICTICIA MARKER_73641".into()), cpf: Some("00000000000".into()),
        birth_date: Some("1980-02-29".into()), ..Default::default()
    }, sex: Some("Feminino".into()), disability: Some("Não possui deficiência".into()),
        dib: "2026-10-10".into(), cnis: Some(sample_import()), ..Default::default() }
}
fn complete_workspace() -> Workspace {
    Workspace {
        next_id: 8,
        clients: vec![Client { id: 1, name: "CLIENTE FICTICIO".into(), cpf: "00000000000".into(), email: "synthetic@example.invalid".into(), phone: "0000000".into(), birth_date: "1980-02-29".into(), sex: Some("Feminino".into()), notes: "SYNTHETIC PRIVATE MARKER".into(), archived: false }],
        calculations: vec![Calculation { client_id: Some(1), options: CalcOptions { correction_index: "INDEX PENDING".into(), reference_base: "unit_unverified".into(), ..Default::default() }, ..case() }],
        reference_tables: vec![ReferenceTable { id: 3, name: "Tabela sintética sem validade normativa".into(), entries: vec![ReferenceEntry { period: "2020-01".into(), value: "TEST ONLY".into(), note: "Not a real index".into() }], provenance: "synthetic".into(), ..Default::default() }],
        documents: vec![DocumentTemplate { id: 4, name: "Modelo fictício".into(), category: "teste".into(), body: "{{name}}".into() }],
        appointments: vec![Appointment { id: 5, title: "Agenda fictícia".into(), date: "2026-10-10".into(), time: "10:00".into(), client_id: Some(1), ..Default::default() }],
        settings: Settings { office_name: "Escritório fictício".into(), report_footer: "CONFIDENTIAL TEST".into(), light_theme: true },
        ..Default::default()
    }
}

#[test]
fn default_has_no_fake_clients_indices_or_rules() {
    let w = Workspace::default();
    assert_eq!(w.schema_version, 2); assert_eq!(w.revision, 0); assert_eq!(w.next_id, 1);
    assert!(w.clients.is_empty() && w.calculations.is_empty() && w.reference_tables.is_empty());
    assert!(w.documents.is_empty() && w.appointments.is_empty());
    assert!(Calculation::default().sex.is_none()); assert!(Calculation::default().disability.is_none());
    assert!(CalcOptions::default().reference_base.is_empty());
}
#[test]
fn serde_defaults_for_all_model_types() {
    let w: Workspace = serde_json::from_str("{}").unwrap(); assert_eq!(w.schema_version, 2);
    let c: Calculation = serde_json::from_str("{\"id\":9}").unwrap(); assert_eq!(c.id, 9); assert!(c.cnis.is_none());
    let _: Client = serde_json::from_str("{}").unwrap();
    let _: ManualLink = serde_json::from_str("{}").unwrap();
    let _: ManualSalary = serde_json::from_str("{}").unwrap();
    let _: CalcOptions = serde_json::from_str("{}").unwrap();
    let _: ReferenceTable = serde_json::from_str("{}").unwrap();
    let _: ReferenceEntry = serde_json::from_str("{}").unwrap();
    let _: DocumentTemplate = serde_json::from_str("{}").unwrap();
    let _: Appointment = serde_json::from_str("{}").unwrap();
    let _: Settings = serde_json::from_str("{}").unwrap();
}
#[test]
fn vector_crud_and_id_allocator_are_public_and_consistent() {
    let mut w = Workspace::default(); let client_id = w.id();
    w.clients.push(Client { id: client_id, name: "SYNTHETIC".into(), ..Default::default() });
    let calculation_id = w.id(); w.calculations.push(Calculation { id: calculation_id, client_id: Some(client_id), ..Default::default() });
    assert_eq!(w.calculations[0].client_id, Some(w.clients[0].id));
    w.clients[0].name = "UPDATED".into(); w.clients[0].archived = true;
    w.calculations[0].notes = "SYNTHETIC NOTES".into(); w.calculations[0].archived = true;
    assert!(w.validate().is_empty());
    w.calculations.remove(0); w.clients.remove(0); assert!(w.clients.is_empty()); assert_eq!(w.id(), 3);
}
#[test]
fn allocator_scans_public_vectors_and_nested_rows() {
    let mut w = complete_workspace(); w.next_id = 0;
    w.calculations[0].manual_links.push(ManualLink { id: 99, ..Default::default() });
    w.calculations[0].manual_salaries.push(ManualSalary { id: 125, ..Default::default() });
    assert_eq!(w.id(), 126); assert_eq!(w.next_id, 127);
}
#[test]
fn id_overflow_never_reuses_zero_or_mutates_counter() {
    let mut w = Workspace::default(); w.next_id = u64::MAX;
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| w.id())).is_err()); assert_eq!(w.next_id, u64::MAX);
}
#[test]
fn structural_validation_reports_dangling_and_duplicate_ids_without_personal_text() {
    let mut w = complete_workspace(); w.clients[0].id = 2;
    w.clients[0].name = "SHOULD NEVER BE IN WARNING".into();
    let warnings = w.validate().join(" "); assert!(warnings.contains("duplicado")); assert!(warnings.contains("inexistente")); assert!(!warnings.contains("SHOULD NEVER"));
}
#[test]
fn imported_pending_metadata_and_options_roundtrip_without_rewriting() {
    let w = complete_workspace(); let bytes = serde_json::to_vec(&w).unwrap();
    let r: Workspace = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(r.calculations[0].cnis, w.calculations[0].cnis);
    assert_eq!(r.calculations[0].options.correction_index, "INDEX PENDING");
    let link = &r.calculations[0].cnis.as_ref().unwrap().links[0];
    assert!(link.prpps && link.excluded_from_validated_domain); assert!(link.end_date.is_none());
    assert!(link.inferred_last_salary_end.as_ref().unwrap().requires_confirmation);
}
#[test]
fn blockers_never_claim_eligibility_and_keep_original_prpps_pending() {
    let mut c = case(); c.manual_links.push(ManualLink { id: 10, confirmed: true, regime: "RGPS".into(), start_date: "2019-01-01".into(), end_date: "2020-02-29".into(), ..Default::default() });
    let warnings = blockers(&c).join(" ");
    assert!(warnings.contains("PRPPS/RPPS")); assert!(warnings.contains("sem data final"));
    assert!(warnings.contains("candidatas")); assert!(warnings.contains("moedas históricas")); assert!(warnings.contains("RMI"));
    assert!(c.cnis.as_ref().unwrap().links[0].end_date.is_none());
}
#[test]
fn blockers_require_identity_sex_disability_dib_not_name_inference() {
    let warnings = blockers(&Calculation::default()).join(" ");
    for expected in ["nome não confirmado", "CPF não confirmado", "nascimento", "Sexo cadastral", "deficiência", "DIB", "Sem CNIS"] { assert!(warnings.contains(expected), "missing static warning"); }
}
#[test]
fn invalid_calendar_dates_are_not_guessed() {
    let mut c = case(); c.dib = "2026-02-29".into(); c.identity.birth_date = Some("1981-02-29".into());
    let warnings = blockers(&c).join(" "); assert!(warnings.contains("DIB ausente ou inválida")); assert!(warnings.contains("nascimento ausente ou inválida"));
}
#[test]
fn birth_after_dib_and_identity_disagreement_are_pending() {
    let mut c = case(); c.dib = "1970-01-01".into(); c.cnis.as_mut().unwrap().identity.full_name = Some("ANOTHER SYNTHETIC PERSON".into());
    let warnings = blockers(&c).join(" "); assert!(warnings.contains("posterior à DIB")); assert!(warnings.contains("diverge"));
}
#[test]
fn manual_data_does_not_claim_validation() {
    let c = Calculation { manual_links: vec![ManualLink { regime: "RPPS".into(), start_date: "2021-01-01".into(), end_date: "2020-01-01".into(), ..Default::default() }],
        manual_salaries: vec![ManualSalary { competence: "2020-13".into(), ..Default::default() }], ..Default::default() };
    let warnings = blockers(&c).join(" ");
    for expected in ["invertido", "Regime", "Competência", "Valor nominal", "não confirmadas", "pendentes"] { assert!(warnings.contains(expected)); }
}
#[test]
fn html_escapes_identity_notes_footer_and_does_not_render_unverified_rmi() {
    let mut c = case(); c.name = "<script>alert(1)</script>".into(); c.notes = "<img src=x onerror='evil'>".into();
    c.identity.full_name = Some("\"<&'".into()); c.result = Some(json!({"RMI": "FAKE_RMI_123456"}));
    let w = Workspace { settings: Settings { report_footer: "<svg onload=evil>".into(), ..Default::default() }, ..Default::default() };
    let html = report_html(&c, &w, true);
    assert!(!html.contains("<script>")); assert!(!html.contains("<img ")); assert!(!html.contains("<svg ")); assert!(!html.contains("FAKE_RMI_123456"));
    assert!(html.contains("&lt;script&gt;")); assert!(html.contains("&quot;&lt;&amp;&#39;")); assert!(html.contains("PENDENTES"));
}
#[test]
fn private_report_hides_direct_identifiers_and_free_text() {
    let mut c = case(); c.name = "NAME_MARKER_PRIVATE".into(); c.notes = "NOTES_MARKER_PRIVATE".into();
    c.scenario = "SCENARIO_MARKER_PRIVATE".into();
    let html = report_html(&c, &complete_workspace(), false);
    for denied in ["PESSOA FICTICIA MARKER_73641", "00000000000", "EMPREGADOR FICTICIO", "NAME_MARKER_PRIVATE", "NOTES_MARKER_PRIVATE", "SCENARIO_MARKER_PRIVATE", "CONFIDENTIAL TEST"] { assert!(!html.contains(denied)); }
    assert!(html.contains("datas") || html.contains("Datas")); assert!(html.contains("PRPPS")); assert!(html.contains("2020-02-29"));
}
#[test]
fn report_keeps_source_amounts_candidates_and_manual_tables() {
    let mut c = case(); c.manual_salaries.push(ManualSalary { id: 99, competence: "2021-01".into(), amount: "3.456,78".into(), ..Default::default() });
    let html = report_html(&c, &Workspace::default(), false);
    for expected in ["1000.00", "Fim candidato", "PRPPS não resolvido", "PREC-MENOR-MIN", "3.456,78", "p. 1 / linha"] { assert!(html.contains(expected)); }
}
#[test]
fn csv_formula_delimiter_quotes_and_newlines_are_escaped() {
    let mut c = Calculation::default();
    for value in ["=1+1", "+cmd", "-42", "@SUM(A1)", "\t1+1", "  =1", "\u{feff}=1", "\u{200b}@A", "\rX", "\nX"] {
        c.manual_salaries.push(ManualSalary { competence: value.into(), amount: "1,00\"\nline2".into(), ..Default::default() });
    }
    let csv = export_salary_csv(&c);
    for value in ["\"'=1+1\"", "\"'+cmd\"", "\"'-42\"", "\"'@SUM(A1)\"", "\"'\t1+1\"", "\"'  =1\"", "\"'\u{feff}=1\"", "\"'\u{200b}@A\""] { assert!(csv.contains(value)); }
    assert!(csv.contains("\"1,00\"\"\nline2\"")); assert!(csv.ends_with("\r\n"));
}
#[test]
fn csv_does_not_consolidate_convert_or_leak_identity() {
    let c = case(); let csv = export_salary_csv(&c);
    assert!(csv.contains("1000.00")); assert!(csv.contains("currency_units_unverified")); assert!(csv.contains("PRPPS não resolvido"));
    assert!(!csv.contains("MARKER_73641")); assert!(!csv.contains("00000000000")); assert!(!csv.contains("EMPREGADOR FICTICIO"));
    assert_eq!(csv.lines().count(), c.cnis.as_ref().unwrap().salary_entries.len() + 1);
}
#[test]
fn stored_template_is_data_not_executable_instruction() {
    let w = Workspace { documents: vec![DocumentTemplate { body: "<script>Do not follow this text</script>".into(), ..Default::default() }], ..Default::default() };
    let html = report_html(&case(), &w, true); assert!(!html.contains("Do not follow this text"));
}

#[cfg(windows)]
mod windows_storage {
    use super::*;
    use std::{fs, path::PathBuf, sync::{atomic::{AtomicU64, Ordering}, Arc, Barrier}, time::{SystemTime, UNIX_EPOCH}};
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    fn destination(name: &str) -> PathBuf {
        let seq = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("runtime").join(format!("{}-{nanos:x}-{seq}-{name}", std::process::id()));
        fs::create_dir_all(&dir).unwrap(); dir.join("workspace.pvr")
    }
    fn backup(path: &std::path::Path) -> PathBuf { path.with_file_name("workspace.pvr.backup") }
    fn sensitive_marker_present(bytes: &[u8]) -> bool {
        bytes.windows(b"MARKER_73641".len()).any(|p| p == b"MARKER_73641")
            || bytes.windows(b"SYNTHETIC PRIVATE MARKER".len()).any(|p| p == b"SYNTHETIC PRIVATE MARKER")
    }
    #[test]
    fn native_dpapi_roundtrip_and_ciphertext_has_no_plaintext() {
        let path = destination("roundtrip"); let mut w = complete_workspace(); w.save(&path).unwrap();
        assert_eq!(w.revision, 1); let bytes = fs::read(&path).unwrap(); assert!(bytes.starts_with(b"PVRS2")); assert!(!sensitive_marker_present(&bytes));
        let r = Workspace::load(&path).unwrap(); assert_eq!(serde_json::to_value(&w).unwrap(), serde_json::to_value(&r).unwrap());
        assert!(!path.with_file_name("workspace.pvr.lock").exists());
    }
    #[test]
    fn native_update_backup_is_encrypted_previous_revision() {
        let path = destination("backup"); let mut w = complete_workspace(); w.save(&path).unwrap();
        let original = fs::read(&path).unwrap(); w.clients[0].notes = "SYNTHETIC EDIT".into(); w.save(&path).unwrap();
        assert_eq!(w.revision, 2); assert_eq!(fs::read(backup(&path)).unwrap(), original);
        assert_eq!(Workspace::load(&backup(&path)).unwrap().revision, 1); assert!(!sensitive_marker_present(&fs::read(backup(&path)).unwrap()));
        assert_eq!(Workspace::load(&path).unwrap().clients[0].notes, "SYNTHETIC EDIT");
    }
    #[test]
    fn third_save_rotates_only_valid_encrypted_backup() {
        let path = destination("rotate"); let mut w = complete_workspace();
        w.save(&path).unwrap(); w.save(&path).unwrap(); let second = fs::read(&path).unwrap(); w.save(&path).unwrap();
        assert_eq!(w.revision, 3); assert_eq!(fs::read(backup(&path)).unwrap(), second); assert_eq!(Workspace::load(&backup(&path)).unwrap().revision, 2);
    }
    #[test]
    fn corruption_fails_loading_saving_and_preserves_valid_backup() {
        let path = destination("corrupt"); let mut w = complete_workspace(); w.save(&path).unwrap(); w.save(&path).unwrap();
        let previous_backup = fs::read(backup(&path)).unwrap(); let mut damaged = fs::read(&path).unwrap(); let last = damaged.len() - 1; damaged[last] ^= 0xff; fs::write(&path, &damaged).unwrap();
        assert!(Workspace::load(&path).is_err()); let revision = w.revision; assert!(w.save(&path).is_err());
        assert_eq!(w.revision, revision); assert_eq!(fs::read(&path).unwrap(), damaged); assert_eq!(fs::read(backup(&path)).unwrap(), previous_backup);
    }
    #[test]
    fn corrupt_backup_is_never_silently_replaced() {
        let path = destination("corrupt-backup"); let mut w = complete_workspace(); w.save(&path).unwrap(); w.save(&path).unwrap();
        fs::write(backup(&path), b"SYNTHETIC CORRUPT BACKUP").unwrap(); let primary = fs::read(&path).unwrap();
        assert!(w.save(&path).is_err()); assert_eq!(w.revision, 2); assert_eq!(fs::read(&path).unwrap(), primary);
        assert_eq!(fs::read(backup(&path)).unwrap(), b"SYNTHETIC CORRUPT BACKUP");
    }
    #[test]
    fn stale_revision_rejected_without_primary_or_backup_changes() {
        let path = destination("stale"); let mut original = complete_workspace(); original.save(&path).unwrap();
        let mut a = Workspace::load(&path).unwrap(); let mut b = Workspace::load(&path).unwrap(); a.save(&path).unwrap();
        let primary = fs::read(&path).unwrap(); let old_backup = fs::read(backup(&path)).unwrap();
        assert!(matches!(b.save(&path), Err(WorkspaceError::StaleRevision { expected: 1, actual: 2 })));
        assert_eq!(b.revision, 1); assert_eq!(fs::read(&path).unwrap(), primary); assert_eq!(fs::read(backup(&path)).unwrap(), old_backup);
    }
    #[test]
    fn simultaneous_saves_allow_exactly_one_success() {
        let path = destination("parallel"); let mut w = complete_workspace(); w.save(&path).unwrap();
        let barrier = Arc::new(Barrier::new(2));
        let handles: Vec<_> = (0..2).map(|_| {
            let mut copy = w.clone(); let path = path.clone(); let barrier = barrier.clone();
            std::thread::spawn(move || { barrier.wait(); copy.save(&path) })
        }).collect();
        let outcomes: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        assert_eq!(outcomes.iter().filter(|r| r.is_ok()).count(), 1);
        assert!(outcomes.iter().filter_map(|r| r.as_ref().err()).all(|e| matches!(e, WorkspaceError::Busy | WorkspaceError::StaleRevision { .. })));
        assert_eq!(Workspace::load(&path).unwrap().revision, 2);
    }
    #[test]
    fn preexisting_user_lock_is_never_deleted_or_overwritten() {
        let path = destination("lock"); let lock = path.with_file_name("workspace.pvr.lock"); fs::write(&lock, b"SYNTHETIC EXISTING LOCK").unwrap();
        let mut w = Workspace::default(); assert!(matches!(w.save(&path), Err(WorkspaceError::Busy)));
        assert_eq!(fs::read(lock).unwrap(), b"SYNTHETIC EXISTING LOCK"); assert_eq!(w.revision, 0); assert!(!path.exists());
    }
    #[test]
    fn missing_and_non_encrypted_files_never_default_or_overwrite() {
        let path = destination("missing"); assert!(Workspace::load(&path).is_err()); assert!(!path.exists());
        fs::write(&path, b"{\"schema_version\":2}").unwrap(); let mut w = Workspace::default();
        assert!(matches!(w.save(&path), Err(WorkspaceError::InvalidEnvelope))); assert_eq!(fs::read(path).unwrap(), b"{\"schema_version\":2}"); assert_eq!(w.revision, 0);
    }
    #[test]
    fn oversized_file_refused_before_decryption() {
        let path = destination("large"); let file = fs::File::create(&path).unwrap(); file.set_len(MAX_WORKSPACE_BYTES + 1).unwrap(); drop(file);
        assert!(matches!(Workspace::load(&path), Err(WorkspaceError::TooLarge)));
        let mut w = Workspace::default(); assert!(matches!(w.save(&path), Err(WorkspaceError::TooLarge))); assert_eq!(w.revision, 0);
        assert_eq!(fs::metadata(path).unwrap().len(), MAX_WORKSPACE_BYTES + 1);
    }
    #[test]
    fn unsupported_schema_and_revision_overflow_do_not_create_files() {
        let path = destination("schema"); let mut w = Workspace::default(); w.schema_version = 3;
        assert!(matches!(w.save(&path), Err(WorkspaceError::UnsupportedSchema(3)))); assert!(!path.exists());
        w.schema_version = 2; w.revision = u64::MAX; assert!(matches!(w.save(&path), Err(WorkspaceError::RevisionOverflow))); assert!(!path.exists());
    }
    #[test]
    fn invalid_or_directory_paths_never_replaced() {
        let mut w = Workspace::default(); assert!(matches!(w.save(std::path::Path::new("relative.pvr")), Err(WorkspaceError::InvalidPath)));
        let path = destination("directory"); fs::create_dir(&path).unwrap();
        assert!(matches!(w.save(&path), Err(WorkspaceError::UnsafePath))); assert!(path.is_dir());
        let ads = path.with_file_name("file.pvr:stream"); assert!(matches!(w.save(&ads), Err(WorkspaceError::InvalidPath)));
        let reserved = path.with_file_name("CON.pvr"); assert!(matches!(w.save(&reserved), Err(WorkspaceError::InvalidPath)));
    }
    #[test]
    fn failure_does_not_leave_owned_lock_or_temp_files() {
        let path = destination("scratch"); let mut w = complete_workspace(); w.save(&path).unwrap();
        w.revision = 0; assert!(w.save(&path).is_err());
        let names: Vec<_> = fs::read_dir(path.parent().unwrap()).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(names.len(), 1); assert_eq!(names[0], "workspace.pvr");
    }
    #[test]
    fn encrypted_storage_keeps_pending_cnis_provenance() {
        let path = destination("cnis-metadata"); let mut w = complete_workspace();
        let expected = w.calculations[0].cnis.clone(); w.save(&path).unwrap(); let r = Workspace::load(&path).unwrap();
        assert_eq!(r.calculations[0].cnis, expected); let cnis = r.calculations[0].cnis.as_ref().unwrap();
        assert!(!cnis.metadata.legal_validation_performed); assert!(cnis.warnings.iter().any(|w| w.code == "currency_units_unverified"));
    }

    #[test]
    fn failed_final_replace_preserves_primary_revision_and_removes_owned_scratch() {
        let path = destination("readonly-primary"); let mut w = complete_workspace(); w.save(&path).unwrap();
        let original = fs::read(&path).unwrap(); let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_readonly(true); fs::set_permissions(&path, perms).unwrap();
        let outcome = w.save(&path);
        let mut perms = fs::metadata(&path).unwrap().permissions(); perms.set_readonly(false); fs::set_permissions(&path, perms).unwrap();
        assert!(outcome.is_err()); assert_eq!(w.revision, 1); assert_eq!(fs::read(&path).unwrap(), original);
        assert_eq!(fs::read(backup(&path)).unwrap(), original);
        let names: Vec<_> = fs::read_dir(path.parent().unwrap()).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(names.len(), 2); assert!(!path.with_file_name("workspace.pvr.lock").exists());
    }
    #[test]
    fn error_display_contains_neither_paths_nor_sensitive_source_values() {
        let path = destination("error"); fs::write(&path, b"MARKER_73641 invalid envelope").unwrap();
        let error = match Workspace::load(&path) { Err(e) => e, Ok(_) => panic!("corrupt input accepted") };
        let text = error.to_string(); assert!(!text.contains("MARKER_73641")); assert!(!text.contains(&path.to_string_lossy().to_string()));
    }
}
#[cfg(not(windows))]
#[test]
fn non_windows_storage_fails_without_plaintext_fallback() {
    let path = std::path::Path::new("/tmp/not-created-previus.pvr"); let mut w = Workspace::default();
    assert!(matches!(w.save(path), Err(WorkspaceError::Unsupported))); assert!(matches!(Workspace::load(path), Err(WorkspaceError::Unsupported)));
}

#[test]
fn unknown_workspace_fields_are_rejected_instead_of_silently_dropped() {
    assert!(serde_json::from_str::<Workspace>("{\"future_private_field\":\"must not be lost\"}").is_err());
    assert!(serde_json::from_str::<Calculation>("{\"future_rule_field\":true}").is_err());
}
#[test]
fn nominal_i64_minimum_exports_without_overflow_or_formula_execution() {
    let mut c = case(); c.cnis.as_mut().unwrap().salary_entries[0].amount_centavos = i64::MIN;
    let csv = export_salary_csv(&c); assert!(csv.contains("\"'-92233720368547758.08\""));
    assert!(report_html(&c, &Workspace::default(), false).contains("-92233720368547758.08"));
}
#[cfg(windows)]
#[test]
fn loaded_revision_cannot_create_a_missing_destination() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/runtime/missing-revision-target.pvr");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut w = Workspace::default(); w.revision = 9;
    assert!(matches!(w.save(&path), Err(WorkspaceError::StaleRevision { expected: 9, actual: 0 })));
    assert!(!path.exists()); assert_eq!(w.revision, 9);
}
