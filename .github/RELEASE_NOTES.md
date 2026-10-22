## previdenciar.io 0.9.0

Primeira versão estável da linha 0.9 para Windows x64. Desktop local, gratuito e open source sob MIT; não é serviço oficial do INSS nem certificação de direito a benefício.

### Novidades e correções

- **Apuração da qualidade de segurado por vínculo, com cálculo próprio e fonte primária citada**: filiação → primeiro pagamento → período de graça de 12 meses (art. 15, II, da Lei nº 8.213/1991) → prorrogação para até 24 meses quando houve mais de 120 contribuições sem interrupção (art. 15, § 1º) → contribuições desconsideradas. Carência de 180 contribuições (art. 25, II) ou faixa da tabela do art. 142 para quem estava inscrito até 24/07/1991. O tempo intercalado em gozo de benefício não conta para carência (art. 19-C, § 1º, do Decreto nº 3.048/1999) e a competência presente em mais de um vínculo é contada uma única vez.
- **8º relatório PDF próprio**: "Cálculo da Qualidade de Segurado", disponível na central, no assistente e na exportação em lote (`previus-08-qualidade-de-segurado.pdf`).
- **Correção da importação de CNIS**: linhas de cooperativa com o marcador "Não Cooperado" e a tabela de *Valores Consolidados por Ano Civil* passaram a ser lidas; a posição do valor na página define o mês. Em um CNIS real de 23 páginas, a leitura passou de 300 para **357 competências** — o mesmo PBC do relatório de referência.
- **Correção de perda silenciosa de texto no renderer**: palavras mais largas que a coluna eram descartadas e as linhas cortadas perdiam o conteúdo. A correção vale para todos os tipos de relatório.
- **Título do relatório editável e persistido** e **opções de apresentação** (anexo técnico de origem, conferência de qualidade, premissas de planejamento), que mudam apenas o que é impresso.
- Entrevista do assistente em sete etapas com hub de conferência, mapa de cobertura e revisão rastreável dos dados importados.

### Base legal e o que permanece em aberto

O cálculo do salário de benefício da pessoa com deficiência segue a remissão do art. 22 da EC 103/2019 para o art. 8º da LC 142/2013 e o art. 29, II, da Lei nº 8.213/1991: média aritmética simples dos **80% maiores** salários-de-contribuição, com descarte dos 20% menores. A regra de carência "4/5" (144 contribuições) **não existe** no texto vigente — foi verificada em dez documentos primários, inclusive a MP 676/2015, a Lei nº 13.183/2015 e as mensagens de veto. A carência de 180 ou a faixa do art. 142 é que se aplica.

Continuam **não implementados** e declarados como tal: a carência legal por tabelas (regra 4/5 e regras 4/5 do RGPS), o "Em Benefício" e a desconsideração por afastamento reproduzidos do software original, e os módulos de planejamento. As fórmulas do software de referência **não são recuperáveis** do binário original: a REA (Ghidra) não encontra os rótulos do relatório de qualidade entre as strings do executável, e o recurso que os contém está comprimido. Por isso esta apuração é calculada por regra legal primária e declarada como cálculo próprio, nunca como reprodução.

### Validação e limites

Validação local: **454 testes Rust**, **97 testes de interface** (15 arquivos) e **36 testes de navegador** aprovados; `tsc` limpo; revisão de fonte pública aprovada sem achados. Testes de navegador usam IPC sintético. A verificação nativa em perfil isolado cobre importação, cálculo, os 8 PDFs, gravação cifrada e reabertura, sem modificar a base normal.

Essas evidências não certificam atualização normativa universal, carência juridicamente válida, deficiência, concessão ou equivalência integral a outro software. O diálogo nativo de Salvar PDF e a aprovação visual humana não estão certificados. Leia `docs/LC142_SCENARIOS.md`, `docs/LIMITATIONS.md` e `docs/PRIVACY.md`.

### Download e compatibilidade

Executável portátil `previdenciar.io-0.9.0-windows-x64.exe`, sem assinatura de código; requer Windows 10/11 x64 e WebView2. Confira `SHA256SUMS.txt` e `BUILDINFO.json`. Não desative proteções do Windows para executar o arquivo.

A base local cifrada e o identificador legado são preservados, sem migração silenciosa. Hipóteses com política/snapshot desta versão podem não abrir em versões antigas; preserve seus backups. Nenhum CNIS, PDF de cliente, base, credencial ou perfil de teste acompanha esta release. Use dados fictícios no primeiro teste.
