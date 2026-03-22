FROM rust:1.85-slim AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release \
    && rm -rf src

COPY src/ src/
RUN touch src/main.rs && cargo build --release && strip target/release/docker-watchdog

FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/target/release/docker-watchdog /usr/local/bin/docker-watchdog
ENTRYPOINT ["docker-watchdog"]
