FROM rust:1-alpine AS api-builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM node:24-alpine AS frontend-builder
RUN corepack enable pnpm
WORKDIR /build/frontend
COPY frontend/package.json frontend/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY frontend/ ./
RUN pnpm run build

FROM alpine:latest
COPY --from=api-builder /build/target/release/lacuna /usr/local/bin/lacuna
COPY --from=frontend-builder /build/frontend/dist /opt/lacuna/frontend/dist
ENTRYPOINT [ "lacuna", "--assets=/opt/lacuna/frontend/dist" ]
