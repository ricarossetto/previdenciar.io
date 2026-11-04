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


## Correcoes dos achados medios (revisao independente)
- Janela do PBC enumerada (meses) e nota de cobertura: mes sem lancamento nao e lacuna automatica nem zero.
- Contagem 'Dentro do PBC' e tabela usam o mesmo recorte com remuneracao; competencias sem remuneracao ficam em tabela propria.
- CTA de identificacao aponta para a etapa 0, onde esta o campo DIB; jornada e testes atualizados.
- Badge do CNIS explicita 'no recorte do PBC' e 'fora do recorte'.
- Documento de build sanitizado (sem caminho absoluto) e numeros de .fingerprint reconciliados.
- Suite: 40 arquivos, 300 testes, 300 aprovados; typecheck exit 0.


## Revisao focada (run_1b6cb1a9305c)
- Veredito: APROVADO COM RESSALVAS; typecheck 0 e 300/300 testes confirmados em HEAD 72ac170.
- Medio corrigido: DIB brasileira reconhecida na rotula/regua; teste adicionado.
- Badge do CNIS separado por dimensao (no recorte / antes / depois / lacunas), sem somar categorias.
- Caminhos absolutos pre-existentes sanitizados no doc de build.
- Build de producao do frontend (build:web) exit 0: 24 arquivos, 3.797.242 B; login renderizado sem erro de console; bundle diferente do publicado (producao index-DlBZOfH4.js).
