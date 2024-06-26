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

RUN --mount=type=secret,id=git-credentials,target=/root/.git-credentials,required=true \
    cd src && \
    cargo chef cook --release --recipe-path recipe.json

COPY src .
RUN --mount=type=secret,id=git-credentials,target=~/.git-credentials,required=true \
    cd src && \
    cargo build --release --bin database-migrator

FROM gcr.io/distroless/cc

COPY --from=builder /app/src/target/release/database-migrator .

COPY src/database-migrator/config config

ENTRYPOINT ["./database-migrator"]
