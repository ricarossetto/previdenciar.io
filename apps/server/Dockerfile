FROM rust:1.99-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY apps/server ./apps/server
COPY apps/desktop/src-tauri ./apps/desktop/src-tauri
RUN cargo build --release --locked -p previdenciar-web-server

FROM debian:bookworm-slim
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates poppler-utils tesseract-ocr tesseract-ocr-por \
  && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /app/target/release/previdenciar-web-server /app/previdenciar-web-server
COPY apps/desktop/dist /app/web
ENV WEB_DIST=/app/web
ENV BIND_ADDR=0.0.0.0:8787
EXPOSE 8787
CMD ["/app/previdenciar-web-server"]
