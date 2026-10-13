# Arquitetura

## Camadas

- apps/desktop/src: React/TypeScript, apresentação, entrevista, editor e centro de relatórios.
- apps/desktop/src/api.ts: fronteira IPC Tauri; não é um backend HTTP público.
- apps/desktop/src/model.ts: contratos do workspace, origem e preparação de dados.
- apps/desktop/src-tauri: comandos assíncronos, validação de payload completo, IO controlado e diálogos locais.
- crates/previus-cnis: extração/parsing de PDF textual e estruturas de origem.
- crates/previus-workspace: modelo persistente, revisão concorrente e cifra DPAPI no Windows.
- crates/previus-core: funções de domínio, datas e finanças experimentais.
- crates/rgps-core: motor computacional observacional com limitações explícitas.
- crates/previus-rgps: adaptador JSON, política explícita e guardas antes do motor.
- crates/previus-reports: PDFs reais multipágina, privacidade e formatos específicos.

## Invariantes

Valores monetários não devem depender de ponto flutuante arbitrário. Origem importada é preservada. Dados ausentes não viram zero. Respostas desconhecidas não são inferidas de outros campos. Resultado bloqueado não é uma RMI válida. O backend recompõe a hipótese para gerar PDF; JSON de resultado salvo não substitui cálculo e renderer.

Alterações de dados invalidam o resultado. Gravação requer workspace completo e revisão correta. A leitura não autoriza sobrescrever um armazenamento inválido. Preview e exportação usam a mesma fronteira do renderer.

## Compatibilidade

A marca pública mudou, mas os nomes internos de crates, o identificador Tauri e o diretório de armazenamento legado não foram renomeados. Isso evita uma migração de dados implícita. Qualquer migração futura precisa de projeto e testes próprios.

## Interface e testes

Animações são uma camada de apresentação, com movimento reduzido e foco de teclado. Testes de navegador usam fixtures IPC sintéticas. Testes Rust e smoke nativo têm escopos separados; sucesso de um não certifica automaticamente o outro.
