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

FROM gcr.io/distroless/cc-debian12

COPY --from=builder /app/target/release/frezze /
COPY --from=builder /app/migrations /migrations

ENV MIGRATIONS_PATH=/migrations

ENTRYPOINT ["/frezze"]
