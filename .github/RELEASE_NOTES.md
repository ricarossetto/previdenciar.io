## previdenciar.io 0.10.3

Versão estável da linha 0.10 para Windows x64. Desktop local, gratuito e open source sob MIT; não é serviço oficial do INSS nem certificação de direito a benefício.

### Novidades

- Espécie de benefício explícita no contrato. Apenas a 42 tem motor; as demais espécies bloqueiam com motivo.
- Fator previdenciário com as alíquotas da Lei 9.876/1999 (0,31 e 0,30) e tábua de expectativa de sobrevida embutida, com vigências de 1992 a 2025. O pedágio de 50% calcula sem expectativa declarada.
- IRRF mensal oficial por competência, com dependentes e isenção de 65 anos declarada. A vigência atual foi conferida com a Lei 14.848/2024.
- Descarte dos menores salários da EC 103 (art. 26, § 6º) como opção explícita, com o mínimo de competências que preserva o direito.
- Transições medidas por tempo apurado, com corte em 13/11/2019, e qualidade de segurado unificada em 12 meses.
- Importação de CNIS sem rejeição pelo índice dos valores consolidados; duplicação de cálculo com identificadores novos; origem dos valores na revisão de contribuições.
- Abertura da base fora da thread principal.

### Preparação guiada a partir do CNIS

- A tela de parâmetros ganhou um botão único que prepara e calcula a hipótese a partir do CNIS: exclui vínculos PRPPS, fecha vínculo sem data final pelo último salário, soma remunerações concomitantes do RGPS e registra cada decisão em notas. A fonte permanece intacta.
- Sem INPC publicado para a DIB, a correção é desativada e o resultado é nominal, com aviso.

### Correção

- Competências com valor abaixo do salário mínimo (indicador PREC-MENOR-MIN) também ficam fora do PBC no caminho RGPS comum, com aviso e lista das competências. Antes a regra valia apenas no cenário PcD.

### Revisão de texto

- Revisão de texto na interface, nos documentos e nas mensagens: travessões substituídos por vírgulas, dois-pontos ou pontos, aspas curvas convertidas em aspas retas, reticências em três pontos e títulos em caixa de frase. Nenhum cálculo, código de bloqueio ou dado foi alterado.

### Paridade e validação

- Caso PcD de referência: PBC 357, divisor 285, soma R$ 1.482.072,91 e RMI R$ 5.200,25. O motor público do TRF3 retorna R$ 5.200,88 e o software de referência R$ 5.200,89, diferença de 0,012% compatível com arredondamento de índices. Tempo e carência batem: 10.698 dias e 357 meses.
- 471 testes Rust, 99 testes de interface e 36 testes de navegador aprovados; `tsc` limpo; revisão de fonte pública sem achados. Os testes de navegador usam IPC sintético.

### O que permanece em aberto

- Decreto 10.410/2020: as duas regras aparecem na interface do software de referência (R$ 5.032,06 e R$ 4.874,98), mas a fórmula não é recuperável do export. Não foi implementada por inferência.
- Conversão de tempo comum para PcD (0,93/0,89/0,83), ROI, Análise da Situação Atual e planejamento na interface continuam fora desta versão.
- O diálogo nativo de Salvar PDF e a aprovação visual humana não estão certificados.

### Download e compatibilidade

Executável portátil `previdenciar.io-0.10.3-windows-x64.exe`, sem assinatura de código; requer Windows 10/11 x64 e WebView2. Confira `SHA256SUMS.txt` e `BUILDINFO.json`. Não desative proteções do Windows para executar o arquivo.

A base local cifrada e o identificador legado são preservados, sem migração silenciosa. Nenhum CNIS, PDF de cliente, base, credencial ou perfil de teste acompanha esta release. Use dados fictícios no primeiro teste.
