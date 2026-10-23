#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![forbid(unsafe_code)]
mod backend;
mod economic;
mod files;
use backend::{BackendState, BootResult};
use previus_cnis::ImportResult;
use previus_workspace::{Calculation, Workspace};
use serde_json::Value;
use tauri::{State, WebviewWindowBuilder};

#[tauri::command]
async fn bootstrap(state: State<'_, BackendState>) -> Result<BootResult, String> {
    // DPAPI, validação e serialização não bloqueiam a thread principal.
    let store = std::sync::Arc::clone(&state.0);
    let result = match tauri::async_runtime::spawn_blocking(move || match store.lock() {
        Ok(mut guard) => guard.bootstrap(),
        Err(_) => BootResult {
            workspace: Workspace::default(),
            storage_ready: false,
            message: Some("Estado privado indisponível; a gravação permanece bloqueada.".into()),
        },
    }).await {
        Ok(result) => result,
        Err(_) => BootResult {
            workspace: Workspace::default(),
            storage_ready: false,
            message: Some("Abertura da base interrompida; a gravação permanece bloqueada até nova abertura.".into()),
        },
    };
    Ok(result)
}

#[tauri::command]
async fn save_workspace(workspace: Workspace, request: tauri::ipc::Request<'_>, state: State<'_, BackendState>) -> Result<Workspace, String> {
    // Tauri typed arguments use the upstream serde model, but it defaults omitted
    // fields. Check the actual invoke body so partial objects cannot erase vectors.
    let raw = match request.body() {
        tauri::ipc::InvokeBody::Json(value) => value.get("workspace").ok_or("Workspace completo ausente.")?,
        _ => return Err("Payload JSON completo obrigatório.".into()),
    };
    backend::validate_full_payload(raw, &workspace)?;
    let guard = std::sync::Arc::clone(&state.0);
    tauri::async_runtime::spawn_blocking(move || {
        guard.lock().map_err(|_| "Estado privado indisponível; gravação recusada.".to_owned())?.save(workspace)
    }).await.map_err(|_| "Gravação interrompida; recarregue para conferir a revisão em disco.".to_owned())?
}

#[tauri::command]
async fn pick_import_cnis() -> Result<Option<ImportResult>, String> {
    tauri::async_runtime::spawn_blocking(files::dialog_import).await
        .map_err(|_| "Importação interrompida; nenhum dado foi gravado.".to_owned())?
}

#[tauri::command]
async fn import_cnis_path(path: String) -> Result<ImportResult, String> {
    tauri::async_runtime::spawn_blocking(move || files::import_path(std::path::Path::new(&path))).await
        .map_err(|_| "Importação interrompida; nenhum dado foi gravado.".to_owned())?
}

#[tauri::command]
async fn run_tool(tool: String, input: Value) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || previus_core::run_tool(&tool, input)).await
        .map_err(|_| "Ferramenta interrompida; nenhum resultado produzido.".to_owned())?
}

#[tauri::command]
async fn run_finance(tool: String, input: Value) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || previus_core::finance::run_finance(&tool, input)).await
        .map_err(|_| "Ferramenta financeira interrompida; nenhum resultado produzido.".to_owned())?
}

#[tauri::command]
async fn fetch_economic_snapshot(through:String)->Result<previus_rgps::pcd::Snapshot,String>{
 tauri::async_runtime::spawn_blocking(move||economic::load(&through)).await.map_err(|_|"Consulta pública interrompida; snapshot anterior preservado.".to_owned())?
}

#[tauri::command]
fn local_economic_snapshot(through: String) -> Result<previus_rgps::pcd::Snapshot, String> {
    previus_rgps::pcd::local_snapshot(&through)
}

#[tauri::command]
async fn calculate_case(calculation: Calculation) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        backend::validate_calculation(&calculation)?;
        backend::validate_assistant_for_calculation(&calculation)?;
        enforce_rgps_envelope(previus_rgps::calculate_case(&calculation)?)
    }).await.map_err(|_| "Motor RGPS interrompido; nenhum resultado produzido.".to_owned())?
}

#[tauri::command]
async fn calculate_rgps(input: Value) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || enforce_rgps_envelope(previus_rgps::calculate_json(input)?)).await
        .map_err(|_| "Motor RGPS interrompido; nenhum resultado produzido.".to_owned())?
}

/// Preserve the actual engine data, warnings and blockers. Never convert a blocked
/// or experimental output into a successful/legally validated benefit result.
fn enforce_rgps_envelope(mut value: Value) -> Result<Value, String> {
    let object = value.as_object_mut().ok_or("Resposta inesperada do motor RGPS; nenhum resultado validado.")?;
    if !object.contains_key("data") || !object.get("warnings").is_some_and(Value::is_array) {
        return Err("Resposta incompleta do motor RGPS; dados e avisos obrigatórios.".into());
    }
    object.insert("legal_validation_performed".into(), Value::Bool(false));
    Ok(value)
}

