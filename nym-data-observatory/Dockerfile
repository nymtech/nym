# this will only work with VPN, otherwise remove the harbor part
FROM harbor.nymte.ch/dockerhub/rust:latest AS builder

COPY ./ /usr/src/nym
WORKDIR /usr/src/nym/nym-data-observatory

RUN cargo build --release

#-------------------------------------------------------------------
# The following environment variables are required at runtime:
#
# NYM_DATA_OBSERVATORY_DB_URL=postgres://nym_data_observatory:data-data-data@localhost/nym_data_observatory_data
#
# And optionally:
#
# NYM_DATA_OBSERVATORY_WEBHOOK_URL="https://webhook.site"
# NYM_DATA_OBSERVATORY_WEBHOOK_AUTH=1234
# NYX_CHAIN_WATCHER_CONFIG_ENV_FILE_ARG = /mnt/sandbox.env for sandbox environment
#
# see https://github.com/nymtech/nym/blob/develop/nym-data-observatory/src/cli/commands/run/args.rs for details
# and https://github.com/nymtech/nym/blob/develop/nym-data-observatory/src/env.rs for env vars
#-------------------------------------------------------------------

FROM harbor.nymte.ch/dockerhub/ubuntu:24.04

RUN apt update && apt install -yy curl ca-certificates

WORKDIR /nym

COPY --from=builder /usr/src/nym/target/release/nym-data-observatory ./
ENTRYPOINT [ "/nym/nym-data-observatory", "run" ]

