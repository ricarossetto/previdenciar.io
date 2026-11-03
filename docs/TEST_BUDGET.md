# Orçamento de disco durante comandos

Este documento descreve o wrapper `scripts/run-with-disk-budget.mjs` da tarefa 1.3b. Ele complementa o monitor de snapshot `scripts/disk-budget.mjs`; não substitui o `--check` e não libera Cargo, build, Docker ou banco.

## Política fixa

- **2 GiB** de crescimento total desde a retomada, somando todas as raízes declaradas.
- **10 GiB** de espaço livre mínimo no volume de cada raiz/cache/volume declarado, mais a reserva.
- **256 MiB** reservados para interrupção, relatórios e o próprio ledger.
- **64 MiB** de cobrança inicial conservadora (schema v2), para os artefatos anteriores à retomada e a incerteza do baseline. Isso **reduz o saldo**; não cria um novo cap.
- O baseline antigo de **46,66 GiB aproximado, incerteza 0,01** não é alterado. A diminuição externa do `target` é registrada no ledger como diminuição, não como crédito nem como novo teto.
- Não há flag de CLI para aumentar o cap, baixar o piso, remover a reserva ou zerar a cobrança inicial. Testes podem injetar política pequena apenas em funções; a CLI pública usa a política fixa.

O ledger é **estado de ambiente**, fora do Git, sob a saga. Ele não é prova pública, não deve ser commitado e não é recriado por uma medição nova. O primeiro uso exige `--init-ledger`; um ledger existente nunca é sobrescrito por `--init-ledger`. Um ledger v1 existente só evolui com `--reconcile-initial`, que registra o ponto anterior, preserva o histórico e cobra a inicial; não aceita valor zero nem reset.

## Reconciliação inicial (v2)

| Item | Valor/limite | Fonte |
|---|---:|---|
| Cobrança inicial | 67.108.864 B (64 MiB) | decisão conservadora autorizada em 03/11/2026 |
| Incerteza do baseline | 10.737.418 B (0,01 GiB) | `baseline-orcamento-20261102.json` |
| Scripts/docs desta tarefa | ~76 kB (três arquivos) | `git show --stat f6d99ca` + `cd46fb0` + `2839cea` |
| Saga/testes/ledger/provas conhecidos | até ordem de MiB | `tmp-historico/`, `tmp-orcamento-*`, ledger, logs, `AUDITORIA.md` |
| Cap total | 2.147.483.648 B | política preservada |

O inventário é conservador, não uma promessa de contagem exata de todo byte histórico. A cobrança de 64 MiB cobre os artefatos conhecidos e a incerteza; não autoriza apagar, renovar cap ou ignorar o baseline antigo. O saldo efetivo é `used = carried_charge + cobranças positivas`; `remaining = cap - used - reserva`.

## Uso

```text
node scripts/run-with-disk-budget.mjs \
  --ledger "$SAGA/run-budget-ledger.json" \
  --baseline "$SAGA/baseline-orcamento-20261102.json" \
  --root target=target \
  --root repo=. \
  --root saga="$SAGA" \
  [--root temp="$SAGA/tmp-testes-integracao"] \
  [--root pg="<pasta-do-postgres>"] \
  [--cache cargo="<cache-cargo>"] \
  [--volume "<outro-volume>"] \
  [--cwd .] [--timeout-seconds N] \
  [--init-ledger | --reconcile-initial] \
  -- <comando> [args...]
```

`--reconcile-initial` é um modo de migração v1→v2 **sem comando**: exige ledger existente, preserva `previous` e histórico e cobra a inicial fixa. Não existe valor de CLI para escolher zero.

- `target`, `repo` e `saga` são obrigatórios; `temp` e `pg` são opcionais.
- `target` precisa estar dentro de `repo`; `repo` exclui `target` da soma para não contar duas vezes. Raízes aninhadas são excluídas umas das outras.
- `--cache` deve apontar para cache **preexistente**; o wrapper não cria cache, não baixa e não instala. Caches não declarados não entram no ledger e não têm garantia.
- `--volume` acrescenta um volume ao piso de 10 GiB + reserva. Todos os segmentos (`roots`, `caches` e `pg`) são verificados no volume em que estão; a deduplicação usa `dev` quando disponível e a raiz do caminho apenas como fallback de chave, sem inferir letra de forma frágil.
- `--cwd` é o diretório do filho; não é impresso no resultado.
- O comando é executado **sem shell**, com `stdio` herdado. O wrapper não interpola strings, não usa `Invoke-Expression` e não imprime `argv`.
- Use `--init-ledger` apenas no primeiro uso, com todas as raízes já definidas. Ledger v1 existente usa `--reconcile-initial`; não há reset por medição.

## O que é medido

O wrapper mede metadados, sem ler conteúdo:

- `target`: artefatos de build existentes;
- `repo`: saídas controláveis fora do target (frontend, scripts, test outputs);
- `saga`: artefatos, ledger, fixtures e provas;
- `temp`: raiz temporária identificada (`tmp-testes-integracao...`);
- `pg` e `--cache`: volumes/caches preexistentes declarados;
- espaço livre por volume via `statfs` no próprio caminho declarado, com fallback `Get-PSDrive`, sem varrer o disco inteiro e sem imprimir paths.

