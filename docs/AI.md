# Copiloto IA opcional

O Copiloto é uma integração **opcional** com provedores externos de modelos. Ele não substitui o motor de cálculo, não valida direito e não envia documentos automaticamente. O envio começa desativado e exige uma autorização explícita por sessão.

## O que o Copiloto faz

- Conversa em português sobre dúvidas jurídicas e de organização do trabalho.
- Consulta, por ferramentas locais somente leitura, os extratos importados de:
  - habilidades oficiais da base de conhecimento;
  - boletim de legislação federal;
  - metadados e texto extraído dos pareceres e-NatJus;
  - ementas de jurisprudência capturadas na auditoria.
- Cita os rótulos retornados (`H1`, `L2`, `N3`, `J4`) para que o operador confira a fonte.
- Mantém o histórico e o rascunho da pergunta apenas em memória; eles sobrevivem à navegação entre telas, mas limpar a conversa, fechar ou recarregar o aplicativo descarta o contexto.
- Permite anexar, opcionalmente, um resumo estruturado de um cálculo local, sem nome, CPF, NIT, salários, notas ou textos livres. O resumo inclui apenas campos estruturais do caso, como módulo, DIB, sexo e condição de deficiência; a marcação é explícita e por sessão.

## O que ele não faz

- Não anexa automaticamente CNIS, clientes, cálculos, PDFs ou o workspace inteiro. O resumo estruturado de um caso só entra se for selecionado e marcado explicitamente.
- Não calcula RMI, tempo de contribuição, carência, elegibilidade ou benefício.
- Não garante vigência, inteiro teor, aplicação ao caso ou resultado.
- Não substitui revisão profissional nem a conferência das fontes oficiais.
- Usa o texto extraído dos PDFs NatJus quando a importação inclui os PDFs; se a extração falhar, mantém os metadados e a ficha local.
- Não salva a conversa no workspace.

## Provedores

O catálogo é apenas uma conveniência. A URL base e o identificador do modelo são editáveis, porque catálogos e planos mudam. Use **Descobrir modelos** para consultar o endpoint `/models` quando o provedor oferecer essa rota.

| Preset | Formato | Base | Modelo inicial | Observação |
|---|---|---|---|---|
| OpenCode Go | OpenAI-compatible | `https://opencode.ai/inference/go/openai/v1` | `deepseek-v4.1-flash` | Preset observado no app operacional de referência. Confirme a autorização de uso jurídico com o provedor. |
| DeepSeek | OpenAI-compatible | `https://api.deepseek.com` | `deepseek-chat` | Endpoint oficial. |
| OpenAI | OpenAI-compatible | `https://api.openai.com/v1` | `gpt-4.1` | Modelos editáveis. |
| Anthropic | Messages API | `https://api.anthropic.com` | `claude-sonnet-4-5` | Adaptador nativo com ferramentas e streaming. |
| Gemini | OpenAI-compatible | `https://generativelanguage.googleapis.com/v1beta/openai` | `gemini-2.5-flash` | Camada compatível do Google. |
| Qwen / DashScope | OpenAI-compatible | `https://dashscope.aliyuncs.com/compatible-mode/v1` | `qwen-plus` | Endpoint compatível. |
| Moonshot / Kimi | OpenAI-compatible | `https://api.moonshot.cn/v1` | `moonshot-v1-8k` | Modelos editáveis. |
| Zhipu / GLM | OpenAI-compatible | `https://open.bigmodel.cn/api/paas/v4` | `glm-4-plus` | Endpoint compatível. |
| MiniMax | OpenAI-compatible | `https://api.minimax.chat/v1` | `abab6.5s-chat` | Confirme o modelo e a região. |
| Baidu Qianfan | OpenAI-compatible | `https://qianfan.baidubce.com/v2` | `ernie-4.5-turbo-128k` | Depende do plano contratado. |
| OpenRouter | OpenAI-compatible | `https://openrouter.ai/api/v1` | `deepseek/deepseek-chat` | Prefixo do provedor no modelo. |
| Ollama local | OpenAI-compatible | `http://127.0.0.1:11434/v1` | `llama3.2` | Sem chave; nada sai da máquina. |
| Personalizado | OpenAI ou Anthropic | informada pelo usuário | informado pelo usuário | Para gateways e endpoints BYOK. |

