# Compilacao do servidor web — alternativas concretas

## Escopo
Somente `previdenciar-web-server` (Axum). Nao recompilar desktop/Tauri nem gerar instalador.

## Alternativa A — GitHub Actions (recomendada)
- Workflow preparado em `.github/workflows/server.yml` na branch candidata; gatilho manual `workflow_dispatch`; permissao `contents: read`.
- Runner `windows-latest`; build `cargo build -p previdenciar-web-server --locked --profile dev|release`.
- Cache somente do registro Cargo; `target` nao e cacheado.
- Artefato privado com `.exe` + JSON de SHA-256/bytes/commit, retencao 3 dias.
- Dados enviados: apenas o codigo do repositorio privado; nenhum PDF/CNIS/dado pessoal.
- Custo: minutos de Actions do repositorio privado (estimativa 10–25 min/execucao; cota a conferir) e download do binario (~50–60 MB).
- Obtencao: `gh run download -n previdenciar-web-server-dev-<sha>`; conferir SHA-256; guardar fora do repo.
- Requer autorizacao para push da branch candidata e consumo da cota de Actions.

## Alternativa B — build local controlado
- C: 73,76 GiB livres; RAM ~1,95 GiB livres; pagefile com historico de cap.
- Comando: `cargo build -p previdenciar-web-server --locked --offline -j 1` via supervisor existente, sem `cargo clean`.
- Crescimento esperado: 0,5–1,5 GiB no `target`; risco de paginacao e invalidacao de fingerprints.
- Parada: bloqueio do supervisor; RAM livre < 1 GiB; pagefile alocado crescendo > 1 GiB. Sem credito de orcamento antigo.
- Recomendacao: nao executar agora; RAM livre insuficiente.

## Desktop x servidor
- Desktop: `previdenciar-next-desktop`/Tauri, instalador e assinatura (workflow separado).
- Servidor: `previdenciar-web-server`; frontend Vite e build separado; o build remoto do servidor nao toca o `target` local.

## Decisao pendente
Autorizar A (push + execucao manual) ou B (build local com teto e condicoes). Recomendacao tecnica: A.