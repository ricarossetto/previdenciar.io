# Arquitetura web

O site é a evolução do desktop, não uma segunda implementação. O React de `apps/desktop` é compilado em modo web e conversa com `apps/server`.

```text
React (apps/desktop, modo web)
  -> /api (REST/JSON)
  -> Rust/Axum (apps/server)
  -> crates do domínio: previdenciar-rgps, rgps-core, previdenciar-cnis, previdenciar-reports, previdenciar-ai
  -> PostgreSQL
  -> S3 (documentos e PDFs)
```

## Decisões

- Conta: Cognito com Google, e-mail/senha e MFA opcional.
- Workspace pessoal por padrão; equipes opcionais com papéis.
- Cadastro aberto com verificação; o módulo fica atrás de `previdenciario:advanced`.
- Dados de casos: PostgreSQL com revisão e trilha de auditoria.
- CNIS: upload para processamento; original descartado após extração.
- PDFs: retenção curta no S3; geração com o renderer Rust existente.
- IA: chave da plataforma no piloto e BYOK depois.
- Desktop: congelado; sem paridade paralela.

## Fases

1. Fundação: Axum, PostgreSQL, contrato REST, adaptador React.
2. Identidade: Cognito, sessão web, permissões.
3. Fluxo principal: clientes, casos, CNIS, leituras, cálculo e relatórios.
4. Produção de conteúdo: Minuta, Copiloto, ferramentas e Fábrica.
5. Deploy: ECS/Fargate, RDS, S3, ALB e troca do domínio.

## Modo local

Sem `AUTH_DISABLED`, a API exige JWT do Cognito. Em desenvolvimento:

```dotenv
AUTH_DISABLED=1
DATABASE_URL=postgres://postgres:dev@127.0.0.1:55432/previdenciar
WEB_DIST=apps/desktop/dist
```

Compile o front com `npm --prefix apps/desktop run build:web` e inicie `cargo run -p previdenciar-web-server`.
