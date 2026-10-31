# Desktop congelado

A versão desktop 0.16.0 fica congelada como legado. Não recebe novas funcionalidades.

O produto oficial passa a ser o aplicativo web em `https://previdenciario.atrium.adv.br`, com o mesmo frontend React, backend Rust/Axum, PostgreSQL, Cognito e processamento na nuvem.

## O que continua no desktop

- Leitura da base local existente.
- Cálculos e relatórios já disponíveis na versão 0.16.0.
- Nenhuma migração automática da base desktop para a web foi feita. O usuário decidiu não migrar dados.

## O que não deve ser feito

- Não adicionar novos recursos exclusivos ao desktop.
- Não corrigir divergências de produto apenas no desktop.
- Não usar o desktop como backend do site.

Correções de segurança críticas podem ser aplicadas ao desktop, mas a evolução funcional ocorre no web.
