// Base comum dos relatórios: cores, cabeçalho, rodapé e componentes.
// Espelha o layout do produto de referência (logo, campos em caixa,
// tabelas com cabeçalho azul-marinho, barras de status).

#let navy = rgb("#1F3A5F")
#let navy-soft = rgb("#E8EDF3")
#let gray-bg = rgb("#F2F3F5")
#let gray-line = rgb("#D5D8DC")
#let gray-text = rgb("#4A5568")
#let green = rgb("#5CB85C")
#let green-soft = rgb("#E3F2E3")
#let green-text = rgb("#2E6B2E")

// Escala tipográfica única. Todo texto usa um destes passos.
#let fs-caption = 7.5pt
#let fs-body = 9pt
#let fs-value = 10pt
#let fs-heading = 11pt
#let fs-title = 12pt
#let fs-highlight = 15pt

// Campo rotulado: rótulo pequeno em cima, valor em caixa clara.
#let data-field(label, value) = block(width: 100%)[
  #text(size: fs-caption, fill: gray-text)[#label]
  #v(1.5pt)
  #box(
    width: 100%,
    inset: (x: 5pt, y: 3.5pt),
    fill: gray-bg,
    stroke: 0.5pt + gray-line,
    radius: 2pt,
  )[#text(size: fs-body, weight: "bold")[#value]]
]

// Bloco de identificação: cada linha define quantas colunas tem.
#let identification-block(d) = {
  for row in d.rows {
    grid(
      columns: row.len(),
      column-gutter: 8pt,
      ..row.map(f => data-field(f.at(0), f.at(1))),
    )
    v(5pt)
  }
}

#let section-title(t) = align(center)[#text(size: fs-heading, weight: "bold")[#t]]

#let subsection(t) = text(size: fs-value, weight: "bold")[#t]

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
    table.header(..headers.map(h => text(fill: white, weight: "bold", size: fs-caption)[#h])),
    ..rows.flatten().map(cell => text(size: fs-body)[#cell]),
  )
}

// Barra de status (verde = atingido, cinza = não exigido).
#let status-bar(label, ok: true) = rect(
  width: 100%,
  inset: (x: 6pt, y: 5pt),
  fill: if ok { green-soft } else { gray-bg },
  stroke: 0pt,
  radius: 3pt,
  align(center, text(size: fs-body, weight: "bold", fill: if ok { green-text } else { gray-text })[#label]),
)

// Relatório padrão. Cabeçalho (logo + título) repete em cada página; a
// identificação fica no corpo (limitação do header do Typst com conteúdo gerado).
#let report(title: "", identification: (:), date: "", body) = {
  set page(
    paper: "a4",
    margin: (top: 3.2cm, bottom: 1.8cm, x: 1.5cm),
    header: [
      #align(center)[#image("logo.png", width: 40%)]
      #v(2pt)
      #align(center)[#text(size: fs-title, weight: "bold")[#title]]
    ],
    footer: context [
      #set text(size: fs-caption, fill: gray-text)
      #grid(
        columns: (1fr, 1fr),
        align: (left, right),
        [#date],
        [#counter(page).display() / #counter(page).final().first()],
      )
    ],
  )
  identification-block(identification)
  v(2pt)
  line(length: 100%, stroke: 0.5pt + gray-line)
  v(10pt)
  body
}
