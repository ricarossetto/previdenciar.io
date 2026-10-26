# Especificação dos relatórios PDF (paridade com o produto de referência)

Fonte estudada: `CALCULOS/` (caso LORETTA), 2026-10-25. Objetivo: os PDFs do
previdenciar.io devem sair iguais em formatação, organização e informação.

## Cabeçalho comum (todas as páginas)

Bloco de identificação em duas colunas, repetido no topo de cada página:

| Campo | Exemplo |
|---|---|
| Nome do Segurado | LORETTA VERCELINO SILVA |
| Nº de Identificação do Trabalhador (NIT) | 267.12342.28-5 |
| CPF | 111.111.111-11 |
| Data de Nascimento | 10/10/1979 |
| Sexo | Feminino |
| Nº do Benefício | (vazio quando não informado) |
| Espécie do Benefício | 42 - Aposentadoria Por Tempo de Contribuição ou Programada |
| Início do Benefício (DIB) | 22/05/2026 |
| Data da Atualização | 22/05/2026 |
| Inscrição no INSS | 16/10/1996 (só em alguns relatórios) |
| Nº do Processo | (só no Cálculo da R.M.I.) |
| Índice Utilizado Para a Atualização | (só no Cálculo da R.M.I.) |

## Rodapé comum

- Esquerda: `n/N` (ex.: `1/23`)
- Direita: data do cálculo no formato `DD/MM/AAAA` (ex.: `22/10/2026`)
- Assinatura (nos relatórios de conferência): `Ricardo De Luca Rossetto`
- O número do processo/escritório não aparece.

## Tipos de relatório

### 1. Relatório do Tempo de Contribuição
- **Períodos Informados**: Início, Fim, Tipo (Normal/Especial), Empresa, Cargo.
  Legenda: `* (Contribuições consideradas na carência)`. Marca `Def` no fim.
- **Períodos Considerados no Cálculo**: Início, Fim, Tipo, Empresa, Tempo de
  Contribuição (Anos|Meses|Dias), Fator, Total (TC x Fator) (Anos|Meses|Dias).
  Legenda: `# (Atividades Concomitantes) / Def (Deficiência)`.
  Totais: `Totais`, `Totais Sem o Tempo Projetado`,
  `Totais Somente dos Períodos Com Deficiência`.
- **Análise dos Dados**: Nº de Contribuições em DIB / Lei 9.876/99 / EC 103/2019;
  Tempo de Contribuição nos mesmos marcos; Idade nos mesmos marcos; Carência.

### 2. Cálculo da Qualidade de Segurado
Tabela: Início, Fim, Filiação, Carência (Anos|Meses|Dias), Tempo Contribuição,
Origem do Vínculo. Por linha: `1º pagamento em dia: MM/AAAA`,
`Período de Graça: DD/MM/AAAA`, prorrogação em 12 meses quando > 120
contribuições, `Contribuições desconsideradas no cálculo da carência: MM/AAAA`.
Totais no fim.

### 3. Análise das Regras de Transição - EC 103/2019
Cards por regra: nome da regra, Tempo de Contribuição, Carência (Necessário),
Idade (Necessário), Provável RMI (R$), `Possível aposentadoria em DD/MM/AAAA`.
Marca `Regra Escolhida`. Nota: projeções repetem o último salário.

### 4. Cálculo da R.M.I.
Tabela: Nº, Data, Dias, Salário de Contribuição, Salário Considerado,
Índice de Atualização, Salário Corrigido, Teto de Contribuição.
Resumo final: Idade na DIB (anos decimais), Nº de Parcelas no PBC,
Tempo de Contribuição (anos decimais), Soma dos N Salários,
Média dos Salários Corrigidos, Salário de Benefício, Coeficiente, RMI.
Marca `D` em competências desconsideradas.

### 5. Análise da Situação Atual do Benefício
- **ANÁLISE DOS REQUISITOS**: TEMPO DE CONTRIBUIÇÃO (requisito atingido),
  IDADE (sem idade mínima necessária quando aplicável).
- **PLANEJAMENTO**: A PAGAR (Salário de contribuição, Alíquota, Nº de
  contribuições, Total de contribuições INSS+IRRF) e A RECEBER (Data de
  aposentadoria, Valor estimado, Expectativa de sobrevida, Total de benefício,
  Contribuição mensal INSS, IRRF, Tempo para repor, Nº estimado de meses).

### 6. Cálculo do Retorno Sobre o Investimento
Uma página por regra. Cabeçalho extra: Periodicidade das Projeções, %
Contribuições. Por regra: Possível Aposentadoria, Provável RMI, Investimento,
Retorno sobre o investimento, Detalhamento (benefício proporcional no mês da
DIB, 13º proporcional, N benefícios mensais antes/depois dos 65), Expectativa
de Sobrevida.

### 7. Regras de Transição - EC 103/2019
Cards por regra com: Possível Aposentadoria, Provável RMI, Idade possuída,
Tempo possuído, Tempo Mínimo exigido, Retorno do Investimento.

## Espécies observadas
- 42 - Aposentadoria Por Tempo de Contribuição ou Programada
- 32 - Invalidez ou Incapacidade Permanente Previdenciária
- 31 - Auxílio Doença Previdenciário ou Incapacidade Temporária

## Ordem no documento consolidado
Tempo de Contribuição -> Qualidade de Segurado -> Regras de Transição ->
Cálculo da R.M.I. -> Análise da Situação Atual -> Retorno do Investimento.
