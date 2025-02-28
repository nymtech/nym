# this will only work with VPN, otherwise remove the harbor part
FROM harbor.nymte.ch/dockerhub/rust:latest AS builder

COPY ./ /usr/src/nym
WORKDIR /usr/src/nym/nym-validator-rewarder

RUN cargo build --release


#-------------------------------------------------------------------
# The following environment variables are required at runtime:
#
#
# And optionally:
#
#
# see https://github.com/nymtech/nym/blob/develop/nym-validator-rewarder/src/cli/mod.rs for details
#-------------------------------------------------------------------

FROM harbor.nymte.ch/dockerhub/ubuntu:24.04

RUN apt-get update && apt-get install -y ca-certificates

WORKDIR /nym

COPY --from=builder /usr/src/nym/target/release/nym-validator-rewarder ./

COPY ./nym-validator-rewarder/entrypoint.sh /nym/entrypoint.sh
ENTRYPOINT [ "/nym/entrypoint.sh" ]
