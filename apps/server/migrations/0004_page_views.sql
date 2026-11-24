-- Contagem anônima de páginas, só com consentimento do visitante.
-- Guarda apenas dia, página e total: nenhum IP, usuário, cookie ou identificador.
CREATE TABLE IF NOT EXISTS page_views (
  day date NOT NULL,
  page text NOT NULL,
  views bigint NOT NULL DEFAULT 0,
  PRIMARY KEY (day, page)
);
