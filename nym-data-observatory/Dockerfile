FROM rust:latest AS builder

COPY ./ /usr/src/nym
WORKDIR /usr/src/nym/nym-data-observatory

RUN cargo build --release

#-------------------------------------------------------------------
# The following environment variables are required at runtime:
#
# NYM_DATA_OBSERVATORY_CONNECTION_URL
#
# And optionally:
#
# NYM_DATA_OBSERVATORY_HTTP_PORT
#
# see https://github.com/nymtech/nym/blob/develop/nym-data-observatory/src/main.rs for details
#-------------------------------------------------------------------

FROM ubuntu:24.04

RUN apt update && apt install -yy curl ca-certificates

WORKDIR /nym

COPY --from=builder /usr/src/nym/target/release/nym-data-observatory ./
ENTRYPOINT [ "/nym/nym-data-observatory" ]
