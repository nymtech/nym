#!/usr/bin/env bash
#
# Single-command boot of an unbonded, standalone nym-node entry gateway with the
# free tier enabled. `nym-node run` initialises the config on first boot and runs
# in one shot - no `--init-only` / `--deny-init` split, since that only existed to
# read identity keys for the bonding flow, which we drop.
#
# `--standalone` makes the node perform zero nym-api / nyx-chain / nym-node calls,
# so the container is hermetic (no mainnet egress). `entry-gateway` mode avoids the
# exit-policy upstream fetch that exit mode performs.
#
# Every knob is an env var; override with `docker run -e NAME=value`.
set -euo pipefail

# Required: the free-tier JWT signer public key (bs58). For a plain boot smoke test
# any valid ed25519 pubkey works (no token is verified until a client registers);
# the Milestone B driver injects the key whose private half it signs tokens with.
: "${NYMNODE_FREE_TIER_SIGNER_PUBKEY:?set NYMNODE_FREE_TIER_SIGNER_PUBKEY to the free-tier signer public key (bs58)}"

export NYMNODE_ID="${NYMNODE_ID:-free-tier-localtest}"
export NYMNODE_MODE="${NYMNODE_MODE:-entry-gateway}"
export NYMNODE_PUBLIC_IPS="${NYMNODE_PUBLIC_IPS:-127.0.0.1}"
export NYMNODE_ACCEPT_OPERATOR_TERMS="${NYMNODE_ACCEPT_OPERATOR_TERMS:-true}"
export NYMNODE_LOCAL="${NYMNODE_LOCAL:-true}"
export NYMNODE_STANDALONE="${NYMNODE_STANDALONE:-true}"
export NYMNODE_WG_ENABLED="${NYMNODE_WG_ENABLED:-true}"
export NYMNODE_FREE_TIER_ENABLED="${NYMNODE_FREE_TIER_ENABLED:-true}"

echo "[entrypoint] booting standalone free-tier gateway"
echo "[entrypoint]   id=${NYMNODE_ID} mode=${NYMNODE_MODE} public-ips=${NYMNODE_PUBLIC_IPS}"
echo "[entrypoint]   standalone=${NYMNODE_STANDALONE} local=${NYMNODE_LOCAL} wg=${NYMNODE_WG_ENABLED}"
echo "[entrypoint]   free-tier signer=${NYMNODE_FREE_TIER_SIGNER_PUBKEY}"

# NAT/forwarding so WG client traffic egresses to the internet, via the real operator
# script. Safe before nymwg exists (rules reference it by name, matched at packet time);
# the native-nft walled garden later filters ahead of these iptables rules. Tolerant:
# a failure warns but still boots so the node is inspectable.
if [ "${NYMNODE_HARNESS_SETUP_NAT:-true}" = "true" ]; then
    echo "[entrypoint] applying WG NAT/forwarding rules (network-tunnel-manager.sh apply_iptables_rules_wg)"
    if ! ./network-tunnel-manager.sh apply_iptables_rules_wg; then
        echo "[entrypoint] WARNING: WG NAT setup failed - internet egress through the tunnel will not work"
    fi
fi

exec ./nym-node run
