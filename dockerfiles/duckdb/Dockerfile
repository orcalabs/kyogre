FROM lukemathwalker/cargo-chef:latest-rust-1.75.0-bullseye AS chef

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
    libssl1.1 \
    libssl-dev \
    protobuf-compiler \
    pkg-config \
    python3.9 \
    python3.9-dev && \
    git config --global credential.helper store

RUN --mount=type=secret,id=cloudsmith-api-key,required=true \
    export CARGO_REGISTRIES_ORCALABS_ORCASTRATOR_TOKEN=$(cat /run/secrets/cloudsmith-api-key) && \
    export CARGO_REGISTRIES_ORCALABS_ORCASTRATOR_CREDENTIAL_PROVIDER=cargo:token && \
    cd src && \
    cargo chef cook --release --recipe-path recipe.json

COPY src src
RUN cd src && \
    cargo build --release --bin duckdb-rs

FROM gcr.io/distroless/cc

COPY src/duckdb-rs/config config

COPY --from=builder /app/src/target/release/duckdb-rs .

ENTRYPOINT ["./duckdb-rs"]
