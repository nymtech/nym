# this will only work with VPN, otherwise remove the harbor part
FROM harbor.nymte.ch/dockerhub/rust:latest AS builder

RUN apt update && apt install -yy libdbus-1-dev pkg-config libclang-dev

COPY ./ /usr/src/nym
WORKDIR /usr/src/nym

RUN cargo build --bin nym-network-monitor-agent --release

FROM harbor.nymte.ch/dockerhub/ubuntu:24.04

RUN apt-get update && apt-get install -y ca-certificates

WORKDIR /nym

COPY --from=builder /usr/src/nym/target/release/nym-network-monitor-agent ./
COPY --from=builder /usr/src/nym/nym-network-monitor-v3/nym-network-monitor-agent/entrypoint.sh ./
RUN chmod +x /nym/entrypoint.sh

ENV SLEEP_TIME=5
ENTRYPOINT [ "/nym/entrypoint.sh" ]
