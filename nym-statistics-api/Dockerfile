# this will only work with VPN, otherwise remove the harbor part
FROM harbor.nymte.ch/dockerhub/rust:latest AS builder

COPY ./ /usr/src/nym
WORKDIR /usr/src/nym/nym-statistics-api

RUN cargo build --release


#-------------------------------------------------------------------
# The following environment variables are required at runtime:
#
# DATABASE_URL
# POSTGRES_USER
# POSTGRES_PASSWORD
# PG_SSL_CERT
#
# And optionally:
#
# NYM_API_URL
# NYM_STATISTICS_API_HTTP_PORT
# PGPORT
#
# see https://github.com/nymtech/nym/blob/develop/nym-statistics-api/src/cli/mod.rs for details
#-------------------------------------------------------------------

FROM harbor.nymte.ch/dockerhub/ubuntu:24.04

RUN apt-get update && apt-get install -y ca-certificates

WORKDIR /nym

COPY --from=builder /usr/src/nym/target/release/nym-statistics-api ./
ENTRYPOINT [ "/nym/nym-statistics-api" ]
