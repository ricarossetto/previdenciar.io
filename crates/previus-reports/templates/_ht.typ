#let df(l, v) = block[
  #text(size: 7pt)[#l]
  #v(1pt)
  #box(width: 100%, fill: rgb("#eeeeee"), inset: 4pt)[#v]
]

#let rows-block(rows) = {
  for row in rows {
    grid(columns: row.len(), column-gutter: 8pt, ..row.map(f => df(f.at(0), f.at(1))))
    v(5pt)
  }
}

#let data-rows = (
  (("Label A", "Valor A"), ("Label B", "Valor B")),
  (("Label C", "Valor C"),),
)

#set page(
  paper: "a4",
  margin: (top: 5cm, x: 1.5cm),
  header: context [
    #align(center)[#text(size: 12pt, weight: "bold")[TESTE HEADER]]
    #v(5pt)
    #rows-block(data-rows)
  ],
)
#lorem(120)
