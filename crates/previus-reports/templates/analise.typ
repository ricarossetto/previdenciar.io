#import "_base.typ": *
#import sys: inputs

#let d = inputs.data

#let highlight-cell(label, value) = rect(
  width: 100%,
  inset: (x: 8pt, y: 6pt),
  fill: white,
  stroke: 0.6pt + gray-line,
  radius: 3pt,
  stack(dir: ttb, spacing: 4pt,
    align(center, text(size: 8.5pt, fill: gray-text)[#label]),
    align(center, text(size: 14pt, weight: "bold", fill: navy)[#value]),
  ),
)

#let req-row(r) = grid(
  columns: (auto, 1fr),
  column-gutter: 10pt,
  align: (left + horizon, left),
  rect(
    inset: (x: 8pt, y: 6pt),
    fill: gray-bg,
    stroke: 0.5pt + gray-line,
    radius: 3pt,
    stack(dir: ttb, spacing: 2pt,
      text(size: 7.5pt, fill: gray-text)[#r.label],
      text(size: 10pt, weight: "bold")[#r.value],
    ),
  ),
  status-bar(r.status, ok: r.ok),
)

#let kv-table(pairs) = grid(
  columns: (1fr, 1fr, 1fr),
  column-gutter: 6pt,
  row-gutter: 6pt,
  ..pairs.map(p => rect(
    width: 100%,
    inset: (x: 6pt, y: 4pt),
    fill: gray-bg,
    stroke: 0.5pt + gray-line,
    radius: 2pt,
    stack(dir: ttb, spacing: 3pt,
      align(center, text(size: 8pt, fill: gray-text)[#p.at(0)]),
      align(center, text(size: 9.5pt, weight: "bold")[#p.at(1)]),
    ),
  )),
)

#report(title: d.title, identification: d.identification, date: d.date)[
  #v(4pt)
  #grid(
    columns: (1fr, 1fr),
    column-gutter: 8pt,
    ..d.highlight.map(h => highlight-cell(h.at(0), h.at(1))),
  )
  #v(12pt)

  #section-title("ANÁLISE DOS REQUISITOS PARA CONCESSÃO DE SUA APOSENTADORIA")
  #v(6pt)
  #stack(dir: ttb, spacing: 6pt, ..d.requirements.map(req-row))
  #v(12pt)

  #subsection("PLANEJAMENTO PARA A SUA APOSENTADORIA")
  #v(4pt)
  #text(size: 9pt, weight: "bold")[A PAGAR:]
  #v(3pt)
  #kv-table(d.a_pagar)
  #v(8pt)
  #text(size: 9pt, weight: "bold")[A RECEBER:]
  #v(3pt)
  #kv-table(d.a_receber)

  #v(28pt)
  #align(center)[
    #line(length: 45%, stroke: 0.5pt + black)
    #v(2pt)
    #text(size: 9pt)[#d.signature]
  ]
]
