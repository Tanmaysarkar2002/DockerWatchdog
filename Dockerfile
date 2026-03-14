FROM rust:1.85-slim AS builder
WORKDIR /app

RUN echo '[package]\nname = "docker-watchdog"\nversion = "0.1.0"\nedition = "2024"\n\n[dependencies]\nbollard = "0.20.1"\nfutures-util = "0.3.32"\ntokio = { version = "1.50.0", features = ["full"] }\ntracing = "0.1.44"\ntracing-subscriber = { version = "0.3.23", features = ["env-filter"] }' > Cargo.toml \
    && mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release \
    && rm -rf src

COPY Cargo.toml Cargo.lock ./
COPY src/ src/
RUN cargo build --release && strip target/release/docker-watchdog

FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/target/release/docker-watchdog /usr/local/bin/docker-watchdog
ENTRYPOINT ["docker-watchdog"]
