## previdenciar.io 0.9.1

Versão estável da linha 0.9 para Windows x64. Desktop local, gratuito e open source sob MIT; não é serviço oficial do INSS nem certificação de direito a benefício.

### Novidades e correções

- **Importação de CNIS corrigida em três defeitos**, todos achados num caso real de 23 páginas: a tabela de contribuições (competência, data de pagamento, contribuição e salário) continuava na página seguinte sem repetir o cabeçalho e as linhas seguintes eram descartadas; a nota de rodapé do CNIS era confundida com um cabeçalho de tabela e marcava salários válidos como "contribuição não resolvida"; e o contexto de tabela não atravessava a quebra de página. Com as correções, a leitura do caso real passou de 485 para 610 lançamentos e os salários por competência passaram a coincidir um a um com o relatório de referência.
- **Série oficial do IGP-DI/FGV embutida** (SGS/BCB, série 190) e INPC até 2026-09, gerados por `scripts/gerar-dados-economicos.mjs` a partir da fonte pública, com a fonte declarada no próprio arquivo.
- **Cenário PcD calculável sem rede**: snapshot montado com as séries e tetos oficiais embutidos, com proveniência declarada (origem local e SHA-256 das tabelas). A consulta pública do TRF3 continua disponível e tem precedência.
- **Apuração da qualidade de segurado por vínculo, com cálculo próprio e fonte primária citada**: período de graça de 12 meses (art. 15, II) prorrogado para até 24 meses com mais de 120 contribuições (art. 15, § 1º), carência de 180 contribuições (art. 25, II) ou a faixa da tabela do art. 142, tempo em gozo de benefício fora da carência (Decreto 3.048/1999, art. 19-C, § 1º) e competência contada uma única vez quando aparece em mais de um vínculo.
- **8º relatório PDF**: "Cálculo da Qualidade de Segurado", na central, no assistente e no lote.
- **Correção de perda silenciosa de texto no renderer**: palavras mais largas que a coluna eram descartadas e as linhas cortadas perdiam o conteúdo. Vale para todos os tipos de relatório.
- Competência sem lançamento na origem deixa de bloquear o cálculo: fica fora do PBC (não é tempo de contribuição) e a omissão é declarada nos avisos. Nunca vira zero.

### Base legal e o que permanece em aberto

O cálculo do salário de benefício da pessoa com deficiência segue a remissão do art. 22 da EC 103/2019 para o art. 8º da LC 142/2013 e o art. 29, II, da Lei nº 8.213/1991: média aritmética simples dos 80% **maiores** salários-de-contribuição, com descarte dos 20% menores. A regra de carência "4/5" (144 contribuições) **não existe** no texto vigente — verificada em dez documentos primários, inclusive a MP 676/2015, a Lei nº 13.183/2015 e as mensagens de veto. Valem a carência do art. 25, II ou a faixa da tabela do art. 142.

Continuam **não implementados** e declarados: a carência legal por tabelas do RGPS, o "Em Benefício" e a desconsideração por afastamento reproduzidos do software original, e os módulos de planejamento. As fórmulas do software de referência **não são recuperáveis** do binário original: a REA (Ghidra) não encontra os rótulos do relatório de qualidade entre as strings do executável, e o recurso que os contém está comprimido. Por isso esta apuração é calculada por regra legal primária e declarada como cálculo próprio, nunca como reprodução.

### Validação e limites

Validação local: **456 testes Rust**, **97 testes de interface** (15 arquivos) e **36 testes de navegador** aprovados; `tsc` limpo; revisão de fonte pública aprovada sem achados. Testes de navegador usam IPC sintético. A verificação nativa em perfil isolado cobre importação, cálculo, os 8 PDFs, gravação cifrada e reabertura, sem modificar a base normal.

O caso real de conferência (em área privada, fora do repositório) reproduz a estrutura do relatório de referência — PBC, regra dos 80% maiores, soma e média por competência e os fatores de correção de 2009 em diante — com a diferença final declarada: sete competências que o CNIS lista apenas na visão "Contribuições" entram no PBC do motor e não entram no do software de referência, o que desloca a RMI em cerca de 4%. A escolha entre as duas leituras é profesional e está documentada.

Essas evidências não certificam atualização normativa universal, carência juridicamente válida, deficiência, concessão ou equivalência integral a outro software. O diálogo nativo de Salvar PDF e a aprovação visual humana não estão certificados. Leia `docs/LC142_SCENARIOS.md`, `docs/LIMITATIONS.md` e `docs/PRIVACY.md`.

### Download e compatibilidade

Executável portátil `previdenciar.io-0.9.1-windows-x64.exe`, sem assinatura de código; requer Windows 10/11 x64 e WebView2. Confira `SHA256SUMS.txt` e `BUILDINFO.json`. Não desative proteções do Windows para executar o arquivo.

A base local cifrada e o identificador legado são preservados, sem migração silenciosa. Hipóteses com política/snapshot desta versão podem não abrir em versões antigas; preserve seus backups. Nenhum CNIS, PDF de cliente, base, credencial ou perfil de teste acompanha esta release. Use dados fictícios no primeiro teste.
