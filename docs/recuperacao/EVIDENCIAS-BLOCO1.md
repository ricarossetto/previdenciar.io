# Evidências — Bloco 1 Atrium (2026-11-04)

## Implementado no branch candidato
- `atrium/run-18015c24b079/candidate` (a partir de `2346a2e`), unindo:
  - UI: JourneyGuide, CnisTimeline, journey.ts, timeline.ts, ajustes em CaseWorkbench/CnisReview/RobotAssistant/main.
  - CNIS: CnisTemporalView, temporal.ts, ajustes em CaseWorkbench (visão temporal DIB/PBC, cálculo não concluído, bloqueios agrupados).
  - Documento `docs/recuperacao/ESTRATEGIA-LEVEZA.md` e script somente leitura `scripts/recuperacao/medir-espaco.ps1`.

## Verificação real executada
- `npm run typecheck` no worktree candidato: exit 0.
- `npm test` (vitest): 39 arquivos, 297 testes, 297 passaram, 0 falharam, exit 0 (91,71 s).
- Nenhum build Rust, cargo, Docker, deploy, push, commit no principal, exclusão ou movimentação de cache.
- `AUDITORIA.md` e `.opencode/` preservados; nenhum dado real de cliente em fixtures/logs versionados.

## O que NÃO está provado
- Cálculo financeiro do caso real conciliado (exige servidor/build real e dados locais).
- PDF gerado/conferido pelo renderer Rust.
- Execução da jornada pela interface autenticada (sessão 59694 sem página autenticada no CDP).
- Revisão independente do branch candidato (em andamento no Atrium).
- Versão publicada (não houve deploy).
## Correcao pos-revisao (ALTO)
- CnisTemporalView nao exibe zero quando nao ha vinculos confirmados: mostra 'Nao apurado'; grupo sem_periodo suprimido sem confirmacao; DIB anterior a 07/1994 rotulada corretamente.
- Teste de render adicionado (3 casos) e suíte completa:       Tests  300 passed (300).
- Typecheck exit 0.

