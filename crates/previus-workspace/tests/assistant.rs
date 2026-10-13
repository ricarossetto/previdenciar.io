//! Synthetic interview contracts only: no production PDF, identity or database.
use previus_workspace::{AssistantInterview, Calculation, Workspace};
use serde_json::{json, Value};

fn interview() -> AssistantInterview {
    AssistantInterview {
        mode: "assisted".into(), current_benefit_received: Some(false),
        import_requested: Some(true), process_kind: Some("Simulação do benefício".into()),
        goal: Some("Calcular um novo benefício".into()), review_confirmed: true,
        completed_steps: vec!["intro".into(), "benefit".into(), "import".into(),
            "process".into(), "goal".into(), "parameters".into(), "periods".into(),
            "salaries".into(), "policy".into(), "review".into(), "result".into()],
    }
}
fn synthetic_workspace() -> Workspace {
    let mut workspace = Workspace::default();
    let id = workspace.id();
    workspace.calculations.push(Calculation { id, name: "SYNTHETIC INTERVIEW ONLY".into(),
        assistant: Some(interview()), ..Default::default() });
    workspace
}
#[test]
fn old_v2_json_without_interview_defaults_to_none_and_canonical_null() {
    let mut old = serde_json::to_value(synthetic_workspace()).unwrap();
    old["calculations"][0].as_object_mut().unwrap().remove("assistant");
    let workspace: Workspace = serde_json::from_value(old).unwrap();
    assert_eq!(workspace.schema_version, 2);
    assert!(workspace.calculations[0].assistant.is_none());
    let canonical = serde_json::to_value(workspace).unwrap();
    assert!(canonical["calculations"][0].as_object().unwrap().contains_key("assistant"));
    assert_eq!(canonical["calculations"][0]["assistant"], Value::Null);
}
#[test]
fn default_calculation_has_null_interview_and_no_guessed_parameters() {
    let case = Calculation::default();
    assert!(case.assistant.is_none()); assert!(case.sex.is_none()); assert!(case.disability.is_none());
    assert!(serde_json::to_value(case).unwrap()["assistant"].is_null());
}
#[test]
fn all_seven_answers_roundtrip_without_touching_sex_disability_or_result() {
    let workspace = synthetic_workspace(); let value = serde_json::to_value(&workspace).unwrap();
    assert_eq!(value["calculations"][0]["assistant"].as_object().unwrap().len(), 7);
    let restored: Workspace = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(value, serde_json::to_value(&restored).unwrap());
    let case = &restored.calculations[0];
    assert!(case.sex.is_none()); assert!(case.disability.is_none()); assert!(case.result.is_none());
}
#[test]
fn false_true_and_unanswered_are_distinct_for_each_nullable_boolean() {
    for current in [None, Some(false), Some(true)] {
        for requested in [None, Some(false), Some(true)] {
            let mut assistant = interview();
            assistant.current_benefit_received = current; assistant.import_requested = requested;
            let decoded: AssistantInterview = serde_json::from_value(serde_json::to_value(assistant).unwrap()).unwrap();
            assert_eq!(decoded.current_benefit_received, current); assert_eq!(decoded.import_requested, requested);
        }
    }
}
#[test]
fn missing_nested_answers_default_to_unanswered_and_review_false() {
    let assistant: AssistantInterview = serde_json::from_value(json!({"mode":"assisted"})).unwrap();
    assistant.validate().unwrap();
    assert!(assistant.current_benefit_received.is_none()); assert!(assistant.import_requested.is_none());
    assert!(assistant.process_kind.is_none()); assert!(assistant.goal.is_none());
    assert!(!assistant.review_confirmed); assert!(assistant.completed_steps.is_empty());
}
#[test]
fn unknown_interview_field_is_rejected_not_discarded() {
    let mut workspace = serde_json::to_value(synthetic_workspace()).unwrap();
    workspace["calculations"][0]["assistant"]["unknown_field"] = json!(true);
    assert!(serde_json::from_value::<Workspace>(workspace).is_err());
}
#[test]
fn string_number_and_null_cannot_forge_boolean_answers_or_review() {
    for field in ["current_benefit_received", "import_requested", "review_confirmed"] {
        for wrong in [json!("false"), json!(0), json!(1), json!([]), json!({})] {
            let mut value = serde_json::to_value(interview()).unwrap(); value[field] = wrong;
            assert!(serde_json::from_value::<AssistantInterview>(value).is_err());
        }
    }
    let mut value = serde_json::to_value(interview()).unwrap(); value["review_confirmed"] = Value::Null;
    assert!(serde_json::from_value::<AssistantInterview>(value).is_err());
}
#[test]
fn only_exact_assisted_and_manual_modes_are_valid() {
    for valid in ["assisted", "manual"] {
        let mut assistant = interview(); assistant.mode = valid.into(); assistant.validate().unwrap();
    }
    for invalid in ["", "Assisted", "assistant", " assisted", "manual ", "SYNTHETIC PRIVATE MARKER"] {
        let mut assistant = interview(); assistant.mode = invalid.into();
        let error = assistant.validate().unwrap_err(); assert!(!error.contains("SYNTHETIC PRIVATE MARKER"));
    }
}
#[test]
fn all_known_ptbr_process_labels_are_preserved() {
    for label in ["Simulação do benefício", "Revisão e valor da causa", "Planejamento futuro",
        "Processo administrativo", "Execução judicial", "Contribuições em atraso"] {
        let mut assistant = interview(); assistant.process_kind = Some(label.into()); assistant.validate().unwrap();
        let json = serde_json::to_value(&assistant).unwrap(); assert_eq!(json["process_kind"], label);
    }
}
#[test]
fn all_known_goals_are_valid_draft_metadata_not_engine_capabilities() {
    for goal in ["Calcular um novo benefício", "Calcular somente o tempo de contribuição", "Restabelecer um benefício"] {
        let mut assistant = interview(); assistant.goal = Some(goal.into()); assistant.validate().unwrap();
    }
}
#[test]
fn unknown_empty_oversized_or_control_labels_are_rejected_without_echo() {
    for text in [String::new(), "SYNTHETIC PRIVATE MARKER".into(), "ç".repeat(129),
        "Simulação do benefício\n".into(), "Calcular um novo benefício\0".into()] {
        let mut assistant = interview(); assistant.process_kind = Some(text.clone());
        let error = assistant.validate().unwrap_err(); assert!(!error.contains("SYNTHETIC PRIVATE MARKER"));
        assistant.process_kind = None; assistant.goal = Some(text);
        let error = assistant.validate().unwrap_err(); assert!(!error.contains("SYNTHETIC PRIVATE MARKER"));
    }
}
#[test]
fn step_ids_are_closed_unique_and_bounded() {
    let mut assistant = interview(); assistant.validate().unwrap();
    assistant.completed_steps.push("intro".into()); assert!(assistant.validate().is_err());
    assistant.completed_steps = vec!["unknown".into()]; assert!(assistant.validate().is_err());
    assistant.completed_steps = vec!["review".into(); 17];
    assert!(assistant.validate().unwrap_err().contains("excede o limite"));
    assistant.completed_steps.clear(); assistant.validate().unwrap();
}
#[test]
fn completion_ids_never_imply_answer_or_review_confirmation() {
    let mut assistant = interview(); assistant.current_benefit_received = None;
    assistant.process_kind = None; assistant.goal = None; assistant.review_confirmed = false;
    assistant.validate().unwrap();
    let restored: AssistantInterview = serde_json::from_value(serde_json::to_value(assistant).unwrap()).unwrap();
    assert!(restored.current_benefit_received.is_none()); assert!(restored.goal.is_none());
    assert!(!restored.review_confirmed); assert!(restored.completed_steps.contains(&"result".into()));
}
#[test]
fn workspace_structural_validation_accepts_incomplete_interview_drafts() {
    let mut workspace = synthetic_workspace();
    workspace.calculations[0].assistant = Some(AssistantInterview { mode: "assisted".into(), ..Default::default() });
    assert!(workspace.validate().is_empty());
    workspace.calculations[0].assistant.as_mut().unwrap().completed_steps = vec!["intro".into(), "intro".into()];
    assert!(!workspace.validate().is_empty());
}