#[tauri::command]
async fn preview_report(calculation: Calculation, workspace: Workspace, format: String, include_personal: bool,
    presentation: Option<files::PresentationRequest>) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || files::preview_pdf_with(&calculation, &workspace, &format, include_personal,
        presentation.unwrap_or_default())).await
        .map_err(|_| "Prévia interrompida; nenhum arquivo foi gravado.".to_owned())?
}

#[tauri::command]
async fn export_report(calculation: Calculation, workspace: Workspace, format: String, include_personal: bool,
    presentation: Option<files::PresentationRequest>) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || files::dialog_export_with(calculation, workspace, format, include_personal,
        presentation.unwrap_or_default())).await
        .map_err(|_| "Exportação interrompida; confira o destino selecionado.".to_owned())?
}

#[tauri::command]
async fn export_reports_batch(calculation: Calculation, workspace: Workspace, formats: Vec<String>, include_personal: bool,
    presentation: Option<files::PresentationRequest>) -> Result<Option<files::BatchExport>, String> {
    tauri::async_runtime::spawn_blocking(move || files::dialog_export_batch(calculation, workspace, formats, include_personal,
        presentation.unwrap_or_default())).await
        .map_err(|_| "Exportação em lote interrompida; confira a pasta selecionada.".to_owned())?
}

#[tauri::command]
async fn export_document(text: String) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || files::dialog_document(text)).await
        .map_err(|_| "Exportação de documento interrompida.".to_owned())?
}

#[tauri::command]
async fn backup_export(state: State<'_, BackendState>) -> Result<Option<String>, String> {
    let path = {
        let guard = state.lock()?;
        guard.ready()?;
        backend::NativeStore::fixed().path()?.to_path_buf()
    };
    tauri::async_runtime::spawn_blocking(move || {
        let bytes = files::encrypted_snapshot(&path)?;
        files::dialog_backup(bytes)
    }).await.map_err(|_| "Exportação de backup interrompida.".to_owned())?
}


fn navigation_allowed(url: &tauri::Url, development: bool) -> bool {
    if url.username() != "" || url.password().is_some() { return false; }
    (url.scheme() == "tauri" && url.host_str() == Some("localhost"))
        || (matches!(url.scheme(), "http" | "https") && url.host_str() == Some("tauri.localhost") && url.port().is_none())
        || (development && url.scheme() == "http" && url.host_str() == Some("localhost") && url.port() == Some(1427))
}

