# ---- planner: compute the dependency recipe ----
FROM rust:1.85-slim AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ---- builder: deps cached as their own layer, then the app ----
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release -p web

# ---- runtime ----
FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN useradd --system --create-home appuser
USER appuser
WORKDIR /home/appuser
COPY --from=builder /app/target/release/web /usr/local/bin/glossarium
ENV BIND_ADDR=0.0.0.0:8080 \
    DATABASE_URL=sqlite:///data/conlang.db
EXPOSE 8080
VOLUME ["/data"]
CMD ["glossarium"]
