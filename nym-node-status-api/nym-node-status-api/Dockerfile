# this will only work with VPN, otherwise remove the harbor part
FROM harbor.nymte.ch/dockerhub/rust:latest AS builder

RUN apt update && apt install -yy libdbus-1-dev pkg-config libclang-dev

# Install go (required for nym-gateway-probe dependency)
RUN wget https://go.dev/dl/go1.22.5.linux-amd64.tar.gz -O go.tar.gz
RUN tar -xzvf go.tar.gz -C /usr/local
ENV PATH=/go/bin:/usr/local/go/bin:$PATH

COPY ./ /usr/src/nym
WORKDIR /usr/src/nym/nym-node-status-api/nym-node-status-api/

RUN cargo build --release


#-------------------------------------------------------------------
# The following environment variables are required at runtime:
#
# EXPLORER_API
# NYXD
# NYM_API
# DATABASE_URL
#
# And optionally:
#
# NYM_NODE_STATUS_API_NYM_HTTP_CACHE_TTL
# NYM_NODE_STATUS_API_HTTP_PORT
# NYM_API_CLIENT_TIMEOUT
# EXPLORER_CLIENT_TIMEOUT
# NODE_STATUS_API_MONITOR_REFRESH_INTERVAL
# NODE_STATUS_API_TESTRUN_REFRESH_INTERVAL
#
# see https://github.com/nymtech/nym/blob/develop/nym-node-status-api/src/cli.rs for details
#-------------------------------------------------------------------

FROM harbor.nymte.ch/dockerhub/ubuntu:24.04

RUN apt-get update && apt-get install -y ca-certificates

WORKDIR /nym

COPY --from=builder /usr/src/nym/target/release/nym-node-status-api ./
ENTRYPOINT [ "/nym/nym-node-status-api" ]
