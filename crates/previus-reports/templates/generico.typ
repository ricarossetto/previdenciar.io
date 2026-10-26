#import "_base.typ": *
#import sys: inputs

#let d = inputs.data

#report(title: d.title, identification: d.identification, date: d.date)[
  #for s in d.sections [
    #section-title(s.titulo)
    #v(5pt)
    #for p in s.paragrafos [
      #text(size: fs-body)[#p]
      #v(4pt)
    ]
    #if s.tabela != none [
      #data-table(s.tabela.headers, s.tabela.rows)
      #v(8pt)
    ]
  ]
]
