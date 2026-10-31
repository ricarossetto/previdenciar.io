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
