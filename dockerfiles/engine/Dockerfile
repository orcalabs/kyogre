FROM lukemathwalker/cargo-chef:latest-rust-1.88.0-bookworm AS chef

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
COPY scripts scripts

RUN cd src && \
    cargo build --release --bin engine

FROM debian:bookworm-slim

RUN apt update -y && \
    apt install -y \
    python3 \
    python3-pip

COPY scripts/python/weather/requirements.txt weather_requirements.txt
COPY scripts/python/ocean_climate/requirements.txt ocean_requirements.txt
COPY scripts/python/fishing_predictor/requirements.txt fishing_predictor.txt
RUN pip install --break-system-packages \
    -r fishing_predictor.txt \
    -r weather_requirements.txt \
    -r ocean_requirements.txt

COPY --from=builder /app/src/target/release/engine .

COPY src/engine/config config

ENTRYPOINT ["./engine"]
