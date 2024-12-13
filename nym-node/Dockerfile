FROM rust:latest AS builder

COPY ./ /usr/src/nym
WORKDIR /usr/src/nym/nym-node

RUN cargo build --release

#-------------------------------------------------------------------
# The following environment variables are required at runtime:
#
# NYMNODE_ACCEPT_OPERATOR_TERMS
# NYMNODE_MODE
#
# And optionally:
#
# NYMNODE_BONDING_INFORMATION_OUTPUT
# NYMNODE_DENY_INIT
# NYMNODE_INIT_ONLY
# NYMNODE_LOCAL
# NYMMONDE_WRITE_CONFIG_CHANGES
# NYMNODE_OUTPUT
#
# Host args
# NYMNODE_PUBLIC_IPS
# NYMNODE_HOSTNAME
# NYMNODE_LOCATION
#
# Http args
# NYMNODE_HTTP_BIND_ADDRESS
# NYMNODE_HTTP_LANDING_ASSETS
# NYMNODE_HTTP_ACCESS_TOKEN
# NYMNODE_HTTP_EXPOSE_SYSTEM_INFO
# NYMNODE_HTTP_EXPOSE_SYSTEM_HARDWARE
# NYMNODE_HTTP_EXPOSE_CRYPTO_HARDWARE
#
# Mixnet args
# NYMNODE_MIXNET_BIND_ADDRESS
# NYMNODE_MIXNET_ANNOUNCE_PORT
# NYMNODE_NYM_APIS
# NYMNODE_NYXD
# UNSAFE_DISABLE_NOISE
#
# Wireguard args
# NYMNODE_WG_ENABLED
# NYMNODE_WG_BIND_ADDRESS
# NYMNODE_WG_IP
# NYMNODE_WG_ANNOUNCED_PORT
# NYMNODE_WG_PRIVATE_NETWORK_PREFIX
#
# Mixnode args
# NYMNODE_VERLOC_BIND_ADDRESS
# NYMNODE_VERLOC_ANNOUNCE_PORT
#
# Entry gateway args
# NYMNODE_ENTRY_BIND_ADDRESS
# NYMNODE_ENTRY_ANNOUNCE_WS_PORT
# NYMNODE_ENTRY_ANNOUNCE_WSS_PORT
# NYMNODE_ENFORCE_ZK_NYMS
# NYMNODE_MNEMONIC
#
# Exit gateway args
# NYMNODE_UPSTREAM_EXIT_POLICY
# NYMNODE_OPEN_PROXY
#
# see https://github.com/nymtech/nym/blob/develop/nym-node/src/env.rs for details
#-------------------------------------------------------------------

FROM ubuntu:24.04

WORKDIR /nym

COPY --from=builder /usr/src/nym/target/release/nym-node ./
ENTRYPOINT [ "/nym/nym-node" ]
