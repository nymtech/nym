# this will only work with VPN, otherwise remove the harbor part
FROM harbor.nymte.ch/dockerhub/rust:latest AS builder

COPY ./ /usr/src/nym
WORKDIR /usr/src/nym/nym-credential-proxy/nym-credential-proxy

RUN cargo build --release

#-------------------------------------------------------------------
# The following environment variables are required at runtime:
#
# NYM_CREDENTIAL_PROXY_MNEMONIC
# NYM_CREDENTIAL_PROXY_AUTH_TOKEN
#
# WEBHOOK_ZK_NYMS_URL
# WEBHOOK_ZK_NYMS_CLIENT_ID
# WEBHOOK_ZK_NYMS_CLIENT_SECRET
#
# And optionally:
#
# NYM_CREDENTIAL_PROXY_PORT
# NYM_CREDENTIAL_PROXY_BIND_ADDRESS
# NYM_CREDENTIAL_PROXY_PERSISTENT_STORAGE_STORAGE
#
# see https://github.com/nymtech/nym/blob/develop/nym-credential-proxy/nym-credential-proxy/src/cli.rs for details
#-------------------------------------------------------------------

FROM harbor.nymte.ch/dockerhub/ubuntu:24.04

RUN apt update && apt install -yy curl ca-certificates

WORKDIR /nym

COPY --from=builder /usr/src/nym/target/release/nym-credential-proxy ./
ENTRYPOINT [ "/nym/nym-credential-proxy" ]

