FROM lukemathwalker/cargo-chef:latest-rust-1.93.1-bookworm AS chef

WORKDIR /app

FROM chef AS planner

COPY src src
COPY .cargo src/.cargo

RUN cd src && \
    cargo chef prepare --recipe-path recipe.json


FROM chef AS builder
COPY --from=planner /app/src/recipe.json src/recipe.json

RUN apt update -y && \
    apt install -y \
    git \
    libssl-dev \
    protobuf-compiler \
    pkg-config \
    python3 \
    python3-dev

RUN --mount=type=secret,id=cloudsmith-api-key,required=true \
    export CARGO_REGISTRIES_ORCALABS_ORCASTRATOR_TOKEN=$(cat /run/secrets/cloudsmith-api-key) && \
    export CARGO_REGISTRIES_ORCALABS_ORCASTRATOR_CREDENTIAL_PROVIDER=cargo:token && \
    cd src && \
    cargo chef cook --release --recipe-path recipe.json

COPY src src
RUN cd src && \
    cargo build --release --bin web-api

FROM debian:bookworm-slim

RUN apt update -y && \
    apt install -y \
    ca-certificates

COPY src/web-api/config config

COPY --from=builder /app/src/target/release/web-api .

ENTRYPOINT ["./web-api"]
