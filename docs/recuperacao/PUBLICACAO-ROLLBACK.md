# Candidato e rollback — estado atual (privado, sem deploy)

## Estado real do candidato

- Branch de trabalho: `wip/consolidacao-20261102-correcoes`.
- HEAD **`20134c4`**; candidato local **`candidato-20261104-03`**.
- Candidatos anteriores preservados: `candidato-20261104-01` e `-02`.
- O worktree antigo `atrium/run-18015c24b079/candidate` está em `fc79848` e **não** contém as correções recentes; não deve ser usado para publicar.
- O `main` principal não foi alterado, não houve merge, push, deploy, migração ou limpeza.

## Verificações desta fase

- Núcleo: 74/74.
- Adaptador RGPS: 72/72.
- Relatórios: 17/17.
- CNIS: 39/39.
- Frontend: 307/307 + 2 testes novos de normalização de texto.
- Typecheck: exit 0.
- Servidor local: SHA-256 `C697051349154EB8D748E8D2E1D3D6C24A819826FD9227F76F66FD990AB2FA43` (compilado antes das últimas mudanças de teste de UI).
- Caso real persistido na revisão 8; PDF final 1.177.259 bytes e PDF identificado 1.185.010 bytes.

## Pendências de homologação

1. Login real: página aberta em `http://127.0.0.1:8788/`, com banco descartável e Cognito; aguarda autenticação manual. Não extrair senha nem token.
2. Jornada completa: Cliente → CNIS → Conferência → Cálculo → Resultado → Relatório.
3. Confirmar PDFs, mensagens, salvamento e PDF identificado na sessão autenticada.
4. Revisão independente encontrou achados altos que precisam ser fechados antes de publicar: base mista do tempo no fator pós-reforma, convenção 30/360 não aditiva por vínculo, carência/benefício e meses abaixo do mínimo, `AmountKind` da coluna Filantropia, reparo persistido da fonte CNIS e a divergência documental do candidato antigo.

## Procedimento de publicação (NÃO EXECUTADO)

1. Fechar os achados altos e rodar as suítes afetadas.
2. Congelar a tag do candidato validado e gerar a imagem do servidor somente após build autorizado.
3. Registrar a revisão/task definition e o digest da imagem atualmente em produção (rollback point).
4. Publicar primeiro em homologação; validar login, CNIS, cálculo e PDF.
5. Promover a imagem; monitorar a rota real de saúde e os erros do serviço.
6. Rollback: reverter o serviço para a revisão/task definition anterior; não reverter banco sem migração explícita.

## Bloqueios

- Sessão autenticada para homologação real.
- Autorização específica para a limpeza restrita de PDBs/incremental (18,91 GB recuperáveis), sem tocar em binários, bancos, PDFs, CNIS ou trabalhos.
- Nenhum deploy, merge no principal, migração ou limpeza será feito sem autorização específica.
