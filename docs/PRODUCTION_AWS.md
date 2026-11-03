# Produção AWS — previdenciar.io

Última atualização: 31/10/2026. Revisão de segurança e redação: 02/11/2026.

Este documento descreve a arquitetura de produção do **produto web oficial** (https://previdenciario.atrium.adv.br). Não descreve o desktop local, que está congelado como legado (veja [DESKTOP_FROZEN.md](DESKTOP_FROZEN.md)).

> **Sobre os identificadores.** Todos os identificadores de infraestrutura foram substituídos por marcadores (`<CONTA_AWS>`, `<POOL_COGNITO>`, `<SUBNET_A>` e assim por diante). O documento continua explicando a arquitetura, mas não serve como inventário de recursos. Consulte os valores reais no console AWS, na conta de produção, com as permissões adequadas. Não grave identificadores, ARNs, chaves ou tokens neste arquivo nem em qualquer arquivo do repositório.

## Arquitetura em produção

- Cloudflare Tunnel `previdenciario-aws` (identificador `<TUNEL_ID>`) recebe o domínio `previdenciario.atrium.adv.br`.
- O túnel encaminha para o ALB `<ALB_DNS>` na região `sa-east-1`.
- ALB `previdenciar-prod` → target group `previdenciar-tg` → ECS Fargate.
- ECS cluster `previdenciar-prod`; service `previdenciar-prod-web`; task definition `previdenciar-web:<REVISAO_TASK>`.
- Imagem: `<ECR_REGISTRY>/previdenciar-app:prod-AAAAMMDD-N`, no ECR da conta `<CONTA_AWS>`.
- RDS PostgreSQL `previdenciar-prod-db`, engine 18.3, classe `db.t4g.micro`, sem acesso público, backup automático de 1 dia.
- S3 privado `<BUCKET_FILES>`; criptografia AES256; ciclo de vida para o prefixo `cnis/` de 1 dia e para `reports/` e `exports/` de 30 dias (detalhes em [Retenção de arquivos no S3](#retenção-de-arquivos-no-s3)).
- Cognito: pool `<POOL_COGNITO>`; app client `<CLIENT_ID_COGNITO>`; domínio `<DOMINIO_COGNITO>`.
- Login com Google e com e-mail/senha; cadastro aberto com confirmação de e-mail; MFA TOTP opcional.
- IA/CommandCode: `https://api.commandcode.ai/provider/v1`, modelo `deepseek/deepseek-v4-flash`.

## Segredos

Não registre valores neste arquivo. Os segredos ficam no Secrets Manager, com estes nomes:

- `previdenciar/prod/database-url` — JSON com `DATABASE_URL`.
- `previdenciar/prod/ai-key` — chave da plataforma CommandCode.
- `previdenciar/prod/fabrica-key`, `previdenciar/prod/app-password` e `previdenciar/prod/tunnel-token` — legado do site antigo.
- Credenciais da RDS são gerenciadas pelo próprio Secrets Manager (`rds!db-...`).

A task definition referencia os segredos por ARN; o valor nunca entra na imagem, no repositório ou no log da aplicação.

## Rede

- VPC default `<VPC_ID>`, em `sa-east-1`.
- Subnets públicas `<SUBNET_A>`, `<SUBNET_B>` e `<SUBNET_C>`.
- Security group do ALB: `<SG_ALB>` (entrada 443 a partir da borda do túnel).
- Security group do ECS: `<SG_ECS>` (entrada apenas do ALB, na porta do serviço).
- Security group da RDS: `<SG_RDS>` (entrada apenas do ECS, na porta 5432).
- A RDS e o bucket S3 não têm exposição pública; o acesso aos arquivos passa pela aplicação autenticada.

## Publicar uma nova versão

```powershell
# 1. Build do frontend web
npm --prefix apps/desktop run build:web

# 2. Login no ECR e build da imagem
$region = 'sa-east-1'
$account = '<CONTA_AWS>'
$repo = "$account.dkr.ecr.$region.amazonaws.com/previdenciar-app"
$tag = 'prod-AAAAMMDD-HHMM'
aws ecr get-login-password --region $region |
  docker login --username AWS --password-stdin "$account.dkr.ecr.$region.amazonaws.com"
docker build -f apps/server/Dockerfile -t "${repo}:${tag}" .
docker push "${repo}:${tag}"

# 3. Registrar nova task definition com a tag da imagem e forçar novo deployment
aws ecs update-service --cluster previdenciar-prod --service previdenciar-prod-web --force-new-deployment
```

A task definition precisa ser registrada com a nova tag de imagem antes do `update-service` quando houver mudança de código. Sem isso, o service sobe de novo a imagem anterior.

## Verificação

```powershell
Invoke-RestMethod https://previdenciario.atrium.adv.br/api/health
```

Esperado:

```json
{"service":"previdenciar-web-server","status":"ok","version":"0.1.0"}
```

Depois testar no navegador:

1. login com Google;
2. login com e-mail/senha;
3. cadastro com confirmação de e-mail;
4. criar cliente e cálculo;
5. importar CNIS com PDF nativo e escaneado;
6. gerar PDF;
7. Copiloto com consentimento.

## Rollback

O site antigo na EC2 `<EC2_LEGADO>` continua existindo, mas o túnel já aponta para o ALB. Para voltar temporariamente, o ingress do túnel pode ser reconfigurado para a instância:

```powershell
# Exemplo: apontar o ingress do túnel de volta para a EC2
# http://127.0.0.1:8787
```

O dono do produto decidiu não manter rollback de produto: a EC2 fica apenas como emergência operacional até a limpeza. Reverter o túnel não restaura dados gravados no PostgreSQL novo.

## Retenção de arquivos no S3

O bucket `<BUCKET_FILES>` é privado e usa criptografia AES256. O ciclo de vida atual é:

| Prefixo | Expiração | Efeito prático |
| --- | --- | --- |
| `cnis/` | 1 dia | O arquivo enviado é apagado pelo S3 no dia seguinte ao upload. |
| `reports/` | 30 dias | PDFs gerados ficam disponíveis por 30 dias. |
| `exports/` | 30 dias | Exportações ficam disponíveis por 30 dias. |

O expurgo é feito pela regra de ciclo de vida do S3, não pela aplicação. Objetos já excluídos não são recuperáveis; versões antigas do objeto, quando existirem, seguem a mesma regra.

### Estado da decisão

- **Data do registro:** 02/11/2026.
- **Situação:** aguardando decisão do dono do produto. Este documento descreve o comportamento atual; não aprova a política.
- **Quem decide:** dono do produto, com apoio jurídico e de segurança. A engenharia não altera a regra de ciclo de vida por conta própria.

### Trade-off à luz da LGPD

A escolha entre 1 dia e um prazo maior envolve duas obrigações que apontam em direções opostas:

- **Minimização e necessidade (art. 6º, III, e art. 15 da LGPD).** O CNIS é dado pessoal e pode conter dado sensível de saúde (art. 5º, II). Manter o arquivo só enquanto ele é necessário para a importação reduz a superfície de vazamento e o custo de um incidente. O prazo de 1 dia atende bem a esse lado: o CNIS é lido, convertido em registros revisados e o original deixa de ser necessário.
- **Retenção para prova em disputa.** Se o titular ou o profissional precisar demonstrar o que foi importado e quando, um arquivo apagado em 24 horas não serve como prova. Nesse cenário, a retenção maior reduz o risco de litígio sobre a fidelidade da importação, mas aumenta o volume de dados pessoais sob guarda e o impacto de um vazamento.

Equilíbrios possíveis, para discussão, sem decisão tomada:

1. manter o expurgo em 1 dia e provar a importação por hash e metadados do arquivo, sem guardar o conteúdo;
2. separar o original do CNIS do resultado revisado, retendo apenas o resultado e o hash;
3. ampliar o prazo do prefixo `cnis/` com base legal e prazo documentados, e registrar a finalidade no aviso de privacidade.

Qualquer mudança no ciclo de vida exige atualizar também o aviso de privacidade do produto e este documento. Enquanto a decisão não existe, vale o default de 1 dia.

Fontes internas: [PRIVACY.md](PRIVACY.md) (armazenamento local, importação e exportação), [LIMITATIONS.md](LIMITATIONS.md) (o CNIS importado é cópia de trabalho, não altera a origem) e este documento, seção de arquitetura.

## Nota de segurança sobre este arquivo

Até 02/11/2026 este documento continha identificadores reais: conta AWS, pool e app client Cognito, domínio Cognito, subnets, security groups, VPC, DNS do ALB, bucket, instância EC2 e identificador do túnel. A revisão substituiu todos por marcadores.

Verificação feita em 02/11/2026: a listagem de `docs/PRODUCTION_AWS.md` em `origin/main` não retorna nada, e a busca pelo ID da conta nos três branches remotos (`origin/main`, `origin/cline/8d01e`, `origin/fix/ec103-especial-professor-20261029`) também não retorna ocorrência. O arquivo, portanto, nunca chegou ao repositório remoto, e `git log --oneline -- docs/PRODUCTION_AWS.md` lista apenas commits locais (`c050a32`, `3b16929`, `320c408`), nenhum deles presente naqueles branches.

Ainda assim, os identificadores existem no histórico local do Git e em outros arquivos do working tree. Trate-os como conhecidos. Antes de publicar qualquer branch que carregue esse histórico, avalie a rotação dos segredos associados (túnel, `app-password`, `fabrica-key`, `ai-key`) e a limpeza do histórico, se for o caso. Identificadores que já aparecem no cliente web publicado, como o domínio Cognito, não podem ser removidos sem quebrar o login.
