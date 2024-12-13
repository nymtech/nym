FROM rust:latest AS builder

COPY ./ /usr/src/nym
WORKDIR /usr/src/nym/nym-node-status-api

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

FROM ubuntu:24.04

RUN apt-get update && apt-get install -y ca-certificates

WORKDIR /nym

COPY --from=builder /usr/src/nym/target/release/nym-node-status-api ./
ENTRYPOINT [ "/nym/nym-node-status-api" ]
