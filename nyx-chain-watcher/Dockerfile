FROM rust:latest AS builder

COPY ./ /usr/src/nym
WORKDIR /usr/src/nym/nyx-chain-watcher

RUN cargo build --release

#-------------------------------------------------------------------
# The following environment variables are required at runtime:
#
# NYX_CHAIN_WATCHER_DATABASE_PATH = /mnt/nyx-chain-watchter.sqlite
# NYX_CHAIN_WATCHER_HISTORY_DATABASE_PATH = /mnt/chain-history.sqlite
# NYX_CHAIN_WATCHER_WATCH_ACCOUNTS = "n1...,n1...,n1..."
#
# And optionally:
#
# NYX_CHAIN_WATCHER_WATCH_CHAIN_MESSAGE_TYPES = "/cosmos.bank.v1beta1.MsgSend,/ibc.applications.transfer.v1.MsgTransfer"
# NYX_CHAIN_WATCHER_CONFIG_ENV_FILE_ARG = /mnt/sandbox.env for sandbox environment
#
# see https://github.com/nymtech/nym/blob/develop/nyx-chain-watcher/src/cli/commands/run/args.rs for details
# and https://github.com/nymtech/nym/blob/develop/nyx-chain-watcher/src/env.rs for env vars
#-------------------------------------------------------------------

FROM ubuntu:24.04

RUN apt update && apt install -yy curl ca-certificates

WORKDIR /nym

COPY --from=builder /usr/src/nym/target/release/nyx-chain-watcher ./
ENTRYPOINT [ "/nym/nyx-chain-watcher", "run" ]

