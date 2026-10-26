// Base comum dos relatórios: cores, cabeçalho, rodapé e componentes.
// Espelha o layout do produto de referência (logo, campos em caixa,
// tabelas com cabeçalho azul-marinho, barras de status).

#let navy = rgb("#1F3A5F")
#let navy-soft = rgb("#E8EDF3")
#let gray-bg = rgb("#F2F3F5")
#let gray-line = rgb("#D5D8DC")
#let gray-text = rgb("#5A6472")
#let green = rgb("#5CB85C")
#let green-soft = rgb("#E3F2E3")

// Campo rotulado: rótulo pequeno em cima, valor em caixa clara.
#let data-field(label, value) = block(width: 100%)[
  #text(size: 7pt, fill: gray-text)[#label]
  #v(1.5pt)
  #box(
    width: 100%,
    inset: (x: 5pt, y: 3.5pt),
    fill: gray-bg,
    stroke: 0.5pt + gray-line,
    radius: 2pt,
  )[#text(size: 8.5pt, weight: "bold")[#value]]
]

// Bloco de identificação: cada linha define quantas colunas tem.
#let identification-block(d) = {
  let rows = d.rows
  for row in rows {
    grid(
      columns: row.len(),
      column-gutter: 8pt,
      ..row.map(pair => data-field(pair.at(0), pair.at(1))),
    )
    v(5pt)
  }
}

#let section-title(t) = align(center)[
  #text(size: 11pt, weight: "bold")[#t]
]

#let subsection(t) = text(size: 9.5pt, weight: "bold")[#t]

// Tabela com cabeçalho azul-marinho e linhas alternadas.
#let data-table(headers, rows, weights: none) = {
  let cols = if weights == none { headers.map(_ => 1fr) } else { weights }
  let n = headers.len()
  table(
    columns: cols,
    inset: (x: 5pt, y: 3.5pt),
    align: (left,) * n,
    stroke: 0.4pt + gray-line,
    fill: (col, row) => if row == 0 { navy } else if calc.even(row) { gray-bg } else { white },
    table.header(..headers.map(h => text(fill: white, weight: "bold", size: 8pt)[#h])),
    ..rows.flatten().map(cell => text(size: 8pt)[#cell]),
  )
}

// Barra de status (verde = atingido, cinza = não exigido).
#let status-bar(label, ok: true) = rect(
  width: 100%,
  inset: (x: 6pt, y: 5pt),
  fill: if ok { green-soft } else { gray-bg },
  stroke: 0pt,
  radius: 3pt,
  align(center, text(size: 8.5pt, weight: "bold", fill: if ok { rgb("#2E6B2E") } else { gray-text })[#label]),
)

// Relatório padrão: cabeçalho (logo + identificação) no topo do corpo,
// rodapé com data e n/N em cada página.
#let report(title: "", identification: (:), date: "", body) = {
  set page(
    paper: "a4",
    margin: (top: 1.5cm, bottom: 1.8cm, x: 1.5cm),
    footer: context [
      #set text(size: 7.5pt, fill: gray-text)
      #grid(
        columns: (1fr, 1fr),
        align: (left, right),
        [#date],
        [#counter(page).display() / #counter(page).final().first()],
      )
    ],
  )
  align(center)[#image("logo.png", width: 46%)]
  v(3pt)
  align(center)[#text(size: 12pt, weight: "bold")[#title]]
  v(5pt)
  identification-block(identification)
  v(2pt)
  line(length: 100%, stroke: 0.5pt + gray-line)
  v(10pt)
  body
}
