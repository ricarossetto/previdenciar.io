#import "_base.typ": *
#import sys: inputs

#let d = inputs.data

#let card(r) = rect(
  width: 100%,
  inset: (x: 10pt, y: 8pt),
  fill: white,
  stroke: 0.6pt + gray-line,
  radius: 3pt,
)[
  #text(size: fs-value, weight: "bold", fill: navy)[#r.nome]
  #v(6pt)
  #grid(
    columns: (1fr, 1fr),
    column-gutter: 10pt,
    row-gutter: 5pt,
    [#text(size: fs-caption, fill: gray-text)[Possível Aposentadoria:] #linebreak() #text(size: fs-body, weight: "bold")[#r.der]],
    [#text(size: fs-caption, fill: gray-text)[Provável RMI:] #linebreak() #text(size: fs-body, weight: "bold")[#r.rmi]],
    [#text(size: fs-caption, fill: gray-text)[Idade (Necessário #r.idade_necessaria):] #linebreak() #text(size: fs-body)[#r.idade_possui]],
    [#text(size: fs-caption, fill: gray-text)[Tempo Mínimo de #r.tempo_minimo:] #linebreak() #text(size: fs-body)[#r.tempo_possui]],
  )
  #v(5pt)
  #text(size: fs-body, weight: "bold")[Retorno do Investimento: #r.retorno]
]

#report(title: d.title, identification: d.identification, date: d.date)[
  #v(2pt)
  #for r in d.regras [
    #card(r)
    #v(8pt)
  ]
  #v(6pt)
  #text(size: fs-caption, fill: gray-text)[#d.nota1]
  #v(3pt)
  #text(size: fs-caption, fill: gray-text)[#d.nota2]
]
