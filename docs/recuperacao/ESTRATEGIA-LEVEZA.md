# Estrategia de leveza do target (build sem compilar)

Status: proposta documentada, nada aplicado. Data da consolidacao: 04/11/2026.
Fonte primaria: `ATRIUM-BLOCO1-FINDINGS-20261104.md`, secao "Medir target de ~42,4 GB e diagnosticar o excesso de espaco" (fora do Git).

Este documento consolida os numeros ja medidos em modo somente leitura e descreve alternativas de reducao de espaco **sem** executar build, cargo, Docker, exclusao, movimentacao, commit ou deploy. Nenhuma acao de escrita no `target` foi feita.

## Regras desta estrategia

- Proibido: `cargo`/build Rust, Docker, deploy, push, commit, migracao, `cargo clean`, apagar/mover `target`, caches, `.env` ou bancos.
- Proibido tocar `AUDITORIA.md` e `.opencode/`.
- Nada de PDF, CNIS ou dado pessoal para servico externo. Numeros reais desta maquina ficam apenas localmente; este documento e sanitizado e nao reproduz PII.
- Qualquer delecao, novo baseline, alteracao de cap/piso ou poda de worktrees exige autorizacao explicita (ver secao 5).
- O script `scripts/recuperacao/medir-espaco.ps1` e 100% leitura: mede e imprime; nao cria, move, apaga nem compila nada.

## 1. Numeros medidos (04/11/2026, somente leitura)

Medicao no checkout principal (`desktop`), onde `target/` existe. Este worktree isolado **nao** tem `target/` (verificado com `Test-Path target` = False); os numeros abaixo sao historicos e devem ser reconfirmados com o script antes de qualquer decisao.

| Item | Bytes | Decimal | Binario | Arquivos |
|---|---:|---:|---:|---:|
| **repo total** | **44.829.815.172** | **44,83 GB** | **41,75 GiB** | 70.957 |
| `target/` | 44.491.472.441 | 44,49 GB | 41,44 GiB | 60.361 |
| target/debug | 40.668.806.183 | 40,67 GB | 37,88 GiB | 52.694 |
| debug/deps | 30.345.953.902 | 30,35 GB | 28,26 GiB | 11.627 |
| debug/incremental | 5.686.258.684 | 5,69 GB | 5,30 GiB | 18.592 |
| debug/examples | 2.412.261.371 | 2,41 GB | 2,25 GiB | 47 |
| debug/build | 2.163.386.675 | 2,16 GB | 2,01 GiB | 5.020 |
| debug/.fingerprint | 4.226.656 | 4,03 MB | 4,03 MiB | 17.403 |
| target/release (congelado desde 30/10) | 3.759.070.654 | 3,76 GB | 3,50 GiB | 7.404 |
| target/wasm32-unknown-unknown | 63.592.810 | 63,6 MB | 60,65 MiB | 261 |
| target/tmp | 0 | - | - | 0 |
| work/ (nao descartavel: work/refs = 143,92 MiB) | 174.546.277 | 174,5 MB | 166,46 MiB | 2.595 |
| apps/ (node_modules = 124,14 MiB) | 139.431.928 | 139,4 MB | 132,97 MiB | 6.896 |
| .git/ (3 packs, garbage 0) | 14.700.144 | 14,7 MB | 14,02 MiB | 816 |
| outros dirs + 21 arquivos da raiz | 9.664.382 | 9,7 MB | 9,22 MiB | ~100 |
| *contexto:* <pagefile> | 31.833.718.784 | 31,8 GB | 29,65 GiB | 1 |
| *contexto:* volume C: usado / livre | 430.416.179.200 / 79.972.380.672 | 400,86 / 74,48 GiB | - | - |

Conferencias: a soma dos itens fecha o total do repo (44.829.815.172 B); a soma das partes de `target` fecha o total de `target` por dois caminhos independentes (`robocopy /L` e `Get-ChildItem` + `Measure-Object`). `target` = 99,2% dos bytes e 85,1% dos arquivos do repo.

### Distribuicao por extensao dentro de `target` (medido)

| Extensao | Tamanho | Arquivos |
|---|---:|---:|
| `*.rlib` | 18,75 GB (17,46 GiB) | 3.613 |
| `*.pdb` | 10,65 GB (9,92 GiB) | 970 |
| `*.rmeta` | 5,79 GB (5,39 GiB) | - |
| `*.o` | 3,61 GB (3,36 GiB) | - |
| `*.exe` | 2,66 GB (2,48 GiB) | - |

So em `debug/` sao 9,51 GiB de PDB em 844 arquivos; `.rlib` com DWARF somam 16,85 GiB em 2.835 arquivos.

## 2. Causas do excesso

