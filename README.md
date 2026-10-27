<p align="center"><strong>previdenciar.io</strong></p>
<h1 align="center">Cálculos previdenciários, às claras.</h1>
<p align="center">Ferramenta desktop gratuita e open source para organizar CNIS, hipóteses de cálculo e relatórios locais.</p>
<p align="center"><a href="https://github.com/ricarossetto/previdenciar.io/releases">Baixar para Windows</a> · <a href="https://ricarossetto.github.io/previdenciar.io/">Apresentação</a> · <a href="docs/GETTING_STARTED.md">Começar</a> · <a href="docs/LIMITATIONS.md">Limitações</a></p>

> **Versão 0.12.1.** O motor continua experimental. O programa não é serviço oficial do INSS, não certifica direito a benefício e não substitui revisão profissional. Hipótese bloqueada continua bloqueada, e dado ausente não vira zero.

## O que já existe

- Cadastro local de clientes, cálculos, agenda e modelos de texto.
- Entrevista em sete etapas que organiza a hipótese, guarda cada resposta e permite voltar a qualquer ponto. A conferência abre com as respostas da entrevista e, logo abaixo, o preparo a partir do CNIS em um clique: a fonte não entra no motor, a cópia revisada entra, e a referência da conferência fica registrada. O quadro de rastreabilidade mostra o que veio do CNIS, o que foi ajustado e o que o motor usou.
- Importação de CNIS em PDF com texto extraível. Lê a tabela de vínculos, a de remunerações, a de contribuições (competência, data de pagamento, contribuição e salário), as linhas de cooperativa e a de valores consolidados por ano civil. A tabela que continua na página seguinte sem repetir o cabeçalho é lida com o contexto herdado.
- Editor de períodos e livro de contribuições rastreável. Alterar um dado invalida o resultado e a revisão anterior.
- Motor RGPS experimental com adaptador restritivo, avisos e memória de cálculo.
- Cenário PcD de grau constante pela LC 142/2013 com premissas declaradas. Consulta os índices e limites públicos da Fábrica de Cálculos/TRF3 e, quando a rede não responde, usa um snapshot local com as séries oficiais embutidas (INPC, IGP-DI e tetos), com a fonte declarada no arquivo.
- Oito relatórios PDF gerados pelo renderer Rust: completo, resumo, tempo de contribuição, remunerações, memória da RMI, regras de transição, pendências e indicadores, e cálculo da qualidade de segurado.
- Apuração da qualidade de segurado por vínculo, com a regra e a fonte impressas no documento: período de graça de 12 meses (art. 15, II, da Lei 8.213/1991), prorrogação para até 24 meses quando houve mais de 120 contribuições (art. 15, § 1º), carência de 180 contribuições (art. 25, II) ou a faixa da tabela do art. 142, contagem do contribuinte individual a partir do primeiro pagamento em dia (art. 27, II), tempo em gozo de benefício fora da carência (Decreto 3.048/1999, art. 19-C, § 1º) e competência contada uma única vez quando aparece em mais de um vínculo.
- Central de relatórios com título editável, opções de apresentação (anexo técnico, conferência de qualidade, premissas de planejamento) e exportação em lote para uma pasta, sem sobrescrever arquivo existente.
- Persistência local cifrada no Windows, gravação explícita e proteção contra sobrescrita depois de erro ou corrupção.
- Interface React/Tauri com temas claro e escuro, navegação móvel e opção de movimento reduzido.

## O que foi conferido

- 486 testes Rust, 108 testes de interface e 36 testes de navegador.
- Verificação nativa em perfil isolado: importação, cálculo, as oito prévias de PDF, gravação cifrada e reabertura, sem tocar na base normal.
- Revisão de fonte pública sem achados.
- Um CNIS real de 23 páginas, conferido fora do repositório, passou a ser lido com 610 lançamentos e nenhuma diferença de salário por competência contra o relatório de um software de referência.
- Com o snapshot público da Fábrica de Cálculos, o mesmo caso chegou a R$ 5.161,10 contra R$ 5.200,89 do software de referência, uma diferença de 0,77%. Ela vem de sete competências que o CNIS lista apenas na visão "Contribuições" e que este programa mantém no período básico de cálculo. A escolha entre as duas leituras é profissional e está documentada.

## O que ainda não está pronto

O livro de contribuições está implementado: edição de competência e valor, moeda e vínculo declarados, filtros, ações em lote, consolidação por competência, exclusão documentada sem apagar o CNIS, histórico, desfazer e aplicação explícita aos salários do cálculo. A validação em Rust recusa origem, histórico ou projeção inconsistentes. Isso não valida regime, CTC ou direito a benefício. Consulte [o guia do editor](docs/CONTRIBUTIONS.md).

