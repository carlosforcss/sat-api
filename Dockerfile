FROM rust:latest

WORKDIR /app

RUN cargo install cargo-watch

EXPOSE 8000

CMD ["cargo", "watch", "-x", "run"]
