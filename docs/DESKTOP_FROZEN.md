# Desktop & Web — Paridade Funcional

As versões **Desktop (Tauri v2)** e **Web (`previdenciario.atrium.adv.br`)** compartilham os mesmos motores Rust (`rgps-core`, `previdenciar-rgps`, `previdenciar-cnis`, `previdenciar-reports`, `previdenciar-ai`), os mesmos 11 relatórios Typst/PDF e a integração completa com a Fábrica de Cálculos do TRF3 (Tempo, RMI nas 17 espécies + PcD, Atrasados, leitores documentais e índices).

## Diferenças operacionais entre Desktop e Web

- **Armazenamento local vs nuvem:** Desktop grava a base encriptada via DPAPI local (`workspace.dat` / cofre `ai-keys.pvai`); Web usa PostgreSQL/S3 autenticados por Cognito.
- **Chave da Fábrica TRF3:** Desktop resolve `api_key` explícita -> cofre DPAPI (`"trf3-fabrica"`) -> variável de ambiente `FABRICA_API_KEY`. Web resolve header `x-fabrica-key` (mantido somente em `sessionStorage`) -> variável de ambiente `FABRICA_API_KEY` do servidor (jamais gravada no Postgres).
- **OCR local de CNIS:** Ambos aplicam fallback OCR via `previdenciar-ai::extract_case_pdf` quando o PDF do CNIS não possui texto selecionável.
