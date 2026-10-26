#import "_base.typ": *
#import sys: inputs

#let d = inputs.data

#let field(label, value) = rect(
  width: 100%,
  inset: (x: 7pt, y: 5pt),
  fill: white,
  stroke: 0.5pt + gray-line,
  radius: 2pt,
  stack(dir: ttb, spacing: 3pt,
    text(size: fs-caption, fill: gray-text)[#label],
    text(size: fs-value, weight: "bold")[#value],
  ),
)

#report(title: d.title, identification: d.identification, date: d.date)[
  #for (i, r) in d.regras.enumerate() [
    #if i > 0 { pagebreak() }
    #rect(width: 100%, inset: (x: 10pt, y: 6pt), fill: navy-soft, radius: 3pt)[
      #text(size: fs-title, weight: "bold", fill: navy)[#r.nome]
    ]
    #v(10pt)
    #grid(
      columns: (1fr, 1fr),
      column-gutter: 8pt,
      row-gutter: 8pt,
      field("Possível Aposentadoria", r.der),
      field("Investimento", r.investimento),
      field("Provável RMI", r.rmi),
      field("Total líquido recebido", r.total),
    )
    #v(10pt)
    #text(size: fs-body, weight: "bold")[Detalhamento:]
    #v(4pt)
    #rect(width: 100%, inset: 8pt, fill: gray-bg, stroke: 0.5pt + gray-line, radius: 2pt)[
      #for line in r.detalhe [
        #text(size: fs-body)[#line]
        #linebreak()
      ]
    ]
    #v(10pt)
    #text(size: fs-body, fill: gray-text)[#r.expectativa]
  ]
  #v(6pt)
  #text(size: fs-caption, fill: gray-text)[#d.nota]
]
