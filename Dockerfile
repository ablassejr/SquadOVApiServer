FROM debian:buster-20200908-slim AS builder
ARG GCP_PROJECT
RUN apt-get update && apt-get install -y --no-install-recommends curl \
    ca-certificates \
    build-essential \
    openssl \ 
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*
RUN mkdir -p /squadov/config
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

COPY src /squadov/src
COPY deps /squadov/deps
COPY config/$GCP_PROJECT.toml /squadov/config/config.toml
COPY Cargo.toml /squadov/
COPY sqlx-data.json /squadov/
COPY devops/gcp /squadov/gcp

WORKDIR /squadov
RUN cargo build --release

FROM debian:buster-20200908-slim
RUN mkdir -p /squadov/config
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /squadov
COPY --from=builder /squadov/target/release/squadov_api_server .
COPY --from=builder /squadov/config/config.toml ./config 
COPY --from=builder /squadov/gcp ./gcp
ENTRYPOINT ["./squadov_api_server", "--config", "./config/config.toml"]