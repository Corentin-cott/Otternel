FROM rust:1.87-slim AS builder
WORKDIR /build

# Cache dependency separately to speed up builds
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release --locked \
    && rm -f target/release/Otternel target/release/deps/Otternel-* \
    && rm -rf src

# Compilation of the actual code
COPY src ./src
RUN cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /build/target/release/Otternel ./otternel
COPY triggers.toml ./

CMD ["./otternel"]
