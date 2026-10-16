## previdenciar.io 0.8.3-beta.1

Pré-release experimental para Windows x64. Desktop local, gratuito e open source sob MIT; não é serviço oficial do INSS nem certificação de direito a benefício.

### Novidades e correções

- Cenário PcD de grau constante pela LC 142, com sexo/grau/regime e premissas documentais explícitos. Graus mistos, CTC automática, conversões e fator previdenciário não são presumidos.
- Consulta opcional de índices e limites públicos da Fábrica de Cálculos/TRF3, com snapshot local e proveniência. A requisição não envia CNIS, identidade, remunerações ou dados de saúde e não exige chave de API.
- Separação entre salários importados, preenchimentos declarados e projeções; edição dos dados revoga a confirmação anterior. Salários manuais sem livro aplicado bloqueiam em vez de serem ignorados.
- Correção da importação de remunerações de cooperativa com colunas intercaladas e de competências parciais no relatório.
- PDFs Rust reais de memória, tempo, salários, resumo, detalhado, comparação e pendências, com limites e hipóteses identificados.
- Proteção contra resposta assíncrona aplicar um rascunho antigo e correção da negociação HTTP no cliente nativo, preservando TLS validado.
- Fluxo de opções da RMI alinhado aos parâmetros realmente suportados; removidas escolhas de interface sem efeito no cálculo.

### Validação e limites

Validação local: 418 testes Rust, 79 testes de interface e 32 testes de navegador aprovados. Testes de navegador usam IPC sintético. A verificação nativa em perfil isolado cobre importação, consulta pública, cálculo, PDFs, gravação cifrada e reabertura, sem modificar a base normal.

Essas evidências não certificam atualização normativa universal, carência juridicamente válida, deficiência, concessão ou equivalência integral a outro software. O diálogo nativo de Salvar PDF e a aprovação visual humana não estão certificados. Leia `docs/LC142_SCENARIOS.md`, `docs/LIMITATIONS.md` e `docs/PRIVACY.md`.

### Download e compatibilidade

Executável portátil `previdenciar.io-0.8.3-beta.1-windows-x64.exe`, sem assinatura de código; requer Windows 10/11 x64 e WebView2. Confira `SHA256SUMS.txt` e `BUILDINFO.json`. Não desative proteções do Windows para executar o arquivo.

A base local cifrada e o identificador legado são preservados, sem migração silenciosa. Hipóteses com política/snapshot desta versão podem não abrir em versões antigas; preserve seus backups. Nenhum CNIS, PDF de cliente, base, credencial ou perfil de teste acompanha esta release. Use dados fictícios no primeiro teste.
