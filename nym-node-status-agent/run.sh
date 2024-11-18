#!/bin/bash

set -eu
environment="qa"

probe_git_ref="0dd5dacdda92b1ddd51cd30a3399515e45613371"

crate_root=$(dirname $(realpath "$0"))
monorepo_root=$(dirname "${crate_root}")
echo "Expecting nym-vpn-client repo at a sibling level of nym monorepo dir"
gateway_probe_src=$(dirname "${monorepo_root}")/nym-vpn-client/nym-vpn-core
echo "gateway_probe_src=$gateway_probe_src"
echo "crate_root=$crate_root"

set -a
source "${monorepo_root}/envs/${environment}.env"
set +a

export RUST_LOG="debug"
export NODE_STATUS_AGENT_SERVER_ADDRESS="http://127.0.0.1"
export NODE_STATUS_AGENT_SERVER_PORT="8000"
export NODE_STATUS_AGENT_PROBE_PATH="$crate_root/nym-gateway-probe"

# each key is required for a separate agent when running in parallel: their
# public counterparts need to be registered with NS API
private_keys=("BjyC9SsHAZUzPRkQR4sPTvVrp4GgaquTh5YfSJksvvWT"
    "4RqSKydrEuyGF8Xtwyauvja62SAjqxFPYQzW2neZdkQD"
    "CfudaSaaLTkgR8rkBijUnYocdFciWTbKqkSNYevepnbn"
    "Dd3fDyPUg4edTpiCAkSweE17NdWJ7gAchbtqAeSj3MBc"
    "HAtfcfnpw5XdpcVzAH6Qsxp6Zf75oe2W54HjTD8ngVbU"
    "8aqWP8wZyhX5W8gfrvyh1SmS6CEgfLAR95DBhWXRUpTm"
    "234U1PMkWpAsn7hD98g1D8hfRFkJJS91T2sJBQDyXmqx"
    "5qUUFu83fgqpACsr3dC6iwGJxhTqN4JJDTecT2QSqwEe"
    "4Pp7Cd9G3aMku9biFcxRMEja8RBMbBRGZuDAZt1yTS7H"
    "4U136QykP8G831EZSDNLLvgWCGYA8naYtT8BQ9kLeL5B"
)

workers=${1:-1}
if ((workers > ${#private_keys[@]})); then
    echo "Error: ${workers} is larger than the number of private keys available (${#private_keys[@]})."
    exit 1
fi
echo "Running $workers workers in parallel"

# build & copy over GW probe
function copy_gw_probe() {
    pushd $gateway_probe_src
    git fetch -a
    git checkout $probe_git_ref
    cargo build --release --package nym-gateway-probe
    cp target/release/nym-gateway-probe "$crate_root"
    $crate_root/nym-gateway-probe --version
    popd
}

function build_agent() {
    cargo build --package nym-node-status-agent --release
}

function swarm() {
    local workers=$1

    for ((i = 1; i <= workers; i++)); do
        export NODE_STATUS_AGENT_AUTH_KEY=${private_keys[i]}
        ../target/release/nym-node-status-agent run-probe &
    done

    wait

    echo "All agents completed"
}

copy_gw_probe
build_agent

swarm $workers
