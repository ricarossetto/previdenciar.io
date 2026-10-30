# Arquitetura

## Camadas

- apps/desktop/src: React/TypeScript, apresentação, entrevista, editor e centro de relatórios.
- apps/desktop/src/api.ts: fronteira IPC Tauri; não é um backend HTTP público.
- apps/desktop/src/model.ts: contratos do workspace, origem e preparação de dados.
- apps/desktop/src-tauri: comandos assíncronos, validação de payload completo, IO controlado e diálogos locais.
- crates/previdenciar-cnis: extração/parsing de PDF textual e estruturas de origem.
- crates/previdenciar-workspace: modelo persistente (schema 3), contratos de evidência/leitura, revisão concorrente e cifra DPAPI no Windows.
- crates/previdenciar-core: funções de domínio, datas e finanças experimentais.
- crates/rgps-core: motor computacional observacional com limitações explícitas.
- crates/previdenciar-rgps: adaptador JSON, política explícita e guardas antes do motor.
- crates/previdenciar-reports: PDFs reais multipágina, privacidade e formatos específicos.
- crates/previdenciar-ai: catálogo de provedores, adaptadores OpenAI-compatible/Anthropic, cofre DPAPI de chaves, loop de ferramentas e leitura somente leitura da base local do Copiloto.
- apps/desktop/src-tauri/src/ai.rs: fronteira Tauri do Copiloto, busca/leitura direta da base de conhecimento, importação com extração de PDF, canal de streaming, cancelamento cooperativo e caminhos fixos do cofre e da base local.
- apps/desktop/src/components/KnowledgePage.tsx e MinutaPage.tsx: consulta local em blocos e fluxo de roteiro/redação/revisão, respectivamente.

## Invariantes

Valores monetários não devem depender de ponto flutuante arbitrário. Origem importada é preservada. Dados ausentes não viram zero. Respostas desconhecidas não são inferidas de outros campos. Resultado bloqueado não é uma RMI válida. O backend recompõe a hipótese para gerar PDF; JSON de resultado salvo não substitui cálculo e renderer.

Alterações de dados invalidam o resultado. A criação de um caso exige gravação explícita; uma entrevista assistida já salva tem autosave local com debounce para preservar a retomada. Gravação requer workspace completo e revisão correta. A leitura não autoriza sobrescrever um armazenamento inválido. Preview e exportação usam a mesma fronteira do renderer.

A PBC é reconciliada em quatro camadas distintas: lançamentos lidos do CNIS, entrada revisada enviada ao motor, competências excluídas antes de 07/1994 ou abaixo do mínimo e competências efetivamente usadas. A tabela de cenários de corte é descritiva e não escolhe regra, coeficiente, fator, tempo ou carência.

O Copiloto é uma fronteira separada do workspace. `previdenciar-ai` não recebe o workspace inteiro, não grava no `.pvr` e não calcula benefício; pode receber um resumo estrutural de caso somente quando ele é selecionado e marcado na interface. O envio ao provedor exige `consent=true` no pedido; o histórico é limitado e mantido só em memória. A chave fica em um cofre DPAPI fora do workspace. As ferramentas locais são somente leitura, aceitam apenas IDs de índice e recusam caminhos absolutos ou travessia. Conteúdo de documento é dado não confiável, não instrução de sistema.

## Evidência e leitura

`source_documents`, `readings` e `facts` guardam proveniência de documento, página e trecho. O cálculo só aceita leitura confirmada e fato inferido confirmado. Contratos e limites estão em [Contratos de evidência](DOMAIN_CONTRACTS.md).

## Compatibilidade

A marca pública mudou, mas os nomes internos de crates, o identificador Tauri e o diretório de armazenamento legado não foram renomeados. Isso evita uma migração de dados implícita. Qualquer migração futura precisa de projeto e testes próprios.

## Interface e testes

Animações são uma camada de apresentação, com movimento reduzido e foco de teclado. Testes de navegador usam fixtures IPC sintéticas. Testes Rust e smoke nativo têm escopos separados; sucesso de um não certifica automaticamente o outro.

## Livro de contribuições

Calculation.contribution_review é um overlay opcional. O contrato usa valores canônicos em centavos, referências à origem e diffs auditáveis. O frontend aplica explicitamente a projeção em manual_salaries; o Rust verifica origem, histórico e igualdade da projeção antes do cálculo e da persistência. Nenhum lançamento CNIS é sobrescrito. O renderer nativo mantém o livro e respeita a ocultação de motivos/justificativas livres.