fn main() {
    let result = tauri::Builder::default()
        .manage(BackendState::default())
        .invoke_handler(tauri::generate_handler![bootstrap, save_workspace, pick_import_cnis, import_cnis_path,
            run_tool, run_finance, fetch_economic_snapshot, calculate_case, calculate_rgps, preview_report, export_report, export_reports_batch, export_document, backup_export])
        .setup(|app| {
            let config = app.config().app.windows.first().ok_or("Configuração da janela principal ausente.")?;
            WebviewWindowBuilder::from_config(app, config)?
                .on_navigation(|url| navigation_allowed(url, cfg!(debug_assertions)))
                .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!());
    if result.is_err() {
        // No error debug dump: operating-system errors can contain private paths.
        eprintln!("Não foi possível iniciar Prévius Rust. Nenhum workspace foi gravado na inicialização.");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn navigation_blocks_external_http_credentials_ports_and_shell_urls() {
        for url in ["https://example.com", "http://localhost:3101", "http://localhost:1427", "file:///C:/secret.txt", "javascript:alert(1)", "https://tauri.localhost:9999", "https://user@tauri.localhost"] {
            assert!(!navigation_allowed(&url.parse().unwrap(), false), "{url}");
        }
        assert!(navigation_allowed(&"http://tauri.localhost/".parse().unwrap(), false));
        assert!(navigation_allowed(&"tauri://localhost/index.html".parse().unwrap(), false));
        assert!(navigation_allowed(&"http://localhost:1427/".parse().unwrap(), true));
    }
    #[test]
    fn rgps_envelope_preserves_blockers_and_forces_unvalidated() {
        let value = json!({"status":"blocked", "data":null, "warnings":["review"], "blockers":["source"], "legal_validation_performed":true});
        let actual = enforce_rgps_envelope(value).unwrap();
        assert_eq!(actual["status"], "blocked"); assert_eq!(actual["warnings"], json!(["review"]));
        assert_eq!(actual["blockers"], json!(["source"])); assert_eq!(actual["legal_validation_performed"], false);
        assert!(enforce_rgps_envelope(json!({"data":42})).is_err());
    }
    #[test]
    fn missing_rgps_inputs_produce_real_blocked_result_not_fabricated_math() {
        let actual = enforce_rgps_envelope(previus_rgps::calculate_case(&Calculation::default()).unwrap()).unwrap();
        assert_eq!(actual["status"], "blocked"); assert!(actual["data"].is_null());
        assert!(actual["blockers"].as_array().unwrap().len() > 0); assert_eq!(actual["legal_validation_performed"], false);
    }
    #[test]
    fn existing_core_facade_is_real_and_unknown_tool_is_error() {
        let value = previus_core::run_tool("inclusive_dates", json!({"start":"2026-10-10","end":"2026-10-10"})).unwrap();
        assert!(!value["data"].is_null()); assert!(previus_core::run_tool("missing_finance", json!({})).is_err());
    }
    #[test]
    fn config_and_capabilities_have_only_local_command_permissions() {
        let config: Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let capability: Value = serde_json::from_str(include_str!("../capabilities/main.json")).unwrap();
        assert_eq!(config["build"]["frontendDist"], "../dist");
        assert_eq!(config["build"]["devUrl"], "http://localhost:1427");
        assert_eq!(config["app"]["windows"][0]["width"], 1440);
        assert_eq!(config["app"]["windows"][0]["minWidth"], 1024);
        let csp = config["app"]["security"]["csp"].as_str().unwrap();
        assert!(csp.contains("default-src 'self'")); assert!(csp.contains("script-src 'self';"));
        assert!(!csp.contains("unsafe-eval")); assert!(!csp.contains("https:"));
        assert!(!csp.contains("localhost:1427")); assert!(csp.contains("frame-src blob:")); assert!(csp.contains("object-src 'none'")); assert_eq!(capability["windows"], json!(["main"]));
        assert!(capability.get("remote").is_none());
        for permission in capability["permissions"].as_array().unwrap() {
            let permission = permission.as_str().unwrap();
            assert!(permission.starts_with("allow-")); assert!(!permission.contains(':'));
        }
        let permissions = capability["permissions"].as_array().unwrap();
        assert!(permissions.iter().any(|value| value == "allow-export-reports-batch"));
        assert_eq!(permissions.len(), 15);
    }
    #[test]
    fn direct_rgps_json_rejects_invalid_schema_without_false_result() {
        assert!(previus_rgps::calculate_json(json!({"unexpected":"synthetic"})).is_err());
    }

    #[test]
    fn finance_command_returns_real_experimental_data_and_preserves_warnings() {
        let input = json!({"principal_money":"1000.00", "monthly_correction_percent":["1"],
            "interest_percent_per_month":"1", "months":1, "interest_mode":"simple",
            "fees_percent":"0", "fees_fixed_amount":"0.00", "rounding":"half_away_from_zero",
            "provenance":"synthetic backend test; operator-only parameters"});
        let actual = tauri::async_runtime::block_on(run_finance("update_values".into(), input)).unwrap();
        assert_eq!(actual["status"], "experimental"); assert!(!actual["data"].is_null());
        assert_eq!(actual["rules_verified"], false); assert!(!actual["warnings"].as_array().unwrap().is_empty());
        assert!(tauri::async_runtime::block_on(run_finance("unknown".into(), json!({}))).is_err());
    }
    #[test]
    fn actual_async_case_command_does_not_promote_blocked_original_cnis() {
        let actual = tauri::async_runtime::block_on(calculate_case(Calculation::default())).unwrap();
        assert_eq!(actual["status"], "blocked"); assert!(actual["data"].is_null());
        assert_eq!(actual["legal_validation_performed"], false);
    }

}

#[cfg(test)]
mod rgps_positive_integration {
    #[test]
    fn actual_async_case_command_returns_native_rmi_for_explicit_synthetic_review() {
        let case: previus_workspace::Calculation = serde_json::from_str(include_str!("../../../../crates/previus-rgps/tests/fixtures/calculation-synthetic.json")).unwrap();
        let result = tauri::async_runtime::block_on(super::calculate_case(case)).unwrap();
        assert_eq!(result["status"], "experimental", "{result}");
        assert_eq!(result["legal_validation_performed"], false);
        assert_eq!(result["data"]["native_result"]["resumo"]["rmi"], "1600.00");
        assert_eq!(result["data"]["native_result"]["resumo"]["competencias"], 300);
        assert!(!result["warnings"].as_array().unwrap().is_empty());
    }
}
