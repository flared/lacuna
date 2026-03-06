FROM rust:1-alpine AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM alpine:latest
COPY --from=builder /build/target/release/lacuna /usr/local/bin/lacuna
ENTRYPOINT ["lacuna"]
