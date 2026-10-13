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