Cada segmento tem `start_bytes`, `high_water_bytes`, `charged_bytes` e `last_observed_bytes`. A cobrança é **cumulativa**: em cada amostra, `Δ > 0` soma em `charged_bytes`; `Δ < 0` não devolve crédito; `high_water_bytes` é apenas metadado. Assim, `100 → 120 → 80 → 100` cobra 20 + 20 = 40, não 20. Uma raiz que diminui não cancela outra que cresce; a soma é feita por segmento e depois somada. O novo `charged_bytes` não é recalculado como `high_water - start`; o ledger v2 preserva o valor carregado.

Se um filho desaparece entre `readdir` e `lstat` (rename normal), há até três tentativas; se continuar ausente, a entrada é pulada sem crédito e sem derrubar a medição. Raiz obrigatória ausente ou `readdir` da raiz com `ENOENT` falha fechado; `EACCES`/erros de I/O falham fechado. Link/reparsepoint nunca é seguido.

A diminuição externa do `target` em relação ao baseline antigo é registrada uma vez no ledger (`external_decrease_bytes`), com aproximação e incerteza preservadas. Ela não cria um novo cap de 2 GiB e não apaga o histórico. Crescimento posterior após a queda conta integralmente.

## Execução e parada

1. Adquire lock exclusivo ao lado do ledger.
2. Valida baseline, schema, caminhos, timestamps, cap, piso, reserva e segmentos.
3. Mede as raízes, aplica a cobrança cumulativa e confere todos os volumes; se `used + reserva > cap` ou o piso falhar, para **antes** do filho.
4. Marca o ledger como `running`, reserva 256 MiB e inicia um único filho.
5. Amostra espaço livre e raízes em intervalos separados. O padrão é 1 s para livre e 5 s para raízes; a medição do target pode ser mais lenta que o intervalo e não é prometida como tempo real.
6. Ao aproximar do cap/piso, timeout, sinal, erro de medição ou falha de ledger, encerra **apenas o filho criado pelo wrapper** e sua árvore.
7. Mede o estado final, grava o ledger atomicamente e só então libera o lock. Falha de persistência mantém o lock e retorna erro; não há “ok” sem prova final.

No Windows, a parada da árvore usa `taskkill /PID <pid-do-próprio-filho> /T /F`, sem `/IM` e sem nome de processo. Em POSIX, usa o grupo do filho. O wrapper não procura, não sinaliza e não encerra Cargo, rustc, Postgres, Docker ou qualquer processo alheio. A posse é o filho Node/PID registrado enquanto vivo; existe uma limitação residual de reutilização de PID pelo SO. Não há Job Object nem promessa de garantia absoluta nesta versão.

## Códigos de saída

- `0`: comando terminou com 0 e o ledger final foi gravado.
- código do filho: comando terminou diferente de zero e o ledger foi gravado; `status=command-failed`.
- `3`: bloqueio de orçamento — piso, cap ou timeout; `status=blocked`.
- `130`: cancelamento por `SIGINT`/`SIGTERM` ou parada solicitada; `status=cancelled`.
- `4`: erro de configuração, medição, baseline, lock, ledger ou impossibilidade de provar o estado final.

O resumo final em `stderr` contém status, código, `usedBytes`, `remainingBytes`, cap, piso, reserva, diminuição externa aproximada e IDs de execução. Não contém `argv`, caminho de perfil, URL de banco, chave ou conteúdo de arquivo.

## Limitações obrigatórias

- **Não é uma cota de filesystem.** Um processo pode escrever entre duas amostras e ultrapassar o alvo antes da leitura seguinte. A reserva de 256 MiB e os intervalos reduzem a janela, mas não eliminam TOCTOU.
- Se o sistema operacional não oferecer quota real, a garantia é de **melhor esforço amostrado com parada fail-closed**, não de limite absoluto. O plano deve usar pacotes pequenos e comandos curtos; não prometer bloqueio físico.
- Caches não declarados, downloads, instalações e processos externos ficam fora do ledger. O comando deve ser offline quando o cache não for declarado.
- O ledger mede bytes de arquivos, não clusters, blocos compactados, metadados do filesystem ou espaço alocado; a diferença é uma limitação de medição.
- O ledger não é backup nem prova pública. Não o commitar, não o apagar para “resetar” e não usar uma nova medição para renovar o cap.
- O baseline antigo permanece fixo; qualquer migração de baseline ou política exige decisão e revisão humana separadas.
- O wrapper não inicia Postgres, Docker, Cargo ou serviço. `pg`/`--volume` só medem um caminho já existente.

## Próxima execução

Antes de Cargo, build, Docker ou teste de integração, o dono precisa aprovar o comando, o destino e a estimativa. O `-j 2` do README é apenas registro histórico; qualquer execução futura deve usar `-j 1` e o orçamento aprovado. O wrapper não transforma aprovação de orçamento em aprovação de build.

## Testes

`node --test scripts/run-with-disk-budget.test.mjs` cobre 23 testes: ledger ausente/malformado/forjado, baseline inválido, cobrança inicial, cap e piso antes do filho, execução serial sem reset, soma de raízes, target não duplicado no repo, diminuição externa sem crédito, recrescimento cumulativo (100→120→80→100 = 40), uma raiz não cancelando outra, `ENOENT` de filho em rename e raiz perdida, volume de saga sem piso, migração v1→v2 carregando uso, timeout/parada do próprio filho, processo externo preservado, código não zero, lock, path duplicado, symlink/reparsepoint e falha de medição. Os testes usam filhos Node pequenos e limites injetados; não executam Cargo, Docker, banco ou container.
