FROM rust:1.49.0-slim-buster as builder

RUN apt update && apt install --no-install-recommends -y pkg-config build-essential libssl-dev curl jq

COPY . /tmp/src
WORKDIR /tmp/src
RUN rustup update && cargo build --release

FROM debian:buster-slim

RUN apt update \
  && apt install -y --no-install-recommends netbase libssl1.1 \
  && apt-get clean \
  && rm -rf /var/lib/apt/lists/*

COPY --from=builder /tmp/src/target/release/nym-socks5-client     /usr/local/bin/
COPY --from=builder /tmp/src/target/release/nym-client            /usr/local/bin/
COPY --from=builder /tmp/src/target/release/nym-gateway           /usr/local/bin/
COPY --from=builder /tmp/src/target/release/nym-mixnode           /usr/local/bin/
COPY --from=builder /tmp/src/target/release/nym-network-monitor   /usr/local/bin/
COPY --from=builder /tmp/src/target/release/nym-network-requester /usr/local/bin/

