# ProduÃ§Ã£o AWS â€” previdenciÃ¡rio

Ãšltima atualizaÃ§Ã£o: 31/10/2026

## Arquitetura em produÃ§Ã£o

- Cloudflare Tunnel `previdenciario-aws` (`30282d26-e7c5-45de-acbe-77a9dab71833`) recebe `previdenciario.atrium.adv.br`.
- O tÃºnel encaminha para o ALB `previdenciar-prod-326171366.sa-east-1.elb.amazonaws.com`.
- ALB `previdenciar-prod` â†’ target group `previdenciar-tg` â†’ ECS Fargate.
- ECS cluster `previdenciar-prod`; service `previdenciar-prod-web`; task definition `previdenciar-web:5`.
- Imagem: `557789254125.dkr.ecr.sa-east-1.amazonaws.com/previdenciar-app:prod-20261031-5`.
- RDS PostgreSQL `previdenciar-prod-db`, engine 18.3, `db.t4g.micro`, privado, backups de 1 dia.
- S3 privado: `previdenciar-prod-files-557789254125`; AES256; ciclo de vida para `cnis/` 1 dia e `reports/`, `exports/` 30 dias.
- Cognito: pool `sa-east-1_K3cfFB6GC`; app client `4vla5ttd8j59jqj5oviviaugsj`; domÃ­nio `previdenciar-auth-557789254125.auth.sa-east-1.amazoncognito.com`.
- Login Google + e-mail/senha; cadastro aberto com confirmaÃ§Ã£o de e-mail; MFA TOTP opcional.
- AI/CommandCode: `https://api.commandcode.ai/provider/v1`, modelo `deepseek/deepseek-v4-flash`.

## Segredos

NÃ£o registre valores neste arquivo.

- `previdenciar/prod/database-url` â€” JSON com `DATABASE_URL`.
- `previdenciar/prod/ai-key` â€” chave da plataforma CommandCode.
- `previdenciar/prod/fabrica-key`, `app-password`, `tunnel-token` â€” legado do site antigo.
- Credenciais da RDS gerenciadas pelo Secrets Manager (`rds!db-...`).

## Rede

- VPC default `vpc-00328a0cc3ee6c071`.
- Subnets pÃºblicas: `subnet-03f1cdf03030f76d7`, `subnet-0a85e132d84158e9e`, `subnet-0d548fe3ba20c7afd`.
- ALB SG: `sg-05529d48e40f98c73`.
- ECS SG: `sg-0e8eeefb211b9be13`.
- RDS SG: `sg-0fdb1e4abbdbd6333`.

## Publicar uma nova versÃ£o

```powershell
# 1. Build do frontend
npm --prefix apps/desktop run build:web

# 2. Login no ECR e build da imagem
$region = 'sa-east-1'
$account = '557789254125'
$repo = "$account.dkr.ecr.$region.amazonaws.com/previdenciar-app"
$tag = 'prod-AAAAMMDD-HHMM'
aws ecr get-login-password --region $region |
  docker login --username AWS --password-stdin "$account.dkr.ecr.$region.amazonaws.com"
docker build -f apps/server/Dockerfile -t "${repo}:${tag}" .
docker push "${repo}:${tag}"

# 3. Registrar task definition com a nova imagem e atualizar o service
aws ecs update-service --cluster previdenciar-prod --service previdenciar-prod-web --force-new-deployment
```

A task definition precisa ser registrada com a nova tag de imagem antes do update do service quando houver mudanÃ§a de cÃ³digo.

## VerificaÃ§Ã£o

```powershell
Invoke-RestMethod https://previdenciario.atrium.adv.br/api/health
```

Esperado:

```json
{"service":"previdenciar-web-server","status":"ok","version":"0.1.0"}
```

Depois testar no navegador:

1. login Google;
2. login e-mail/senha;
3. cadastro com confirmaÃ§Ã£o;
4. criar cliente e cÃ¡lculo;
5. importar CNIS com PDF nativo e escaneado;
6. gerar PDF;
7. Copiloto com consentimento.

## Rollback

O site antigo na EC2 `i-0ac6b115b6a2868c5` continua existindo, mas o tÃºnel jÃ¡ aponta para o ALB. Para voltar temporariamente:

```powershell
# Exemplo: apontar o ingress do tÃºnel de volta para a EC2
# http://127.0.0.1:8787
```

O usuÃ¡rio decidiu nÃ£o manter rollback de produto; a EC2 fica apenas como emergÃªncia operacional atÃ© a limpeza.


