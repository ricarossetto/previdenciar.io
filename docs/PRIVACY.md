# Privacidade e dados locais

## Armazenamento

No Windows, a base protegida usa DPAPI e o caminho legado LOCALAPPDATA/PreviusRust/workspace.pvr, mantido por compatibilidade. Dados são vinculados ao ambiente da conta Windows; mover um arquivo cifrado não garante que ele poderá ser aberto em outra conta ou máquina.

Não há sincronização em nuvem, envio de CNIS ou modelo de linguagem remoto nos fluxos implementados. Uma release pode ser baixada via GitHub, e dependências usam rede durante o desenvolvimento/build; isso não significa upload automático dos documentos do usuário.

## Importação e exportação

Importar não grava automaticamente a fonte na base nem confirma sua validade jurídica. Salvar é explícito. Relatórios e backups são arquivos sensíveis, mesmo quando a identificação direta é ocultada. Datas, remunerações e vínculos podem permitir correlação. O modo privado não é garantia de anonimização.

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
