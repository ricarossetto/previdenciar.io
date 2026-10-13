<p align="center"><strong>previdenciar.io</strong></p>
<h1 align="center">Cálculos previdenciários, às claras.</h1>
<p align="center">Ferramenta desktop gratuita e open source para organizar CNIS, hipóteses de cálculo e relatórios locais.</p>
<p align="center"><a href="https://github.com/ricarossetto/previdenciar.io/releases">Baixar para Windows</a> · <a href="https://ricarossetto.github.io/previdenciar.io/">Apresentação</a> · <a href="docs/GETTING_STARTED.md">Começar</a> · <a href="docs/LIMITATIONS.md">Limitações</a></p>

> **Versão experimental.** Não é um serviço oficial do INSS, não certifica direito a benefício e não substitui revisão profissional. Resultados bloqueados permanecem bloqueados; informações ausentes não são inventadas.

## O que já existe

- Cadastro local de clientes, cálculos, agenda e modelos de texto.
- Entrevista progressiva para organizar uma hipótese RGPS, salvar e retomar.
- Importação local de CNIS em PDF com texto extraível e conferência de origem, vínculos, competências e indicadores.
- Editor manual de períodos e salários, preparação do PBC para revisão e invalidação do resultado ao alterar dados.
- Motor RGPS experimental, com adaptador restritivo, avisos e memória nos cenários suportados.
- Ferramentas de datas, atualização de valores e fórmulas financeiras com parâmetros explícitos.
- Relatórios PDF reais: detalhado, resumo, períodos, salários, memória, transições e indicadores.
- Persistência local cifrada no Windows, gravação explícita e proteção contra sobrescrita após erro/corrupção.
- Interface React/Tauri, temas claro/escuro, navegação móvel e animações com opção de movimento reduzido.

## O que ainda não está pronto

**O editor completo de contribuições é prioridade da próxima etapa.** O editor manual já permite alterar competência, valor, indicadores e confirmação. Isso não equivale a um painel completo para editar/reconciliar cada contribuição importada do CNIS: não há ainda um fluxo completo de ajustes em lote, exclusão documentada, operações de concomitância e rastreamento de todas as alterações por lançamento.

O programa não faz atualização automática de toda regra/índice oficial, não calcula todos os regimes ou benefícios, não é um sistema de tramitação judicial e não tem um backend web público. Consulte [limitações](docs/LIMITATIONS.md) e [roadmap](docs/ROADMAP.md).

## Baixar e executar

1. Abra [Releases](https://github.com/ricarossetto/previdenciar.io/releases) e escolha a pré-release mais recente.
2. Baixe o executável Windows e o arquivo SHA256SUMS.txt.
3. Confira o SHA-256 antes de executar. A distribuição inicial não possui assinatura digital de código.
4. Requer Windows 10/11 x64 com Microsoft Edge WebView2. Nunca desative proteções do Windows para ignorar alertas; prefira compilar a partir do código se não confiar no binário.
5. Use primeiro dados fictícios e leia os avisos. Não compartilhe CNIS, PDFs ou arquivos da base em issues.

A primeira versão pública usa **identidade textual provisória**. Logo e identidade final serão produzidas no OpenDesign e integradas posteriormente.

## Privacidade

Os fluxos implementados não enviam documentos a servidores. Importação, cálculos e relatórios ocorrem localmente; a exportação é explícita. A base protegida do Windows é mantida no caminho legado de compatibilidade, sem migração silenciosa. Exportar com identificação oculta **não é garantia de anonimização**, pois datas e valores podem permitir correlação.

A página GitHub Pages é apenas apresentação/documentação/download: não recebe CNIS nem executa o motor Rust. Leia [privacidade](docs/PRIVACY.md).

## Compilar

Pré-requisitos: Rust estável, Microsoft C++ Build Tools, WebView2 e Node.js 24 com npm. O backend desktop desta distribuição tem foco no Windows.

~~~powershell
cd apps/desktop
npm ci
npm run build
npm test
cd ../..
cargo test --workspace --locked
cargo build --release --locked -p previus-next-desktop --bin previus-next-desktop
~~~

O binário é gerado em target/release/previus-next-desktop.exe. O nome interno dos crates e o identificador Tauri são mantidos para compatibilidade; a marca exibida é previdenciar.io.

Para testes de navegador, dentro de apps/desktop:

~~~powershell
npx playwright install chromium
npm run test:e2e
~~~

Esses testes usam IPC sintético em memória e não certificam o transporte nativo Tauri, o diálogo de exportação ou a conformidade jurídica. A suíte Rust testa separadamente parsing, armazenamento, guardas, cálculo e geração de PDFs sintéticos.

## Documentação

- [Primeiros passos](docs/GETTING_STARTED.md)
- [Arquitetura e contratos](docs/ARCHITECTURE.md)
- [Privacidade](docs/PRIVACY.md)
- [Limitações](docs/LIMITATIONS.md)
- [Roadmap](docs/ROADMAP.md)
- [Build, release e deploy](docs/RELEASING.md)
- [Origem do código](docs/PROVENANCE.md) e [atribuições](NOTICE.md)
- [Prompt para logo e identidade no OpenDesign](brand/OPENDESIGN_PROMPT.md)
- [Contribuir](CONTRIBUTING.md) · [segurança](SECURITY.md)

## Licença e independência

Código autoral sob [MIT](LICENSE); as licenças dos componentes e fontes de terceiros continuam válidas. Gratuito para baixar e usar, sem assinatura exigida pelo aplicativo. A licença MIT também permite usos comerciais de terceiros, preservadas suas condições.

Projeto independente: sem vínculo oficial com INSS ou com aplicações proprietárias utilizadas como referência comportamental. O nome não comprova registro de domínio ou de marca. Não distribuímos software proprietário, dumps, bases, documentos de clientes ou o histórico privado de desenvolvimento.
