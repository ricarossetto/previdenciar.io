# previdenciar.io contributor instructions

Read README.md, docs/ARCHITECTURE.md and docs/PRIVACY.md before changing the application.

Preserve exact money, source provenance, pending states and experimental-result labels. Never manufacture missing data or turn a blocked calculation into a financial result. All fixtures must be synthetic.

Do not commit CNIS, personal PDFs, encrypted workspaces, application databases, logs, screenshots of clients, secrets or proprietary binaries. Run the public source review before publishing.

Keep the Tauri application identifier and the legacy storage path compatible unless an explicit, tested migration is implemented. Brand display text does not authorize a storage migration.

PDF preview and export must use the real Rust renderer. Do not replace them with HTML/JSON mocks. Native tests and browser IPC fixtures must be described separately.

The brand assets are part of the product: `apps/desktop/public/logo-completa.png` (horizontal logo), `logo-ico.png` (favicon), `apps/desktop/src-tauri/icons` (app/installer icons) and `docs/assets` (README). Do not replace them with generic placeholders or a text-only brand.
