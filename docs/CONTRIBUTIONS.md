# Livro de contribuições

## Fluxo

No editor de dados, abra 5. Salários e escolha Abrir livro de contribuições. O mesmo painel aparece na etapa Remunerações da entrevista guiada.

A preparação inclui todos os lançamentos importados e mantém salários manuais anteriores como registros separados. Não descarta duplicidades nem confirma moeda. Se havia PBC manual e CNIS, revise possíveis duplicidades de representação antes de aplicar.

- Edite competência, valor nominal, moeda, vínculo declarado e indicadores. Informe uma justificativa documental nova. O original permanece no CNIS e é visível ao lado do ajuste.
- Inclua contribuições manuais com competência e valor próprios. Não invente ausência como zero.
- Filtre por intervalo, vínculo e situação; selecione lançamentos por página. A seleção pode atravessar páginas, mas mudar filtros limpa a seleção para evitar ações em registros escondidos.
- Ações em lote exigem justificativa: declarar BRL, conferir, excluir da hipótese e reincluir. Excluir nunca apaga a origem.
- Consolide lançamentos ativos da mesma competência e moeda somente quando documentalmente apropriado. A soma é nominal e exata; a consolidação remove a confirmação anterior e exige nova conferência. Nenhum indicador é resolvido pelo botão de consolidar.
- Desfaça operações em ordem, inclusive após aplicar. O desfazer é registrado e não restaura resultado financeiro antigo.
- Clique Aplicar revisão ao cálculo quando todos os registros ativos forem conferidos, documentados, em BRL, sem indicadores pendentes e dentro do PBC suportado. Concomitâncias precisam estar consolidadas explicitamente. Registros excluídos também exigem justificativa.

## Fronteira do motor

Aplicar gera os salários manuais usados pelo adaptador RGPS. O Rust verifica que eles coincidem exatamente com o livro. Depois de qualquer ajuste, o resultado e a confirmação do assistente são invalidados; o motor bloqueia até a reaplicação.

Isso não libera automaticamente parâmetros, regime, CTC ou exclusão documental da fonte original. O adaptador continua restritivo e experimental. Não há conversão histórica automática ou certificação de enquadramento.

## Origem e histórico

O livro é opcional e versionado, vinculado ao hash informado na importação e aos índices dos lançamentos. Referências ausentes/duplicadas, omissão de origem, alterações iniciais sem histórico, cadeias incompatíveis e projeção divergente são recusadas no Rust.

Cada operação mantém antes/depois e justificativa. A trilha é local e auditável, não uma assinatura digital nem prova imutável contra o próprio dono dos arquivos. Há limites de 10.000 registros, 200 operações e 50.000 snapshots de registros no histórico: o sistema recusa excedentes, não corta operações silenciosamente.

Para trocar o CNIS de uma hipótese com livro existente, crie outro cálculo e preserve o caso anterior. Os fluxos de importação não substituem uma origem já revisada.

## Persistência e compatibilidade

Workspaces antigos carregam com contribution_review=null, sem escrever a base apenas por leitura. A gravação continua explícita. Após salvar o novo campo, versões anteriores podem recusar a base por não conhecerem o contrato: use a nova versão e preserve backups; não apague a base para contornar isso. Identificador Tauri e caminho legado de armazenamento permanecem iguais.

## PDFs

Os PDFs detalhado, de salários e de memória incluem origem/ajustes e histórico. Em modo privado, justificativas e motivos livres são ocultados. Datas e valores podem permitir correlação: identificação oculta não garante anonimização.

## Exemplo de PDFs reais com dados fictícios

Execute cargo run -p previus-reports --example contributions_report -- target/contribution-example. O exemplo cria dois PDFs novos, sem sobrescrever arquivos; não calcula benefício e nunca lê documentos pessoais. Para repetir, indique outro diretório vazio.
