# Cenários PcD de grau constante

O adaptador `previus-rgps::pcd` é um cálculo local independente e experimental, não uma certificação de direito. Sem a política versionada `lc142-constant-grade-v1` e a confirmação documental das premissas, o cenário continua bloqueado (`data: null`). O caminho RGPS anterior não é usado como substituto automático para PcD.

## Entradas e separação de origem

- O CNIS original não é modificado. Vínculos sobrepostos são unidos; regimes próprios, CTC e graus mistos não são convertidos automaticamente.
- Sexo, grau constante, período da deficiência, regime e revisão da origem precisam ser declarados. Não são extraídos como conclusões médicas ou jurídicas do CNIS.
- Fim de vínculo ausente não vira uma data confirmada. A continuidade até a DIB só entra mediante premissa explícita.
- Lacunas salariais e projeções futuras ficam identificadas separadamente. Não se infere salário dividindo contribuição por alíquota e não se considera dado ausente igual a zero.
- Salários manuais precisam de livro de contribuições validado e aplicado. Sem ele, o cálculo bloqueia em vez de ignorar silenciosamente a edição.
- Alterar dados revoga a confirmação das premissas e o resultado anterior. Uma consulta iniciada antes de outra edição não reaplica o rascunho antigo.

## Perfil matemático comparativo

A data-base define a referência conhecida dos índices; a DIB define o corte do cenário. Não trocar essas duas datas nem resolver divergências documentais pelo nome de uma pasta.

1. Parcelas entre o primeiro salário suportado a partir de 1994-07 e o mês anterior à DIB, somente em períodos considerados.
2. Limitação de cada salário ao teto da competência. Limites futuros, quando a projeção foi autorizada, ficam congelados na data-base e não são valores oficiais futuros.
3. Correção composta com IPC-r até 1995-06, INPC até 1996-03, IGP-DI até 2004-01 e INPC depois, com exceções INSS opcionais do provedor. O método histórico de variações negativas é uma opção declarada do perfil comparativo, não uma conclusão normativa automática.
4. O perfil usa a parte inteira de 80% da quantidade de parcelas, ordenadas pelo valor corrigido e com desempate cronológico. Essa escolha fica registrada no PDF e depende de revisão profissional para aplicação fora do estudo comparativo.
5. Os dias exibidos por competência são derivados da cobertura dos períodos unidos, inclusive meses parciais. Dinheiro usa aritmética decimal e arredondamento em centavos, com meio afastado de zero. O fator exibido é truncado em seis casas, conforme o perfil de referência; o salário usa o fator integral antes do arredondamento monetário.
6. Tempo usa convenção declarada 30/360 inclusiva com união de períodos. Carência do cenário considera competências cobertas pelos períodos, incluindo projeções declaradas; isso não certifica que cada competência é juridicamente válida para carência.
7. Coeficiente declarado de 100%, com piso/teto conhecidos. Sem os requisitos do cenário, a RMI fica nula. Fator previdenciário, graus mistos, atividade especial e comparação automática de todas as regras não estão implementados neste caminho.

## Consulta pública

O comando nativo `fetch_economic_snapshot` recebe somente `through` (`AAAA-MM`) e consulta os endpoints fixos `indices/api/v1/tabela` e `indices/api/v1/limites` da Fábrica de Cálculos/TRF3. Não lê `.env`, não recebe chave e não envia dados de caso. Usa negociação HTTP explícita de JSON, idioma e conteúdo sem compressão, mantendo a validação normal de certificados TLS. Não segue redirecionamentos, limita tamanho/tempo de resposta e rejeita dados incompletos ou posteriores ao corte. A conferência de RMI é outro comando, opcional e com chave própria; ela não altera esse snapshot público.

O snapshot registra URL, corte, instante e SHA-256 das duas respostas. Após a consulta, cálculo e PDF usam o snapshot local. Metadados de proveniência não substituem uma assinatura do provedor nem prova de atualização normativa.

Um HTTP 200 em catálogo público não valida uma credencial. Consultar ou enviar casos a endpoints autenticados é um fluxo diferente, não implementado aqui.

## Verificação reproduzível

`cargo test --locked --workspace` cobre o motor, guardas, persistência e PDFs reais com fixtures inteiramente sintéticas. `npm test` e `npm run test:e2e`, em `apps/desktop`, cobrem a interface; os testes de navegador usam IPC sintético, não o backend nativo.

O exemplo `private_scenario_reports` recebe explicitamente um JSON privado e um diretório novo, não sobrescreve evidências e executa cálculo e renderer Rust reais. Dados reais, comparações e PDFs nunca devem entrar no repositório público. A verificação em executável deve usar perfil isolado, preservar a base normal e declarar separadamente o que foi testado via IPC, interface e diálogo de salvar.
