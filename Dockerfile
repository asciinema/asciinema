ARG RUST_VERSION=1.70.0
FROM rust:${RUST_VERSION}-bookworm as builder
WORKDIR /app

RUN --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    <<EOF
set -e
cargo build --locked --release
cp ./target/release/asciinema /usr/local/bin/
EOF

FROM debian:bookworm-slim as run
COPY --from=builder /usr/local/bin/asciinema /usr/local/bin
ENTRYPOINT ["/usr/local/bin/asciinema"]
