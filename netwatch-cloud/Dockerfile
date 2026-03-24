FROM rust:1-bookworm AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY netwatch-cloud/ netwatch-cloud/
RUN mkdir -p netwatch-agent/src && echo "fn main() {}" > netwatch-agent/src/main.rs
COPY netwatch-agent/Cargo.toml netwatch-agent/Cargo.toml
RUN cargo build --release --package netwatch-cloud

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/netwatch-cloud /usr/local/bin/netwatch-cloud
CMD ["netwatch-cloud"]
