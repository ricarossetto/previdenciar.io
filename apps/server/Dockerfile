# O site é gerado aqui, sempre em modo web. Copiar o dist local já levou ao ar
# o pacote do aplicativo desktop (sem tela de entrada e com a base bloqueada).
FROM node:22-bookworm-slim AS web
WORKDIR /app/apps/desktop
COPY apps/desktop/package.json apps/desktop/package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY scripts /app/scripts
COPY apps/desktop/index.html apps/desktop/tsconfig.json apps/desktop/vite.config.ts ./
COPY apps/desktop/public ./public
COPY apps/desktop/src ./src
# Identificadores públicos do Cognito; os demais já têm padrão no código.
ARG VITE_COGNITO_GOOGLE=1
ENV VITE_COGNITO_GOOGLE=$VITE_COGNITO_GOOGLE
RUN npm run build:web \
  && grep -q "atrium-login" dist/assets/index-*.js

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
COPY --from=web /app/apps/desktop/dist /app/web
ENV WEB_DIST=/app/web
ENV BIND_ADDR=0.0.0.0:8787
EXPOSE 8787
CMD ["/app/previdenciar-web-server"]
