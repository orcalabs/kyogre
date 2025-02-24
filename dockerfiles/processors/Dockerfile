FROM lukemathwalker/cargo-chef:latest-rust-1.85.0-bookworm AS chef

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
    libssl-dev

RUN --mount=type=secret,id=cloudsmith-api-key,required=true \
    export CARGO_REGISTRIES_ORCALABS_ORCASTRATOR_TOKEN=$(cat /run/secrets/cloudsmith-api-key) && \
    export CARGO_REGISTRIES_ORCALABS_ORCASTRATOR_CREDENTIAL_PROVIDER=cargo:token && \
    cd src && \
    cargo chef cook --release --recipe-path recipe.json

COPY src src
COPY scripts scripts

RUN cd src && \
    cargo build --release --bin processors

FROM gcr.io/distroless/cc

COPY --from=builder /app/src/target/release/processors .

COPY src/processors/config config

ENTRYPOINT ["./processors"]
