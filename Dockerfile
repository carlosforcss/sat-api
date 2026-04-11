FROM rust:latest

WORKDIR /app

# Cache dependencies separately from source code.
# Only re-runs when Cargo.toml/Cargo.lock change.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build && rm -rf src

COPY . .

EXPOSE 8000

CMD ["cargo", "run"]
