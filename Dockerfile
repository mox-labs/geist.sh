# Build context: mox/ directory (parent of geist.sh, slick, x.uma)
#
# Usage from mox/:
#   docker build -f geist.sh/Dockerfile -t geist-edge .
#
# Or via docker-compose:
#   docker compose -f geist.sh/docker-compose.yml up

FROM rust:1.83-slim AS builder
WORKDIR /app

# Copy path dependencies
COPY slick/ slick/
COPY x.uma/rumi/core/ x.uma/rumi/core/

# Copy geist.sh workspace
COPY geist.sh/Cargo.toml geist.sh/Cargo.lock* geist.sh/
COPY geist.sh/geist/ geist.sh/geist/

WORKDIR /app/geist.sh
RUN cargo build --release -p geist-sh

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/geist.sh/target/release/geist-sh /usr/local/bin/
EXPOSE 3000
CMD ["geist-sh"]
