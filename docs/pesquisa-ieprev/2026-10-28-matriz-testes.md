# Matriz de testes previdenciários

Data: 28/10/2026. A matriz define **entradas e critérios**. Nenhum resultado
numérico de referência será registrado sem execução observada e fonte jurídica.

## Cenários mínimos

| ID | Perfil | Regra em foco | Entradas que precisam ser explícitas | Critério de observação |
|---|---|---|---|---|
| RGPS-01 | Homem, 65 anos, 20 anos de contribuição | Idade permanente pós-EC 103 | nascimento, DIB, sexo, vínculos, competências, PBC | elegibilidade, carência, coeficiente, RMI e fonte |
| RGPS-02 | Mulher, 62 anos, 15 anos | Idade permanente pós-EC 103 | idem | limites e memória |
| RGPS-03 | Homem, 35 anos de tempo antes de 13/11/2019 | Direito adquirido | tempo até a reforma, PBC, DIB | regra antiga e fator previdenciário |
| RGPS-04 | Mulher, 28 anos antes, 2 anos faltantes | Pedágio de 50% | tempo até a reforma, expectativa de sobrevida | pedágio, fator e RMI |
| RGPS-05 | Homem, 33 anos antes | Pedágio de 100% | tempo até a reforma, DIB | idade mínima, pedágio e coeficiente |
| RGPS-06 | Mulher, 30 anos, 58 anos | Pontos | idade, tempo, ano da DIB | pontos exigidos, faltas e RMI |
| RGPS-07 | Homem, 35 anos, 61 anos | Idade progressiva | idade, tempo, ano | idade exigida, faltas e coeficiente |
| RGPS-08 | Professor, 25 anos | Regra de professor | sexo, tempo de magistério, DIB | requisito específico e fundamento |
| RGPS-09 | Atividade especial 25 anos | Especial | períodos, agente nocivo, PPP/CNIS | elegibilidade e conversão |
| RGPS-10 | PcD grau moderado | LC 142/2013 | grau, período, laudo, sexo, DIB | cenário bloqueado/experimental |
| RGPS-11 | Rural + urbano | Híbrida/rural | períodos rurais, provas, DIB | enquadramento e contagem |
| RGPS-12 | Competências concomitantes | PBC e carência | dois vínculos no mesmo mês | soma, duplicidade e aviso |
| RGPS-13 | Lacuna contributiva | Qualidade de segurado | filiação, último vínculo, DIB | período de graça e carência |
| RGPS-14 | Benefício anterior | Revisão/benefício atual | espécie, DIB, DCB, RMI | bloqueio de hipótese incompatível |
| RGPS-15 | Data exatamente 13/11/2019 | Marco temporal | DIB antes/depois | regra aplicável e justificativa |

## Regras de execução

- Um campo por vez; registrar antes/depois.
- Não publicar nome, CPF, NIT, empregador, salário ou PDF privado.
- Separar: importador, motor, adaptador, renderer de PDF e interface.
- Uma coincidência numérica não prova regra; a fonte normativa e a memória
  precisam concordar.
- Se a plataforma de referência não expuser a memória, registrar apenas o
  resultado observado como hipótese, não como fórmula comprovada.

## Saídas esperadas

Para cada cenário: elegibilidade por regra, requisitos faltantes, data de
aquisição, PBC, coeficiente, RMI, teto/piso, avisos, fundamento e hash do
documento de origem privado (não o conteúdo).

## Verificação local desta sessão

- `cargo test --workspace --locked`: exit 0, 493 testes aprovados.
- `cargo test -p previdenciar-cnis --locked`: 38 testes aprovados.
- `npm test` em `apps/desktop`: 108 testes aprovados.
- `npm run test:e2e` em `apps/desktop`: 36 testes aprovados.
- O CNIS privado foi lido localmente e não foi incluído no repositório.
