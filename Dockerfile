FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev musl gcc

WORKDIR /app
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl && \
    strip target/x86_64-unknown-linux-musl/release/sys-wall
