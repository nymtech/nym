#!/bin/bash
set -e

# SDK and its dependencies
PACKAGES=(
  "nym-api"
  "nym-bandwidth-controller"
  "nym-bin-common"
  "nym-client-core"
  "nym-client-core-config-types"
  "nym-client-core-gateways-storage"
  "nym-client-core-surb-storage"
  "nym-compact-ecash"
  "nym-config"
  "nym-contracts-common"
  "nym-coconut-dkg-common"
  "nym-credential-storage"
  "nym-credential-utils"
  "nym-credentials"
  "nym-credentials-interface"
  "nym-crypto"
  "nym-ecash-contract-common"
  "nym-ecash-signer-check-types"
  "nym-ecash-time"
  "nym-exit-policy"
  "nym-gateway-client"
  "nym-gateway-requests"
  "nym-group-contract-common"
  "nym-http-api-client"
  "nym-http-api-client-macro"
  "nym-http-api-common"
  "nym-id"
  "nym-metrics"
  "nym-mixnet-client"
  "nym-mixnet-contract-common"
  "nym-multisig-contract-common"
  "nym-network-defaults"
  "nym-noise"
  "nym-noise-keys"
  "nym-nonexhaustive-delayqueue"
  "nym-node-requests"
  "nym-ordered-buffer"
  "nym-outfox"
  "nym-pemstore"
  "nym-performance-contract-common"
  "nym-serde-helpers"
  "nym-service-providers-common"
  "nym-socks5-client-core"
  "nym-socks5-proxy-helpers"
  "nym-socks5-requests"
  "nym-sphinx"
  "nym-sphinx-acknowledgements"
  "nym-sphinx-addressing"
  "nym-sphinx-anonymous-replies"
  "nym-sphinx-chunking"
  "nym-sphinx-cover"
  "nym-sphinx-forwarding"
  "nym-sphinx-framing"
  "nym-sphinx-params"
  "nym-sphinx-routing"
  "nym-sphinx-types"
  "nym-statistics-common"
  "nym-task"
  "nym-ticketbooks-merkle"
  "nym-topology"
  "nym-upgrade-mode-check"
  "nym-validator-client"
  "nym-vesting-contract-common"
  "nym-wireguard-types"
  "nym-sqlx-pool-guard"
  "nym-sdk"
)

PACKAGE_FLAGS=""
for pkg in "${PACKAGES[@]}"; do
  PACKAGE_FLAGS="$PACKAGE_FLAGS -p $pkg"
done

cargo release \
  $PACKAGE_FLAGS \
  --prev-tag-name "" \
  --no-push \
  --no-tag \
  --no-publish \
  --allow-branch '*' \
  -v \
  "$@"
