#import "_base.typ": *
#import sys: inputs

#let d = inputs.data

#report(title: d.title, identification: d.identification, date: d.date)[
  #v(2pt)
  #section-title("Períodos Informados")
  #v(6pt)
  #data-table(d.informados.headers, d.informados.rows)
  #v(5pt)
  #text(size: 7.5pt, fill: gray-text)[#d.legenda_informados]
  #v(16pt)

  #section-title("Períodos Considerados no Cálculo")
  #v(6pt)
  #data-table(d.considerados.headers, d.considerados.rows)
  #v(5pt)
  #text(size: 7.5pt, fill: gray-text)[#d.legenda_considerados]
  #v(10pt)
  #for t in d.totais [
    #text(size: 8.5pt)[*#t.at(0):* #t.at(1)]
    #v(3pt)
  ]
  #v(16pt)

  #section-title("Análise dos Dados")
  #v(6pt)
  #for a in d.analise [
    #text(size: 8.5pt)[*#a.at(0):* #a.at(1)]
    #v(4pt)
  ]
]