1. **Sem `[profile.dev]`** em `Cargo.toml:5-8` (so existe `[profile.release]`): o default `debug = 2` gera PDB e DWARF.
2. **`debug/deps` = 28,26 GiB (68% do target)**: rlib + rmeta + PDB de todas as dependencias, normal e regeneravel.
3. **`debug/incremental` = 5,30 GiB** em 18.592 arquivos (3,56 GiB em `.o`): cache incremental puro.
4. **`debug/examples` = 2,25 GiB em 47 arquivos**: 15 `.exe` (285 MB) + PDBs gigantes; pior razao espaco/utilidade da arvore.
5. **Duas arvores paralelas**: `debug` quente (deps ate 03/11 20:07), `release` congelado desde 30/10 e `wasm32`. `release/incremental` e `target/tmp` ja estao vazios.
6. **`.fingerprint`**: ~5 MB, mas 17.403 arquivos; custo de backup/antivirus/indexacao, nao de bytes.
7. `target/` e o target-dir **padrao** (sem `.cargo/config.toml`, `CARGO_TARGET_DIR` vazio): equivale a 10,3% do volume C: usado e 56% do espaco livre atual.

Observacao sobre PDBs orfaos: os 970 `.pdb` medidos incluem artefatos que deixam de ser referenciados quando o perfil ou as features mudam. Trocar `debug` invalida fingerprints e deixa PDBs antigos orfaos; eles **so** saem com limpeza autorizada, nunca por efeito da configuracao.
Resumo dos candidatos: os 970 `.pdb` (9,92 GiB) sao os PDBs orfaos em potencial; `incremental` = 5,3 GiB; worktrees Git fora do repo = ~1,79 GB (24 na medicao de 04/11). Com config apenas (sem build): `debug = 0` reduz ESTIMATIVA 14 a 19 GiB; `line-tables-only`, 10 a 14 GiB; combinado com `incremental = false` leva o target a ~20-25 GiB, reducao estimada de ~19 a ~24 GiB.

## 3. Alternativas: efeito x custo x risco x autorizacao

Rotulo: **MEDIDO** = numero desta medicao; **ESTIMATIVA** = projecao que ainda nao foi comprovada por build (proibido neste bloco).

| Alternativa | Efeito no repouso | Custo | Risco / reversao | Autorizacao |
|---|---|---|---|---|
| `[profile.dev] debug = 0` | ESTIMATIVA -14 a -19 GiB | so config, sem build | perde debug linha a linha; reverter = 1 linha | editar `Cargo.toml` (fora deste bloco) |
| `debug = "line-tables-only"` (recomendado) | ESTIMATIVA -10 a -14 GiB | so config, sem build | baixo; mantem arquivo:linha em backtrace; reversivel | editar `Cargo.toml` |
| `incremental = false` | ESTIMATIVA -5,3 GiB, mas o dir antigo **so** encolhe com remocao autorizada; a config sozinha nao apaga o existente | so config | rebuilds 2 a 5x mais lentos; reversivel | config + delecao separada |
| combinado: dev sem debuginfo e sem incremental | ESTIMATIVA -19 a -24 GiB, target ~20-25 GiB | proxima compilacao integral invalida fingerprints e gera pico temporario | PDBs antigos ficam orfaos ate limpeza aprovada | config + rodada de build futura autorizada |
| `target-dir` em outro volume | MEDIDO ate -41,4 GiB em C: | mover ~60 mil arquivos; custo integral no destino | medio; exige re-declarar baseline/ledger, sem reset silencioso; reversivel | mover/realocar exige autorizacao |
| `target-dir` no mesmo volume / split por perfil | 0 B; split piora +20 a +35 GiB | - | nao recomendado: duplica `deps` | - |
| build so em CI/remoto/sccache | nao reduz o repouso sozinho; sccache ocupa 5-15 GiB fora do repo e nao encolhe os 37,9 GiB locais | rede e CI; hoje nao ha cache de Rust nos 4 workflows | alto no fluxo, baixo no disco; desfazer = restaurar workflow | mudanca de CI |
| podar members do workspace | ESTIMATIVA 1 a 5 GiB | editar lista de members | medio: quebra `--workspace` e testes | edicao de `Cargo.toml` |

## 5. O que exige autorizacao explicita

1. **Delecao/movimentacao** de qualquer parte de `target/` (inclusive `incremental`, PDBs orfaos, `.rlib` velhos) ou de caches.
2. **Novo baseline/ledger** de espaco: proibido reset silencioso (`docs/TEST_BUDGET.md`).
3. **`cargo clean`**, `cargo build`, `cargo test` ou qualquer compilacao Rust.
4. **Realocar `target`** para outro volume ou mudar `CARGO_TARGET_DIR`.
5. **Poda de worktrees** Git (24 vinculados, ~1,79 GB fora do repo): usar `git worktree prune/remove`, nunca apagar `.git/worktrees` a mao.
6. **Editar `Cargo.toml`**: adicionar `[profile.dev]` ou podar members altera o contrato de build.
7. **Alterar workflows/CI** (hoje nenhum tem cache de Rust).
8. **Publicacao** de numeros/PII: os numeros reais desta maquina nao vao para o Git nem para servico externo.

## 6. Caminho recomendado sem compilar

1. Remedir **agora** com `scripts/recuperacao/medir-espaco.ps1` (leitura pura).
2. Reconciliar a unidade do brief (42,4 GB) com o medido (41,44 GiB / 44,49 GB).
3. O dono decide o perfil; so entao uma mudanca separada aplica esta proposta (nao aplicada aqui):

