## previdenciar.io 0.12.0

Versão para Windows x64. Desktop local, gratuito e open source sob MIT. Não é serviço oficial do INSS nem certifica direito a benefício. O motor segue experimental: hipótese bloqueada continua bloqueada e dado ausente não vira zero.

### Conferência guiada em dois passos

- A Etapa 5 abre com as respostas da entrevista: benefício atual, finalidade, objetivo, segurado, sexo e deficiência, e DIB. Cada grupo tem o botão que volta à etapa certa.
- O preparo a partir do CNIS passou a ficar na própria página, sem modal: mostra a referência documental, o que o clique faz (exclui a fonte como entrada, monta vínculos e remunerações, aplica a política e calcula) e o que ele não faz.
- Quando as pendências são fonte, política, vínculos e remunerações, a conferência informa que um clique as resolve e deixa de mandar o operador para as abas.
- A preparação deixou de inventar respostas da entrevista: benefício atual, finalidade e objetivo sem resposta continuam pendentes e podem ser corrigidos na conferência.

### Instalador e atualização

- Instalador NSIS com atalhos e desinstalador.
- Verificação de atualização assinada pelo atualizador do Tauri, com `latest.json` assinado pela chave do projeto.
- A logo do escritório passou a ser configurável pelo workspace; no repositório permanece um placeholder genérico.

### Correções

- Rodapé mostra a versão real do build, não mais um texto fixo.
- Opções da Etapa 3 nomeiam o objetivo, em vez de começar com "Não" ou "Sim".
- O botão de avanço da Etapa 4 passou a ser "Prosseguir para a conferência".
- Os testes de interface voltaram a rodar: a versão do build é definida no ambiente de teste.
- A suíte Rust voltou a compilar com o campo da logo e com as permissões do atualizador no teste de capacidades.
- Documentação de usabilidade sem nome de cliente e sem valor derivado; o CPF de exemplo do formato de relatório é sintético.

### Verificação

- 485 testes Rust, 107 testes de interface e 36 testes de navegador.
- O fluxo guiado do CNIS à conferência e ao preparo em um clique tem teste próprio com IPC sintético.
- Revisão de fonte pública sem achados.

### Limites

- O motor cobre hoje a espécie 42. Outras espécies bloqueiam com motivo.
- Revisão de benefício recebido, execução judicial completa e cenários RPPS não têm cálculo automático.
- Sem INPC publicado para o mês da DIB, a correção é desativada e o resultado é nominal, com aviso.
- A distribuição não tem assinatura de código; confira o SHA-256 publicado.
