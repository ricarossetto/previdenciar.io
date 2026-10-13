# Limitações e uso responsável

Esta distribuição é uma pré-release experimental. Não use a existência de uma tela ou o resultado de um teste como prova de conformidade normativa.

- O motor RGPS não cobre todos os benefícios, regimes, cenários, datas históricas ou hipóteses especiais. Parâmetros não suportados devem bloquear, não produzir valores presumidos.
- O modelo observacional original possui limitações documentadas em crates/rgps-core/tests/model_limits.rs; o adaptador bloqueia caminhos conhecidos inadequados em vez de recomendá-los.
- Concomitâncias, PRPPS, CTC, datas abertas, moedas históricas e indicadores exigem análise e confirmação; não são resolvidos por padrão.
- A edição manual de salários existe, mas o painel completo de edição/reconciliação de contribuições importadas ainda não existe.
- Tabelas/índices não recebem uma promessa de atualização normativa automática. Registre fonte, período e versão.
- PDF de imagem não tem OCR completo integrado nesta distribuição. Não interprete falha de extração como ausência de contribuições.
- PDFs são documentos de revisão e memória, não concessão de benefício, parecer jurídico ou reprodução certificada de outro software.
- Restauração automática de backup não está implementada. Não prometa esse fluxo com base na presença de um botão.
- A versão inicial é focada no Windows e não possui assinatura digital de código.
- O site publicado não executa o aplicativo desktop e não recebe documentos.
- A identidade visual definitiva está pendente do OpenDesign; o ícone atual é provisório.

## Validação conhecida

A rodada local anterior tinha dois apontamentos de contraste inconclusivos no diálogo Novo cálculo, sem supressões. Esses apontamentos não são tratados como aprovação visual. A versão pública recebe verificações próprias, registradas em CI e nas notas de release.

Testes de navegador usam IPC sintético. Mesmo com testes verdes, o diálogo nativo de salvar PDF, revisão visual humana e validação jurídica não estão automaticamente certificados.
