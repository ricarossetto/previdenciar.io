## previdenciar.io 0.14.0

O motor deixou de atender só a espécie 42. Aposentadoria por idade (41),
incapacidade permanente (32) e auxílio por incapacidade temporária (31) agora
têm fórmula, requisitos e base legal próprios. O que não tem regra implementada
continua bloqueado, com motivo — nunca aproximado.

### O que passou a funcionar

- **41 — Aposentadoria por idade**: 62/65 anos (60/55 rural), 180 contribuições,
  regra permanente e regra do art. 18 da EC 103 para quem já era filiado em
  13/11/2019. Coeficiente de 60% + 2% por ano acima de 15/20.
- **32 — Incapacidade permanente**: carência de 12 ou isenção por acidente,
  doença ocupacional ou doença da lista; 60% + 2% na regra comum e 100% na
  origem acidentária. A DER anterior à reforma usa o art. 44 e avisa sobre a DII
  e o Tema 1300 do STF.
- **31 — Incapacidade temporária**: 91% do salário de benefício, limitado à
  média dos últimos 12 salários (art. 29, § 10).
- **Requisitos na tela e no PDF**: idade, carência, tempo, incapacidade e
  origem aparecem com valor exigido, apurado, meses faltantes e a marca
  "declarado, não verificado pelo motor" quando é o caso.
- **Escolha da espécie** no editor e na entrevista guiada, com a origem da
  incapacidade no editor de política.

### O que continua bloqueado, de propósito

- **46 — Especial**: exige PPP, enquadramento e 15/20/25 anos de exposição.
- **57 — Professor**: exige comprovação exclusiva de magistério.
- Pensão por morte, salário-maternidade, auxílio-acidente e demais prestações
  não são aposentadorias e não têm fórmula neste motor.
- A incapacidade é declarada. O programa calcula a renda; não faz perícia nem
  certifica nexo, DII ou qualidade de segurado.

### Também nesta versão

- A matriz completa está em `docs/COVERAGE.md`.
- O resumo do PDF mostra a espécie e a regra escolhida; a memória mostra cada
  requisito.
- 495 testes Rust, 111 de interface e 36 de navegador.

### Limites

- O direito adquirido da espécie 42 continua sem recálculo da RMI antiga (80%
  maiores e fator/85-95); ele aparece como elegível, sem valor.
- O módulo PcD da LC 142/2013 continua separado e com premissas declaradas.
- A distribuição não tem assinatura de código; confira o SHA-256.