**CommandCode e Cline.** São agentes de programação que aceitam chaves de provedores (BYOK); não são gateways universais de modelo. Ferramentas de design não são provedores de LLM. O caminho correto é configurar o provedor subjacente no preset correspondente ou usar um endpoint personalizado documentado.

### OpenCode

O pacote instalado do OpenCode é um binário compilado, não um conjunto de módulos JavaScript legível. Não copiamos código dele. A referência concreta usada foi a documentação do app operacional de referência (`OPENCODE-GO.md`), que registra o preset OpenAI-compatible acima. O endpoint do Go exige o header `x-opencode-session`; o aplicativo envia um identificador opaco da sessão local, sem nome, processo ou texto documental. O padrão adotado — catálogo de provedores, URL/modelo editáveis, descoberta de modelos e adaptador de ferramentas — é o mesmo tipo de desenho usado por agentes como OpenCode e CommandCode, mas implementado de forma independente neste repositório.

## Chaves e armazenamento

- As chaves ficam em `%LOCALAPPDATA%\Previdenciar\ai-keys.pvai`.
- O arquivo usa DPAPI com `CRYPTPROTECT_UI_FORBIDDEN`, vinculado ao usuário Windows. Não há fallback em texto claro e não há `LOCAL_MACHINE`.
- A interface recebe apenas `hasKey`; a chave nunca volta para o JavaScript.
- O cofre é substituído por arquivo temporário + `MoveFileExW` atômico. Um cofre corrompido bloqueia novas gravações em vez de ser sobrescrito.
- A chave existe em memória durante a requisição e não é registrada em logs.
- O arquivo é local e não entra no workspace, no backup `.pvr` nem no Git.

## Consentimento e escopo

Antes de enviar, a pessoa precisa marcar **Permitir enviar esta conversa ao provedor**. O pedido contém:

1. o texto da pergunta;
2. o histórico desta conversa, limitado localmente;
3. o resumo estruturado do caso, somente se você selecionar um caso e marcar o anexo;
4. os trechos retornados pelas ferramentas locais, quando habilitadas;
5. o prompt de sistema do Copiloto.

Não são anexados automaticamente CNIS, identidade, CPF, salários, cálculos, PDFs ou o workspace. O resumo opcional do caso exclui nome, CPF, NIT, valores, notas e textos livres; o seletor mostra o nome apenas na interface local. O provedor pode reter e processar o texto conforme os próprios termos. A autorização pode ser desmarcada a qualquer momento; ela não é persistida no `localStorage`.

## Ferramentas locais

| Ferramenta | Função | Limite |
|---|---|---|
| `buscar_habilidades` / `ler_habilidade` | Catálogo e texto das habilidades da base de conhecimento. | Índice + Markdown importados. |
| `buscar_legislacao` / `ler_legislacao` | Boletim federal e ficha da norma. | Vigência e inteiro teor devem ser conferidos na fonte oficial. |
| `buscar_natjus` / `ler_natjus` | Metadados, ficha e texto extraído dos pareceres e-NatJus. | PDFs escaneados podem falhar; o link oficial permanece. |
| `buscar_jurisprudencia` / `ler_jurisprudencia` | Ementas capturadas e link oficial. | Amostra importada, não acervo completo. |

As ferramentas são somente leitura, recusam caminhos absolutos e travessia de diretório, limitam o tamanho dos índices e truncam o conteúdo devolvido. O modelo não escolhe caminhos de arquivo: só IDs retornados pelos índices. Conteúdo de ferramenta é tratado como dado não confiável; instruções dentro de documentos são ignoradas pelo prompt de sistema.

## Importação da base local

O importador copia apenas índices e textos públicos da auditoria para fora do repositório. Na tela **Copiloto IA > Base local**, use **Selecionar pasta da auditoria** e escolha a pasta `base-conhecimento`; o aplicativo copia os dados para `%LOCALAPPDATA%\Previdenciar\conhecimento`. Os PDFs NatJus são copiados e têm o texto extraído por padrão.

