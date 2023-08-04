FROM lukemathwalker/cargo-chef:latest-rust-1.71.0-bullseye AS chef

WORKDIR /app

FROM chef AS planner

COPY src .
COPY .cargo/release.toml .cargo/config.toml

RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

RUN apt update -y && \
    apt install -y \
    git \
    libssl1.1 \
    libssl-dev \
    protobuf-compiler \
    pkg-config && \
    git config --global credential.helper store

RUN --mount=type=secret,id=git-credentials,target=/root/.git-credentials,required=true \
    cargo chef cook --release --recipe-path recipe.json

COPY src .
RUN --mount=type=secret,id=git-credentials,target=~/.git-credentials,required=true \
    cargo build --release --bin database-migrator

FROM gcr.io/distroless/cc

COPY --from=builder /app/target/release/database-migrator .

COPY src/database-migrator/config config

ENTRYPOINT ["./database-migrator"]