```toml
[profile.dev]
debug = "line-tables-only"
incremental = false
```

4. A primeira compilacao depois da mudanca e integral (fingerprints invalidados) e gera pico temporario; so rodar com build autorizado e `scripts/run-with-disk-budget.mjs` acompanhando.
5. PDBs e `incremental` antigos continuam no disco ate limpeza autorizada; a config nao apaga nada.

## 7. Como usar o script de medicao

```powershell
pwsh -NoProfile -File scripts/recuperacao/medir-espaco.ps1
pwsh -NoProfile -File scripts/recuperacao/medir-espaco.ps1 -RepoRoot "C:\\caminho\\do\\checkout"
pwsh -NoProfile -File scripts/recuperacao/medir-espaco.ps1 -Json
```

- Saida: volume C:, total do repo, `target/` por subdiretorio, extensoes (`.pdb`, `.rlib`, `.rmeta`, `.o`, `.exe`), worktrees Git e pagefile.
- Codigos de saida: `0` medicao ok; `2` `target/` ausente (aviso, sem erro); `1` falha de leitura.
- O script **nao** grava arquivos, nao apaga, nao move, nao compila e nao chama cargo. `-Json` apenas muda o formato da saida.
- `-IncludeWorktrees` mede tambem o tamanho das worktrees listadas (mais lento); por padrao apenas lista os caminhos.

## 8. Como reverter

- **Este bloco (doc + script)**: nada foi alterado no ambiente; reverter = remover `docs/recuperacao/ESTRATEGIA-LEVEZA.md` e `scripts/recuperacao/medir-espaco.ps1` (ou `git revert` do commit futuro).
- **Mudanca futura de perfil**: apagar as linhas de `[profile.dev]` e recompilar; os artefatos voltam ao formato anterior.
- **Mudanca futura de `target-dir`**: desfazer a variavel/config, re-declarar o baseline no ledger e revalidar com `--check`.

## 9. Riscos abertos

1. Unidade do brief (42,4 GB) x medido (41,44 GiB / 44,49 GB): reconciliar antes de dimensionar qualquer acao.
2. Qualquer delecao de `incremental`, PDB orfao ou `.rlib` velho esta **fora** deste bloco: exige autorizacao + novo baseline (`docs/TEST_BUDGET.md`).
3. Editar perfil invalida fingerprints: proxima compilacao e integral, com pico temporario antes do ganho.
4. 24 worktrees Git vinculados (~1,79 GB, fora do repo): podar so com `git worktree prune/remove` e decisao humana.
5. `work/refs` (143,9 MiB) parece artefato real, nao cache: nao tratar como descartavel.
6. `.opencode/` e `AUDITORIA.md` sao untracked preexistentes: nao tocar.
7. Nenhum dado real saiu da maquina; o script e local e somente leitura.

## Referencias

- `ATRIUM-BLOCO1-FINDINGS-20261104.md` (fora do Git), secao "Medir target".
- `Cargo.toml:2` (members) e `Cargo.toml:5-8` (somente `[profile.release]`).
- `scripts/disk-budget.mjs`, `scripts/run-with-disk-budget.mjs`, `scripts/m1-budget-supervisor.mjs`.
- `docs/TEST_BUDGET.md` (politica fixa e proibicao de reset silencioso).
- `.github/workflows/release.yml:40-41` e `:67-70` (build release no CI, sem cache de Rust).

## 10. Verificacao da execucao deste bloco

Teste de aceite em 03-04/11/2026, somente leitura, no checkout principal `<repo-desktop>` (onde existe `target/`):

- `target` = **44.491.472.441 B / 60.361 arquivos** — identico ao numero da medicao de 04/11.
- `debug/deps` 28,26 GiB / 11.627; `debug/incremental` 5,30 GiB / 18.592; `debug/examples` 2,25 GiB / 47; `debug/build` 2,01 GiB / 5.020; `.fingerprint` 4,03 MiB / 17.403.
- Extensoes: `*.pdb` 9,92 GiB / 970; `*.rlib` 17,46 GiB / 3.613; `*.rmeta` 5,39 GiB / 4.441; `*.o` 3,36 GiB / 18.848; `*.exe` 2,47 GiB / 805.
- `repo total` = 44.830.031.596 B / 70.989 arquivos: variacao de **+216.424 B / +32 arquivos** em relacao a medicao de 04/11 (44.829.815.172 B / 70.957). E drift normal de uso, nao contradiz os numeros.
- Worktrees: o script lista **29 entradas** (28 worktrees vinculadas + o checkout principal), contra as 24 da medicao de 04/11; a contagem e viva e cresce com novas worktrees. Nenhuma foi removida ou podada.
- Em worktree sem `target/` (este checkout): mensagem "target: ausente", saida **2**, sem erro e sem escrita.
- `-Json` emite o mesmo conteudo em JSON; nao grava arquivo.

Nada foi excluido, movido, compilado, commitado ou enviado para fora da maquina.

