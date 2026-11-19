# Sessão autenticada IEPREV: observações estruturais

Data: 28/10/2026. A sessão foi executada em navegador isolado com uma conta
autorizada. Nenhum nome, CPF, NIT, empregador, salário, RMI, PDF ou token foi
copiado para este documento ou para o repositório. As respostas brutas ficaram
em `work/`, que é ignorada pelo Git.

## Pro: importação e cálculo

- A importação do CNIS produziu **23 períodos** e preencheu identidade, CPF,
  NIT, nascimento e nome da mãe.
- A base de cálculo do Pro (`PBC`) continha **320 contribuições** desde
  07/1994; **319** foram selecionadas e **1** foi excluída da média.
- Havia **7 contribuições concomitantes** e **2** valores limitados ao teto.
- A regra de pedágio de 50% foi considerada **elegível**. O resultado exibiu
  uma renda e um fator previdenciário, mas os valores do cliente não são
  registrados aqui.
- O detalhe do cálculo expôs `optimization_summary`: uma contribuição baixa foi
  excluída da média e o tempo caiu **30 dias**. A tela do resultado mostrou o
  tempo anterior à otimização; a API usou o tempo posterior.
- Antes de calcular, o Pro perguntou se competências vazias deveriam ser
  ajustadas ao salário mínimo. Foi escolhida a opção **“Continuar sem ajustar”**.
- Foram exibidos **12 benefícios**, incluindo regras de transição, regras
  pré-reforma e benefícios por incapacidade. Pedágio de 100%, idade
  progressiva, pontos e regras pré-reforma apareceram como não elegíveis, com
  lacunas e datas projetadas.

## Calc 1.0: importação e cálculo

- O Calc importou **26 linhas**: 23 períodos com datas e 3 linhas “Benefício”
  vazias. A linha vazia é um estado de interface, não um período novo.
- O Calc **não preencheu automaticamente** sexo e data de filiação; ambos eram
  obrigatórios. Para prosseguir, foi usada a data do primeiro período como
  filiação.
- Ao encontrar competências sem salário ou abaixo do mínimo, o Calc ofereceu
  limitar ao salário mínimo ou voltar ao passo 4. Não havia uma opção
  equivalente ao “continuar sem ajustar” do Pro. Foi escolhida a limitação para
  concluir o fluxo, portanto o resultado **não é comparável** ao do Pro.
- A contagem exibiu **39 anos, 8 meses e 28 dias** no total, **477 meses** de
  carência e **33 anos, 1 mês e 11 dias** até a EC 103/2019. Também mostrou
  marcos históricos (1988, 1991, 1994, 1998, 1999 e EC 103).
- O resultado da regra de pedágio de 50% ofereceu cinco cenários de PBC:
  **352, 360, 368, 376 e 382** salários de contribuição. O cenário de 382 foi
  marcado como mais vantajoso. Esses números refletem a limitação ao mínimo
  feita no passo anterior.
- O Calc separou os resultados em quatro grupos de “direito adquirido” por
  período e não ofereceu a mesma visão consolidada de 12 benefícios do Pro.

## Diferenças que importam para o previdenciar.io

1. **Tratamento de competências vazias**: o Pro permite continuar sem ajuste;
   o Calc força limitar ou editar. O previdenciar.io deve manter as duas
   escolhas explícitas e nunca preencher valor silenciosamente.
2. **Otimização da PBC**: o Pro remove contribuições baixas para melhorar a
   média, alterando também o tempo. O Calc oferece vários cortes de PBC. O
   previdenciário deve mostrar a memória antes/depois e exigir confirmação.
3. **Tempo exibido × tempo usado**: o Pro mostrou o tempo pré-otimização na
   tela e usou o pós-otimização no cálculo. Isso precisa ser corrigido no
   produto próprio com um único valor e um aviso de descarte.
4. **Importação incompleta**: o Calc criou três linhas de benefício vazias e
   exigiu dados que o CNIS já tinha. O importador próprio deve separar
   benefícios de vínculos e não criar linhas vazias.
5. **Regras e benefícios**: o Pro cobre mais espécies e uma visão consolidada;
   o Calc oferece detalhamento histórico mais rico e vários cenários de PBC.
6. **Comparação numérica**: os dois resultados não podem ser tratados como
   iguais porque partiram de tratamentos diferentes de salários mínimos. Uma
   comparação válida exige repetir o mesmo cenário com o mesmo ajuste.

## Efeitos colaterais

- Foram criados dois cálculos de auditoria na conta: um no Pro e um no Calc.
  Eles contêm o CNIS real e permanecem privados. Não foram apagados sem
  autorização.
- O arquivo de cópia para upload está em `work/CNIS-upload.pdf`, ignorado pelo
  Git. Não deve ser commitado nem enviado a terceiros.

## Aplicação no previdenciar.io

- O resultado agora exibe reconciliação da fonte, PBC, parcelas descartadas,
  tempo efetivo por vínculo e regras de transição comparadas.
- O adaptador registra quantas competências foram recebidas, quantas ficaram
  fora por serem anteriores a 07/1994, quantas ficaram abaixo do mínimo e
  quantas chegaram ao PBC.
- O CNIS separa benefícios de vínculos em uma aba própria; benefícios não são
  contados como tempo de contribuição.
- A comparação de médias com cortes de 0%, 10% e 20% é apenas descritiva. Ela
  não escolhe regra, coeficiente ou fator nem reduz tempo ou carência.

## Próximos experimentos

1. Repetir o cenário no Calc com o mesmo critério de não ajustar, se a interface
   permitir, ou registrar a impossibilidade como limitação.
2. Comparar o relatório PDF do Pro com o relatório do Calc, verificando
   memória, fundamento, tempo e arredondamentos — sem publicar valores.
3. Testar as fronteiras de 13/11/2019 e uma competência com salário zero.
4. Transformar as diferenças acima em testes sintéticos do previdenciar.io.
