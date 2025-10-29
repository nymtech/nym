# this will only work with VPN, otherwise remove the harbor part
FROM harbor.nymte.ch/dockerhub/rust:latest AS builder

RUN apt update && apt install -yy libdbus-1-dev pkg-config libclang-dev

# Install go
RUN wget https://go.dev/dl/go1.22.5.linux-amd64.tar.gz -O go.tar.gz
RUN tar -xzvf go.tar.gz -C /usr/local
ENV PATH=/go/bin:/usr/local/go/bin:$PATH

COPY ./ /usr/src/nym
WORKDIR /usr/src/nym
RUN cargo build --release --package nym-gateway-probe

WORKDIR /usr/src/nym/nym-node-status-api/nym-node-status-agent
RUN cargo build --release

#-------------------------------------------------------------------
# The following environment variables are required at runtime:
#
# NODE_STATUS_AGENT_SERVER_ADDRESS
# NODE_STATUS_AGENT_SERVER_PORT
#
# see https://github.com/nymtech/nym/blob/develop/nym-node-status-agent/src/cli.rs for details
#-------------------------------------------------------------------

FROM harbor.nymte.ch/dockerhub/ubuntu:24.04

RUN apt-get update && apt-get install -y ca-certificates

WORKDIR /nym

COPY --from=builder /usr/src/nym/target/release/nym-node-status-agent ./
COPY --from=builder /usr/src/nym/target/release/nym-gateway-probe ./
COPY --from=builder /usr/src/nym/nym-node-status-api/nym-node-status-agent/entrypoint.sh ./
RUN chmod +x /nym/entrypoint.sh

ENV SLEEP_TIME=5
ENV NODE_STATUS_AGENT_PROBE_PATH=/nym/nym-gateway-probe
ENTRYPOINT [ "/nym/entrypoint.sh" ]
