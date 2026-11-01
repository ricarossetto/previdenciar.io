# Longrun — fila de correções e validações

## Publicado
- [x] Task `previdenciar-web:9`.
- [x] Google: aceitar token sem `email_verified` explícito.
- [x] Mobile: login e termos com rolagem.
- [x] Cloudflare: purge total.

## Próximas execuções
1. Reproduzir login Google em produção com conta real e registrar claims do ID token.
2. Se o loop continuar, validar client secret do Google IdP e callback no Cognito.
3. Implementar OIDC no Atrium (`/oauth/authorize`, `/oauth/token`, JWKS) ou SSO assinado.
4. Cadastrar Atrium como IdP no Cognito e mapear `sub`/e-mail.
5. Aumentar upload CNIS/PDF para 32/64 MiB e tratar 413 como JSON.
6. Mover OCR, cálculo e PDF para `spawn_blocking` com semáforo.
7. Adicionar CSP, HSTS, nosniff, Referrer-Policy e `no-store` em `/api/*`.
8. Aplicar rate limit em auth, CNIS, cálculo, ferramentas e lote.
9. Corrigir `pdfs.html` e integrar PDFs salvos ao centro de relatórios.
10. Exibir classificação e confirmação da Minuta antes da redação.
11. Adicionar teste de auth, admin, S3 e SSE no servidor.
12. Configurar RDS com 7 dias de backup ou documentar a limitação free tier.
13. Criar staging isolado com Cognito/RDS/S3 próprios.
14. Criar GitHub Actions de deploy com OIDC e aprovação manual.
15. Configurar alarmes CloudWatch por e-mail.
16. Remover ou congelar definitivamente a EC2 antiga.
17. Revisar mojibake de prompts e mensagens antigas.
18. Rodar crítica independente novamente e repetir até zerar achados críticos/altos.

## Resultado da validação TRF3

- [x] Chamada oficial ao endpoint `/rgps/api/v1/calculos-rmi`.
- [x] Caso sintético 42: oficial R$ 4.067,11 / SB R$ 5.083,89.
- [x] Local nominal R$ 1.600,00 / SB R$ 2.000,00.
- [x] Local corrigido R$ 3.779,69 / SB R$ 4.724,61.
- [ ] Equivalência: **não**. Divergência de 7,07%; motor local usa INPC fixo, API usa IPC-R/INPC/IGP-DI/INPC.
- [ ] Implementar perfil oficial versionado e validar SB/RMI em múltiplos casos.

## Skills

- [x] `saga` instalada para orquestração autônoma com workers.
- [x] `cross-critique` instalada e usada na revisão da divergência.
- [ ] Rodar nova crítica independente após a correção do motor.