#[cfg(windows)]
mod native_dpapi {
    use super::*;
    use std::{fs, io::Write, os::windows::process::CommandExt, path::PathBuf,
        process::{Command, Stdio}, sync::atomic::{AtomicU64, Ordering}, time::{SystemTime, UNIX_EPOCH}};
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    fn destination() -> PathBuf {
        let n = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let parent = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/runtime")
            .join(format!("assistant-synthetic-{}-{n:x}-{sequence}", std::process::id()));
        fs::create_dir(&parent).unwrap(); parent.join("workspace.pvr")
    }
    fn legacy_envelope(mut json: Value) -> Vec<u8> {
        json["calculations"][0].as_object_mut().unwrap().remove("assistant");
        // Build an old v2 synthetic envelope independently of the current serializer.
        // No personal data, CLI arguments with data, unsafe code or production paths.
        let script = r#"Add-Type -AssemblyName System.Security; [Console]::InputEncoding = [System.Text.UTF8Encoding]::new($false); $s = [Console]::In.ReadToEnd(); $p = [System.Text.Encoding]::UTF8.GetBytes($s); $c = [System.Security.Cryptography.ProtectedData]::Protect($p, $null, [System.Security.Cryptography.DataProtectionScope]::CurrentUser); $o = [Console]::OpenStandardOutput(); $o.Write($c, 0, $c.Length); $o.Flush()"#;
        let mut child = Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-Command", script])
            .creation_flags(0x08000000).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        child.stdin.take().unwrap().write_all(&serde_json::to_vec(&json).unwrap()).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success(), "Synthetic legacy DPAPI fixture generation failed");
        assert!(output.stderr.is_empty()); assert!(!output.stdout.is_empty());
        let mut bytes = b"PVRS2".to_vec(); bytes.extend_from_slice(&(output.stdout.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&output.stdout); bytes
    }
    #[test]
    fn all_answers_roundtrip_through_actual_dpapi_without_plaintext() {
        let path = destination(); let mut workspace = synthetic_workspace(); workspace.save(&path).unwrap();
        let bytes = fs::read(&path).unwrap(); assert!(bytes.starts_with(b"PVRS2"));
        for marker in [b"SYNTHETIC INTERVIEW ONLY".as_slice(), b"current_benefit_received", b"completed_steps"] {
            assert!(!bytes.windows(marker.len()).any(|window| window == marker));
        }
        let loaded = Workspace::load(&path).unwrap();
        assert_eq!(serde_json::to_value(loaded).unwrap(), serde_json::to_value(workspace).unwrap());
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }
    #[test]
    fn old_v2_ciphertext_load_adds_null_in_memory_without_rewriting_fixture() {
        let path = destination(); let old = legacy_envelope(serde_json::to_value(synthetic_workspace()).unwrap());
        fs::write(&path, &old).unwrap();
        for _ in 0..2 {
            let loaded = Workspace::load(&path).unwrap(); assert!(loaded.calculations[0].assistant.is_none());
            assert_eq!(loaded.schema_version, 2); assert!(loaded.calculations[0].sex.is_none());
            assert!(loaded.calculations[0].disability.is_none()); assert_eq!(fs::read(&path).unwrap(), old);
        }
        assert!(!path.with_file_name("workspace.pvr.backup").exists());
    }
    #[test]
    fn explicit_save_preserves_old_ciphertext_as_backup_and_new_answers() {
        let path = destination(); let old = legacy_envelope(serde_json::to_value(synthetic_workspace()).unwrap());
        fs::write(&path, &old).unwrap(); let mut loaded = Workspace::load(&path).unwrap();
        loaded.calculations[0].assistant = Some(interview()); loaded.save(&path).unwrap();
        assert_eq!(fs::read(path.with_file_name("workspace.pvr.backup")).unwrap(), old);
        let reloaded = Workspace::load(&path).unwrap();
        assert_eq!(serde_json::to_value(reloaded).unwrap(), serde_json::to_value(loaded).unwrap());
        assert!(Workspace::load(&path.with_file_name("workspace.pvr.backup")).unwrap().calculations[0].assistant.is_none());
    }
}
