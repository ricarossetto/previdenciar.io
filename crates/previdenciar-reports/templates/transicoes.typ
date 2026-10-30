#import "_base.typ": *
#import sys: inputs

#let d = inputs.data

#let rule-card(r) = {
  let chosen = r.chosen
  rect(
    width: 100%,
    inset: (x: 10pt, y: 8pt),
    fill: if chosen { green-soft } else { white },
    stroke: (0.6pt + (if chosen { green } else { gray-line })),
    radius: 3pt,
  )[
    #grid(
      columns: (1fr, auto),
      align: (left, right),
      text(size: fs-value, weight: "bold", fill: navy)[#r.nome],
      if chosen { text(size: fs-body, weight: "bold", fill: rgb("#2E6B2E"))[Regra Escolhida] } else { [] },
    )
    #v(5pt)
    #grid(
      columns: (1fr, 1fr, 1fr, 1fr),
      column-gutter: 8pt,
      [#text(size: fs-caption, fill: gray-text)[Tempo de Contribuição] #linebreak() #text(size: fs-body)[#r.tempo]],
      [#text(size: fs-caption, fill: gray-text)[Carência] #linebreak() #text(size: fs-body)[#r.carencia]],
      [#text(size: fs-caption, fill: gray-text)[Idade] #linebreak() #text(size: fs-body)[#r.idade]],
      [#text(size: fs-caption, fill: gray-text)[Provável RMI] #linebreak() #text(size: fs-body, weight: "bold", fill: navy)[#r.rmi]],
    )
    #v(4pt)
    #text(size: fs-body, fill: gray-text)[Possível aposentadoria em #r.der]
  ]
}

#report(title: d.title, identification: d.identification, date: d.date)[
  #v(2pt)
  #for r in d.regras [
    #rule-card(r)
    #v(8pt)
  ]
  #v(6pt)
  #text(size: fs-caption, fill: gray-text)[#d.nota]
]
