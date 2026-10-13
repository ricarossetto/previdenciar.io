# Adaptacoes Amicro

Fontes oficiais: tools/amicro/upstream/registry/ui/entrance/fade-up.tsx, registry/ui/hover/tilt-card.tsx e registry/lib/presets.ts. Licenca MIT preservada nesta pasta.

- Usa motion/react de motion@12.42.2; upstream alterna motion/react e framer-motion. Nao instalamos dois motores de animacao.
- FadeUp: spring, distancias menores e opcao de movimento reduzido.
- TiltCard: inclinacao limitada a 2 graus, apenas com mouse; nenhum efeito sobre tabelas, formularios ou dados financeiros.
- NavIndicator: spring compartilhado entre paginas, sem camada decorativa quando o movimento e reduzido. aria-current permanece no botao real.
- PageTransition: chave somente por rota, sem animar novamente durante digitacao ou alteracao de dados.
- Modal: conserva a pilha inert, captura de Tab, Escape e restauracao de foco existentes.
- DashboardOverview: contagens derivadas de registros locais ativos; nenhum grafico ou resultado financeiro sintetico.
- Botao Reduzir animacoes: preferencia local opcional, sem alterar schema de negocio e sem sobrepor a preferencia de acessibilidade do sistema.

Acervo completo em tools/amicro/upstream. Componentes e exemplos nao utilizados ficam fora do bundle do aplicativo. Sem importacao do backend, analytics, pagamentos ou servicos de IA do site.
