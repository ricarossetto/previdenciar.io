# Limitações e uso responsável

Esta distribuição é uma pré-release experimental. Não use a existência de uma tela ou o resultado de um teste como prova de conformidade normativa.

- O motor RGPS não cobre todos os benefícios, regimes, cenários, datas históricas ou hipóteses especiais. Parâmetros não suportados devem bloquear, não produzir valores presumidos.
- O modelo observacional original possui limitações documentadas em crates/rgps-core/tests/model_limits.rs; o adaptador bloqueia caminhos conhecidos inadequados em vez de recomendá-los.
- Concomitâncias, PRPPS, CTC, datas abertas, moedas históricas e indicadores exigem análise e confirmação; não são resolvidos por padrão.
- O livro de contribuições é um overlay documental, não uma alteração do CNIS. A aplicação exige BRL, competências a partir de 1994-07, confirmação, justificativa, indicadores resolvidos e consolidação explícita de concomitâncias. A política/validação de períodos, regime, CTC e fonte continua separada.
- Tabelas/índices não recebem uma promessa de atualização normativa automática. Registre fonte, período e versão.
- PDF de imagem não tem OCR completo integrado nesta distribuição. Não interprete falha de extração como ausência de contribuições.
- PDFs são documentos de revisão e memória, não concessão de benefício, parecer jurídico ou reprodução certificada de outro software.
- Restauração automática de backup não está implementada. Não prometa esse fluxo com base na presença de um botão.
- A exportação em lote grava um PDF novo por tipo selecionado na pasta escolhida pelo operador. Nomes existentes não são sobrescritos e a falha de um tipo não desfaz os demais. O seletor de pasta é um diálogo nativo do sistema e não foi automatizado nos testes.
- O título do relatório e as opções de apresentação mudam apenas o documento impresso, nunca o cálculo. O título fica registrado no cálculo e pode ser alterado a qualquer momento; a apresentação (anexo técnico, conferência de qualidade, premissas de planejamento) vale para a geração em curso.
- A qualidade de segurado é **apurada** por vínculo pelo próprio motor (`crates/previus-core/src/insured_quality.rs`): filiação → primeiro pagamento → período de graça de 12 meses (Lei nº 8.213/1991, art. 15, II) → prorrogação **para até 24 meses** quando havia mais de 120 contribuições sem interrupção (art. 15, §1º) → contribuições desconsideradas. O tempo intercalado em gozo de benefício não conta para carência (Decreto nº 3.048/1999, art. 19-C, §1º) e a competência presente em mais de um vínculo é contada uma única vez. A carência exigida é a do art. 25, II (180 contribuições) ou a faixa da tabela do art. 142 para o segurado inscrito até 24/07/1991, com o ano de implementação declarado ou um **ano candidato** explicitado no documento. A regra "4/5" (144 contribuições) **não existe** no texto vigente — foi verificada em dez documentos primários, inclusive a MP 676/2015, a Lei nº 13.183/2015 e as mensagens de veto — e por isso não é aplicada nem inferida. O relatório "Cálculo da Qualidade de Segurado" declara a fonte de cada regra. Isso não reproduz o relatório de qualidade do software original, cujas fórmulas não são recuperáveis do binário (a REA não encontra os rótulos correspondentes entre as strings do executável), e os outputs "Em Benefício" do original não são reproduzidos.
- A importação de CNIS lê as tabelas de relações e remunerações, a de contribuições (competência, data de pagamento, contribuição e salário), as remarcações de cooperativa e a tabela de valores consolidados por ano civil, em que a posição do valor na página define o mês. Uma tabela que continua na página seguinte sem repetir o cabeçalho é lida com o contexto herdado. A tabela de valores consolidados **não declara o vínculo de origem**: a competência é atribuída ao único período que a cobre e, havendo mais de um, fica sem vínculo e exige conferência.
- O cenário PcD pode ser calculado **sem rede**, com as séries e tetos oficiais embutidos (INPC/IBGE, IGP-DI/FGV e tetos), e a proveniência do snapshot fica declarada (origem local e SHA-256 das tabelas). A consulta pública do TRF3 continua disponível e tem precedência quando responde. A política de variação negativa histórica é declarada pelo operador e muda o resultado: ela precisa ser escolhida, não presumida.
- Competência sem lançamento na origem **não** entra no PBC — não é tempo de contribuição — e a omissão é declarada na lista de avisos do resultado. Ela nunca vira zero, mas a conferência documental continua obrigatória: uma falha de leitura da origem se assemelharia a uma lacuna real. Competência cujo valor consolidado fica abaixo do salário mínimo (indicador PREC-MENOR-MIN) também não entra no PBC nem no tempo até a complementação, como no próprio extrato e no motor público; a exclusão é declarada nos avisos.
- O enquadramento e a carência por vínculo não são apurados além da comparação informativa acima.
- Finalidades sem motor próprio (revisão de valor, planejamento futuro, execução judicial e contribuições em atraso) são registradas e bloqueiam a execução de RMI; elas não produzem valor estimando.
- A versão inicial é focada no Windows e não possui assinatura digital de código.
- O site publicado não executa o aplicativo desktop e não recebe documentos.
- A identidade visual definitiva está pendente do OpenDesign; o ícone atual é provisório.

## Validação conhecida

O diálogo Novo cálculo recebeu superfícies de leitura mensuráveis, e os novos testes axe-core verificam os temas claro/escuro sem ignorar regras. O editor e o diálogo de ajustes também recebem verificações de WCAG em desktop/mobile. Isso é evidência automatizada limitada aos estados testados, não certificação completa de acessibilidade ou aprovação visual humana.

Testes de navegador usam IPC sintético. Mesmo com testes verdes, o diálogo nativo de salvar PDF, revisão visual humana e validação jurídica não estão automaticamente certificados.

## PcD de grau constante

Existe um caminho experimental de simulação com premissas explícitas, descrito em [Cenários LC 142](LC142_SCENARIOS.md). Não cobre graus mistos, conversões, fator previdenciário ou certificação de carência/deficiência. Coincidência matemática com um relatório não comprova continuidade contributiva, laudo, regime, atualização normativa ou direito a benefício. A divisão inteira dos 80% e a convenção 30/360 são escolhas declaradas do perfil comparativo, não uma recomendação automática para todos os casos.
