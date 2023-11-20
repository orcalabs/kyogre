FROM lukemathwalker/cargo-chef:latest-rust-1.74.0-bullseye AS chef

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

RUN --mount=type=secret,id=git-credentials,target=/root/.git-credentials,required=true \
    cd src && \
    cargo chef cook --release --recipe-path recipe.json

COPY src src
COPY scripts scripts

RUN --mount=type=secret,id=git-credentials,target=~/.git-credentials,required=true \
    cd src && \
    cargo build --release --bin engine

FROM debian:bullseye-slim

RUN apt update -y && \
    apt install -y \
    python3.9 \
    python3-pip

COPY scripts/python/weather/requirements.txt weather_requirements.txt
COPY scripts/python/ocean_climate/requirements.txt ocean_requirements.txt
COPY scripts/python/fishing_predictor/requirements.txt fishing_predictor.txt
RUN pip install \
    -r fishing_predictor.txt \
    -r weather_requirements.txt \
    -r ocean_requirements.txt

COPY --from=builder /app/src/target/release/engine .

COPY src/engine/config config
COPY src/kyogre-core/shoreline.json.gz .

ENTRYPOINT ["./engine"]