Não estão implementados: a carência legal por tabelas do RGPS, a desconsideração por afastamento como o software de referência a faz (as fórmulas dele não são recuperáveis do binário, e nada foi inferido no lugar) e os módulos de planejamento (contribuição, frequência, alíquota, IRRF, retorno). O programa também não atualiza sozinho toda regra ou índice oficial, não cobre todos os regimes e benefícios, não é sistema de tramitação judicial e não tem backend web público. Consulte [limitações](docs/LIMITATIONS.md) e [roadmap](docs/ROADMAP.md).

## Baixar e executar

1. Abra [Releases](https://github.com/ricarossetto/previdenciar.io/releases) e escolha a versão 0.12.1.
2. Baixe o instalador `previdenciar.io_0.12.1_x64-setup.exe` (recomendado, cria atalhos e verifica atualizações) ou o executável portátil, junto com o `SHA256SUMS.txt`.
3. Confira o SHA-256 antes de executar. A distribuição não possui assinatura digital de código; o instalador é assinado apenas para o atualizador do próprio aplicativo.
4. Requer Windows 10/11 x64 com Microsoft Edge WebView2. Não desative proteções do Windows para ignorar alertas. Se não confiar no binário, compile a partir do código.
5. Use primeiro dados fictícios e leia os avisos. Não compartilhe CNIS, PDFs ou arquivos da base em issues.

A versão pública usa **identidade textual provisória**. Logo e identidade final serão produzidas no OpenDesign e integradas depois.

## Privacidade

Importação, cálculo, PDFs e exportação ocorrem na máquina do operador. Os fluxos implementados não enviam documentos a servidor nenhum. A consulta opcional de índices e limites públicos envia apenas a competência pedida à Fábrica de Cálculos/TRF3, sem CNIS, identidade, salários ou dados de saúde. O aplicativo não recebe chave de API: usa as rotas públicas de índices e referenciais.

A base protegida do Windows fica no caminho legado de compatibilidade, sem migração silenciosa. Exportar com identificação oculta **não garante anonimização**, porque datas e valores ainda permitem correlação. A página no GitHub Pages é apenas apresentação, documentação e download: não recebe CNIS nem executa o motor Rust. Leia [privacidade](docs/PRIVACY.md).

## Compilar

Pré-requisitos: Rust estável, Microsoft C++ Build Tools, WebView2 e Node.js 24 com npm. O desktop desta distribuição tem foco no Windows.

~~~powershell
cd apps/desktop
npm ci
npm run build
npm test
cd ../..
cargo test --workspace --locked
cargo build --release --locked -p previus-next-desktop --bin previus-next-desktop
~~~

O binário é gerado em target/release/previus-next-desktop.exe. Os nomes internos dos crates e o identificador Tauri ficam como estão por compatibilidade. A marca exibida é previdenciar.io.

Para os testes de navegador, dentro de apps/desktop:

~~~powershell
npx playwright install chromium
npm run test:e2e
~~~

Esses testes usam IPC sintético em memória e não cobrem o transporte nativo do Tauri, o diálogo de exportação ou a conformidade jurídica. A suíte Rust cobre separadamente o parsing, o armazenamento, as guardas, o cálculo e a geração dos PDFs sintéticos.

## Documentação

- [Primeiros passos](docs/GETTING_STARTED.md)
- [Arquitetura e contratos](docs/ARCHITECTURE.md)
- [Privacidade](docs/PRIVACY.md)
- [Limitações](docs/LIMITATIONS.md)
- [Editor de contribuições](docs/CONTRIBUTIONS.md)
- [Cenários PcD da LC 142](docs/LC142_SCENARIOS.md)
- [Roadmap](docs/ROADMAP.md)
- [Build, release e deploy](docs/RELEASING.md)
- [Origem do código](docs/PROVENANCE.md) e [atribuições](NOTICE.md)
- [Prompt para logo e identidade no OpenDesign](brand/OPENDESIGN_PROMPT.md)
- [Contribuir](CONTRIBUTING.md) · [segurança](SECURITY.md)

## Licença e independência

Código autoral sob [MIT](LICENSE). As licenças dos componentes e das fontes de terceiros continuam válidas. O uso é gratuito, sem assinatura exigida pelo aplicativo, e a licença MIT também permite uso comercial por terceiros, preservadas as condições.

Projeto independente, sem vínculo oficial com o INSS nem com aplicações proprietárias usadas como referência de comportamento. O nome não comprova registro de domínio ou de marca. Não distribuímos software proprietário, dumps, bases, documentos de clientes nem o histórico privado de desenvolvimento.
