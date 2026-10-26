#import "_base.typ": *
#import sys: inputs

#let d = inputs.data

#report(title: d.title, identification: d.identification, date: d.date)[
  #v(2pt)
  #section-title(d.subtitle)
  #v(6pt)
  #if d.rows.len() > 0 [
    #data-table(d.headers, d.rows)
    #v(14pt)
  ]
  #grid(
    columns: (1fr, 1fr),
    column-gutter: 10pt,
    row-gutter: 6pt,
    ..d.resumo.map(p => rect(
      width: 100%,
      inset: (x: 6pt, y: 4pt),
      fill: gray-bg,
      stroke: 0.5pt + gray-line,
      radius: 2pt,
      stack(dir: ttb, spacing: 3pt,
        text(size: 8pt, fill: gray-text)[#p.at(0)],
        text(size: 10pt, weight: "bold", fill: navy)[#p.at(1)],
      ),
    )),
  )
]
