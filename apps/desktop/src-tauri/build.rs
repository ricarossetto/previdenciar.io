fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(
            tauri_build::AppManifest::new().commands(&[
                "bootstrap", "save_workspace", "pick_import_cnis", "import_cnis_path",
                "run_tool", "run_finance", "fetch_economic_snapshot", "conferir_fabrica_rmi",
                "fabrica_status", "fabrica_save_key", "fabrica_clear_key",
                "calcular_fabrica_tempo", "calcular_fabrica_atrasados",
                "ler_documento_fabrica", "consultar_fabrica_indices",
                "local_economic_snapshot", "prepare_case_from_cnis", "calculate_case",
                "calculate_rgps", "project_rgps", "preview_report", "export_report",
                "export_reports_batch", "export_document", "merge_pdfs", "backup_export",
                "ai_provider_catalog", "ai_key_status", "ai_save_key", "ai_delete_key",
                "ai_test", "ai_models", "ai_data_status", "ai_import_data",
                "ai_knowledge_search", "ai_knowledge_read", "save_export_file",
                "pick_case_pdf_text", "ai_chat", "ai_cancel",
            ]),
        ),
    ).expect("Falha na configuração de build Tauri (nenhum dado de usuário lido).");
}
