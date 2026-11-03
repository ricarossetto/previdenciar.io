# Privacidade e dados locais

## Armazenamento

No Windows, a base protegida usa DPAPI e o caminho LOCALAPPDATA/Previdenciar/workspace.pvr. Dados são vinculados ao ambiente da conta Windows; mover um arquivo cifrado não garante que ele poderá ser aberto em outra conta ou máquina.

Os fluxos de importação, cálculo, PDF, relatórios e backup não enviam documentos a servidor nenhum. O Copiloto IA é uma exceção **opcional e explícita**: ele só envia a pergunta, o histórico da conversa e os trechos de ferramentas locais depois que a pessoa marca a autorização. CNIS, clientes, cálculos e PDFs não são anexados automaticamente. As chaves ficam em `%LOCALAPPDATA%\Previdenciar\ai-keys.pvai`, protegidas por DPAPI, e não voltam para a interface. Uma release pode ser baixada via GitHub, e dependências usam rede durante o desenvolvimento/build; isso não significa upload automático dos documentos do usuário.

## Copiloto IA opcional

O Copiloto não é acionado na inicialização e não recebe o workspace. A tela mostra quais dados serão enviados antes do primeiro pedido: o texto da pergunta, o histórico em memória, o prompt de sistema e, se a consulta local estiver habilitada, os trechos públicos retornados pelas ferramentas. Se você selecionar um caso e marcar o anexo, também é enviado um resumo estrutural sem nome, CPF, NIT, salários, notas ou textos livres. Esse resumo contém apenas campos do caso, como módulo, DIB, sexo e condição de deficiência; o envio depende da marcação explícita e pode ser desfeito antes de cada pedido. A autorização é por sessão e pode ser desmarcada.

O histórico da conversa não é salvo no workspace, no backup ou no cofre. O provedor externo escolhido pode reter o texto conforme a própria política; o aplicativo não controla essa retenção. Não envie dados de cliente, CNIS ou documentos por chat sem necessidade e sem conferir a política do provedor.

As ferramentas locais leem os extratos importados de habilidades, legislação, NatJus e jurisprudência em `%LOCALAPPDATA%\Previdenciar\conhecimento`. Elas não gravam no workspace e não enviam nada por conta própria; o resultado só chega ao provedor quando o modelo o solicita dentro de uma conversa já autorizada. Na Minuta assistida, o resumo estrutural do caso, o roteiro e o texto em revisão também só são enviados depois da marcação de consentimento. O PDF anexado é convertido em texto localmente, com OCR local opcional quando Tesseract e Poppler estão instalados; as imagens não são enviadas. O texto extraído pode conter dados pessoais; ele só entra no pedido depois que você confirma a leitura, e você pode remover o anexo se não quiser enviá-lo. A minuta exportada ou salva como modelo local passa a ser um arquivo do operador; revise antes de compartilhar.

## Importação e exportação

Importar não grava automaticamente a fonte na base nem confirma sua validade jurídica. Um caso novo só entra na base por ação explícita de salvar; depois disso, a entrevista assistida de um rascunho já salvo é autossalva localmente enquanto é editada, para permitir retomada. Relatórios e backups são arquivos sensíveis, mesmo quando a identificação direta é ocultada. Datas, remunerações e vínculos podem permitir correlação. O modo privado não é garantia de anonimização.

## Repositório público

Não envie CNIS, PDFs, identificação, CPF/NIT, bases, backups, screenshots de clientes, arquivos temporários ou logs reais. Issues e pull requests devem usar dados fictícios e mensagens mínimas. Arquivos de teste públicos são sintéticos; as integrações com documentos pessoais não foram publicadas.

## GitHub Pages

O site é estático e possui links para documentação e releases. Não há campo de upload, cálculo remoto, formulário de clientes, analytics de terceiros ou backend previdenciário hospedado.

## Índices públicos por solicitação explícita

A tela de cenário PcD permite consultar índices e limites públicos na Fábrica de Cálculos/TRF3. A requisição contém somente o intervalo de competências, sem CNIS, identidade, salários ou dados de saúde. Esse comando não lê variáveis de credenciais nem exige chave. O snapshot pode integrar a base local protegida quando o usuário salva explicitamente o cálculo. Falha na rede não substitui a tabela anterior nem cria valores ausentes.

## Conferência online opcional de RMI

A conferência na Fábrica de Cálculos/TRF3 é uma ação separada, disparada pelo botão no resultado. Ela exige uma chave de API opcional, informada em Configurações e guardada na base local cifrada. A chave é enviada somente no cabeçalho `x-api-key` para o endpoint fixo `https://fabrica-de-calculos.app.trf3.jus.br/rgps/api/v1/calculos-rmi`; não entra no corpo, no PDF, nos relatórios ou nos logs do aplicativo.

O corpo enviado contém apenas data de nascimento, sexo, DIB, espécie, períodos e salários revisados. Nome, CPF, NIT, CNIS, PDF, endereço, anotações e a chave não são enviados. O provedor é público e pode registrar IP, data e conteúdo da requisição conforme a política dele; o aplicativo não controla a retenção do lado do provedor. O resumo retornado, a fonte, o horário e os hashes ficam na base local protegida; a resposta bruta não é gravada.

Sem chave, sem confirmação no diálogo ou com dados não revisados, nada é enviado. O cálculo local, os PDFs e a exportação continuam funcionando offline. Backups protegidos incluem a chave cifrada e os resumos de conferência; trate o arquivo de backup como documento sensível.

## Produto web oficial: arquivos no S3

O produto web (https://previdenciario.atrium.adv.br) é outro artefato: usa login pelo Cognito, PostgreSQL e um bucket S3 privado. As regras de retenção do bucket estão descritas em [PRODUCTION_AWS.md](PRODUCTION_AWS.md), seção "Retenção de arquivos no S3".

- **`cnis/` expira em 1 dia.** O CNIS enviado é lido para a importação e o objeto é apagado pelo ciclo de vida do S3 no dia seguinte ao envio. O expurgo é automático e não depende de ação do usuário.
- **`reports/` e `exports/` expiram em 30 dias.** Os PDFs e as exportações geradas ficam disponíveis nesse prazo.

O trade-off é o seguinte. De um lado, a minimização e a necessidade (art. 6º, III, e art. 15 da LGPD) recomendam manter o CNIS só enquanto ele serve para a importação, porque o documento pode conter dado pessoal sensível de saúde (art. 5º, II) e um arquivo apagado não vaza. Do outro, a prova em disputa pode exigir demonstrar o que foi importado e quando, e nesse caso um arquivo apagado em 24 horas não serve como evidência.

O prazo de 1 dia é o **comportamento atual do sistema**, não uma política aprovada. A decisão final cabe ao dono do produto, com apoio jurídico e de segurança. Estado em 02/11/2026: **aguardando decisão**. Enquanto ela não existe, vale o default de 1 dia, e a engenharia não altera a regra de ciclo de vida por conta própria.
