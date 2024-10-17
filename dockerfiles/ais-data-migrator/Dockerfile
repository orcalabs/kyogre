FROM lukemathwalker/cargo-chef:latest-rust-1.81.0-bookworm AS chef

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
    cargo build --release --bin ais-data-migrator

FROM gcr.io/distroless/cc

COPY --from=builder /app/src/target/release/ais-data-migrator .

COPY src/ais-data-migrator/config config

ENTRYPOINT ["./ais-data-migrator"]
