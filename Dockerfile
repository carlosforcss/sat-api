FROM rust:bookworm

# Install Chromium and curl
RUN apt-get update && apt-get install -y --no-install-recommends \
    chromium ca-certificates fonts-liberation libnss3 libatk-bridge2.0-0 \
    libgtk-3-0 libxss1 libasound2 curl \
    && rm -rf /var/lib/apt/lists/*

ENV CHROME_PATH=/usr/bin/chromium

# Install cargo-watch via prebuilt binary — avoids compiling it from source.
RUN curl -L --proto '=https' --tlsv1.2 -sSf \
    https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
    && cargo binstall --no-confirm cargo-watch

# Compiled artifacts live outside /app so the source volume mount cannot shadow them.
# Backed by a named Docker volume — persists across container restarts.
ENV CARGO_TARGET_DIR=/cargo-target

WORKDIR /app

EXPOSE 8000

CMD ["cargo", "watch", "-x", "run"]