Também existe o caminho por terminal, útil em máquinas sem a interface:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/import-base-conhecimento.ps1 -Source "C:\caminho\da\base-de-conhecimento"
```

O destino padrão é `%LOCALAPPDATA%\Previdenciar\conhecimento`. Use `-Source` para indicar outra pasta de auditoria e `-SkipNatjusPdf` para não copiar os PDFs NatJus. O script de terminal copia os PDFs, mas não extrai o texto. Para texto pesquisável dos NatJus, use o importador da tela **Base local**; ele faz a extração e grava os arquivos em `natjus/pdf-text`. Ambos os caminhos não apagam arquivos existentes, não usam rede e não gravam no workspace.

Contagens observadas na importação de 29/10/2026:

- 2.069 habilidades;
- 64 normas no boletim;
- 112 pareceres NatJus, 112 com texto extraído;
- 118 ementas normalizadas.

Esses números são do extrato local, não uma garantia de cobertura, vigência ou qualidade do acervo.

## Minuta assistida

A página **Minuta assistida** usa o mesmo provedor e o mesmo consentimento do Copiloto em três etapas:

1. **Roteiro**: tipo de documento, objetivo e resumo opcional do caso geram um plano editável.
2. **Minuta**: o roteiro aprovado é enviado com o resumo do caso para gerar um texto em Markdown, com ferramentas locais e pendências marcadas.
3. **Revisão**: a minuta é analisada em busca de citações não verificadas, dados faltantes, afirmações excessivas e problemas de estrutura.

O texto final pode ser copiado, exportado em texto, exportado em DOCX, impresso/salvo como PDF pelo diálogo do sistema ou salvo como modelo local no workspace. O botão **Anexar PDF do caso** extrai texto de PDFs textuais e, se não houver texto suficiente, tenta OCR local nas primeiras 20 páginas quando Tesseract (com idioma português) e Poppler estão instalados; o anexo mostra hash SHA-256, origem e páginas, e o texto fica apenas na memória da página. O texto só entra no contexto depois que você marca a confirmação da leitura. O texto pode conter dados pessoais, então revise antes de autorizar o envio. A **revisão adversarial** é uma etapa separada. Antes de exportar, a tela mostra avisos locais para marcações de pendência, seções sem texto e citações não retornadas; os avisos não bloqueiam a exportação e não substituem a revisão humana. Versões podem ser salvas e restauradas enquanto o aplicativo estiver aberto; a navegação entre telas não as apaga. Comentários ancorados e persistência de versões entre sessões ainda não fazem parte desta entrega. O OCR local depende das ferramentas instaladas e pode ser mais lento em autos longos.

## Limites de implementação

- A conversa não é persistida; recarregar a página perde o histórico.
- O cancelamento é cooperativo: a interface pede o encerramento, mas o provedor pode demorar a fechar o stream.
- Há retentativa limitada para 429/5xx/falha de rede antes do streaming; depois que o stream começa, não há retomada automática.
- O consumo de tokens é exibido quando o provedor o informa; nem todo endpoint envia usage em streaming.
- O teste de conexão prova que o endpoint respondeu, não que o modelo seja adequado ao uso jurídico.
- A descoberta de modelos depende de o provedor expor `/models`; vários gateways não expõem.
- A extração de texto dos PDFs NatJus pode falhar em documentos escaneados; nesse caso, a ferramenta usa a ficha local e o link oficial.
- O anexo de PDF da Minuta tenta texto nativo e, se necessário, OCR local. Para o OCR, instale o Tesseract com o pacote de português e o Poppler (`pdftoppm`); o Tesseract pode ficar no PATH ou no diretório padrão de instalação, e o Poppler precisa estar no PATH. O OCR limita-se às primeiras 20 páginas e roda na máquina; as imagens não são enviadas. O texto extraído só segue ao provedor dentro do pedido autorizado. O anexo também procura padrões de instrução dirigida ao modelo e avisa, mas não é um detector completo de texto oculto, fonte minúscula ou texto branco.
- O resumo do caso é montado no frontend a partir de campos estruturais e pode omitir pendências que dependem de texto livre. Ele não substitui a leitura do caso e não é enviado sem seleção e marcação explícitas.

## Verificação

```powershell
cargo test -p previdenciar-ai
cargo test -p previdenciar-next-desktop
cd apps/desktop
npm run typecheck
npm run build
```

Os testes automatizados de provedor usam transporte sintético e fixtures. Em 29/10/2026 foi executado um teste real com chave temporária do OpenCode Go e o modelo `deepseek-v4.1-flash`: o fluxo incluiu busca em 2.069 habilidades, leitura da habilidade `aposentadoria-tempo-contribuicao`, streaming e encerramento com `finish=stop` (5.236 tokens de entrada e 764 de saída). A chave temporária foi apagada e não foi gravada no repositório. Nenhum CNIS ou documento de cliente foi enviado.

