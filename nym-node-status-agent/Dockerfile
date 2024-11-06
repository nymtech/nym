FROM rust:latest AS builder

ARG GIT_REF=main

RUN apt update && apt install -yy libdbus-1-dev pkg-config libclang-dev

# Install go
RUN wget https://go.dev/dl/go1.22.5.linux-amd64.tar.gz -O go.tar.gz
RUN tar -xzvf go.tar.gz -C /usr/local

RUN git clone https://github.com/nymtech/nym-vpn-client /usr/src/nym-vpn-client
RUN cd /usr/src/nym-vpn-client && git checkout $GIT_REF
ENV PATH=/go/bin:/usr/local/go/bin:$PATH
WORKDIR /usr/src/nym-vpn-client/nym-vpn-core
RUN cargo build --release --package nym-gateway-probe

COPY ./ /usr/src/nym
WORKDIR /usr/src/nym/nym-node-status-agent
RUN cargo build --release

FROM ubuntu:24.04

RUN apt-get update && apt-get install -y ca-certificates

WORKDIR /nym

COPY --from=builder /usr/src/nym/target/release/nym-node-status-agent ./
COPY --from=builder /usr/src/nym-vpn-client/nym-vpn-core/target/release/nym-gateway-probe ./

ENV NODE_STATUS_AGENT_PROBE_PATH=/nym/nym-gateway-probe
ENTRYPOINT [ "/nym/nym-node-status-agent" ]
