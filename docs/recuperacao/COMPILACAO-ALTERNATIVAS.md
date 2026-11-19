# Compilacao do servidor web — alternativas concretas

## Fato novo de seguranca
O repositorio `ricarossetto/previdenciar.io` e **PUBLICO**. Portanto:
- push de branch e logs de Actions ficam publicos;
- artefato de Actions em repositorio publico nao e um local privado para o binario;
- nao devemos publicar o executavel do servidor por esse caminho.

## Alternativa A — GitHub Actions em repositorio PRIVADO (recomendada)
- Destino: repositorio privado autorizado (por exemplo, um fork privado ou o repositorio privado ja existente do produto), nunca o `previdenciar.io` publico.
- Workflow `.github/workflows/server.yml`: `workflow_dispatch`, `contents: read`, runner `windows-latest`, build `cargo build -p previdenciar-web-server --locked --profile dev|release`.
- O workflow tem trava: job so roda se `github.event.repository.private == true`; em repositorio publico falha sem gerar artefato.
- Dados enviados: apenas codigo-fonte; nenhum PDF/CNIS/PII.
- Custo: minutos de Actions do repositorio privado (estimativa 10–25 min/execucao; cota depende do plano) e download do binario (~50–60 MB).
- Artefato: `.exe` + `server-build.json` (SHA-256/bytes/commit), retencao 3 dias.
- Obtencao: `gh run download -n previdenciar-web-server-dev-<sha>`; conferir SHA-256; guardar fora do repo.
- Requer autorizacao especifica: push da branch no repositorio privado, execucao manual do workflow e consumo de cota.

## Alternativa A2 — build no repositorio publico com artefato cifrado
- Cifrar o binario (ex.: age/gpg) antes do upload, com chave privada so na maquina local.
- Ainda expoe branch e logs publicos; a chave precisa ser configurada como secret.
- Mais complexo e com superficie de erro maior; nao recomendado frente a A.

## Alternativa B — build local controlado
- Estado medido: C: 73,76 GiB livres; RAM total 15,66 GiB, ~1,95 GiB livres; pagefile com historico de estouro.
- Comando: `cargo build -p previdenciar-web-server --locked --offline -j 1`, via supervisor existente, sem `cargo clean`.
- Crescimento esperado: 0,5–1,5 GiB no `target`; risco de paginacao e invalidacao de fingerprints.
- Para tentar com seguranca: fechar aplicativos pesados, exigir pelo menos ~6 GiB de RAM livre e ~10 GiB de disco livre; supervisor com cap; parar se RAM < 1 GiB livre ou pagefile crescer > 1 GiB.
- Nao reutiliza autorizacao antiga de 2 GiB.
- Recomendacao: somente se A estiver indisponivel e a RAM for liberada.

## Desktop x servidor
- Desktop: `previdenciar-next-desktop`/Tauri, instalador e assinatura (workflow separado).
- Servidor: `previdenciar-web-server`; frontend Vite e build separado.
- Build remoto do servidor nao toca o `target`, o desktop ou dados locais.

## Decisao pendente
1. Autorizar A em repositorio privado (destino, push e execucao manual), ou
2. Autorizar B com RAM liberada e teto local, ou
3. Manter somente auditoria/testes sem build.

Recomendacao tecnica: **A em repositorio privado**.