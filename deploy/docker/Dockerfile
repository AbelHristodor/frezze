FROM rust:bookworm AS chef

LABEL org.opencontainers.image.title=frezze
LABEL org.opencontainers.image.version=latest
LABEL org.opencontainers.image.source=https://github.com/AbelHristodor/frezze
LABEL org.opencontainers.image.description="Github bot to freeze your repos!"
LABEL org.opencontainers.image.licenses=GPL-3.0-or-later

RUN cargo install cargo-chef 
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare  --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

RUN apt-get update && apt-get install -y pkg-config libssl-dev

# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json

# Build application
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin frezze

FROM debian:bookworm AS runtime

RUN apt-get update && apt-get install -y \
  libssl3 \
  ca-certificates \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Create db directory for SQLite database
RUN mkdir -p /app/db

COPY --from=builder /app/target/release/frezze /usr/local/bin
COPY --from=builder /app/migrations /app/migrations

ENV MIGRATIONS_PATH=/app/migrations

ENTRYPOINT ["/usr/local/bin/frezze"]
