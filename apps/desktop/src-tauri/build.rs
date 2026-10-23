fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(
            tauri_build::AppManifest::new().commands(&[
                "bootstrap", "save_workspace", "pick_import_cnis", "import_cnis_path",
                "run_tool", "run_finance", "fetch_economic_snapshot", "local_economic_snapshot", "prepare_case_from_cnis", "calculate_case", "calculate_rgps", "preview_report", "export_report", "export_reports_batch",
                "export_document", "backup_export",
            ]),
        ),
    ).expect("Falha na configuração de build Tauri (nenhum dado de usuário lido).");
}
