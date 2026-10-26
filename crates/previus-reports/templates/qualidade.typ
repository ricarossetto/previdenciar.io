#import "_base.typ": *
#import sys: inputs

#let d = inputs.data

#report(title: d.title, identification: d.identification, date: d.date)[
  #v(2pt)
  #data-table(d.headers, d.rows)
  #v(8pt)
  #for t in d.totais [
    #text(size: 8.5pt)[*#t.at(0):* #t.at(1)]
    #v(3pt)
  ]
  #v(10pt)
  #text(size: 7.5pt, fill: gray-text)[#d.nota]
]
