# Candidato e rollback — Bloco 1 (privado, sem deploy)

## Estado do candidato
- Branch: `atrium/run-18015c24b079/candidate`; HEAD no momento da geração: `4c1375c`.
- Base: `2346a2e` (branch `wip/consolidacao-20261102`); principal **não** foi alterado.
- Verificação: typecheck exit 0; 40 arquivos, **301 testes, 301 aprovados, 0 falhas**; build web exit 0.
- Build web gerado em `apps/desktop/dist` (24 arquivos, 3797223 B).
- index-B9u-0MSx.js — 716213 B — SHA-256 513F1588DE839AC22A56CEA17C13C231650E5759844800C26A20FBAAA861A6B6
- index-CZwy9-VG.css — 234442 B — SHA-256 C48E7A8269904CE07C3C23EF804EBDEAC8A217BB0C106F2C43EEE278E6B71469
- index-DqhrN302.js — 339218 B — SHA-256 C3FBD42D2C713CB3DD9B94003035E627E17A573E92FE4532EAB0EF2EB38271C4

## Produção observada (somente leitura)
- `https://previdenciario.atrium.adv.br` responde 200 via Cloudflare.
- Assets públicos atuais: `index-DlBZOfH4.js`, `index-CM0sSWgM.css` — **diferentes** do candidato; o site publicado não contém este bloco.
- Imagem/task de produção não foi inspecionada/alterada; o identificador `prod-20261031-10` é referência do relato anterior, não foi verificado por AWS nesta sessão.

## Procedimento de publicação (NÃO EXECUTADO)
1. Congelar tag do candidato e gerar imagem do servidor somente após build autorizado e validado.
2. Registrar a revisão/task definition e o digest da imagem atualmente em produção (rollback point).
3. Publicar em ambiente de homologação primeiro; validar login, CNIS, cálculo e PDF.
4. Promover a imagem; monitorar `/login` e `/health` (rota real da API) e erros do serviço.
5. Rollback: reverter o serviço para a revisão/task definition anterior; não reverter banco sem migração explícita (não há migração neste bloco).

## Bloqueios para concluir
- Autorização e teto para build Rust controlado (servidor), com pré-check de disco/RAM/pagefile.
- Sessão autenticada para validar importação, cálculo e PDF pela interface real.
- Nenhum deploy, merge no principal, migração ou limpeza de cache será feito sem autorização específica.