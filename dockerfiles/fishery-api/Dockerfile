FROM lukemathwalker/cargo-chef:latest-rust-1.73.0-bullseye AS chef

WORKDIR /app

FROM chef AS planner

COPY src .
COPY .cargo .cargo

RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json


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
    cargo chef cook --release --recipe-path recipe.json

COPY src .
RUN --mount=type=secret,id=git-credentials,target=~/.git-credentials,required=true \
    cargo build --release --bin web-api

FROM gcr.io/distroless/cc

COPY src/web-api/config config

COPY --from=builder /app/target/release/web-api .

ENTRYPOINT ["./web-api"]
