# --- Build stage ---
FROM rust:latest AS builder

WORKDIR /app
COPY . .
RUN cargo build --release

# --- Runtime stage ---
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/sat-api /usr/local/bin/sat-api

EXPOSE 8000

CMD ["sat-api"]
