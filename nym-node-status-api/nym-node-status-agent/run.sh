#!/bin/bash

set -eu
export ENVIRONMENT=${ENVIRONMENT:-"sandbox"}

probe_git_ref="nym-vpn-core-v1.0.0-rc.6"

crate_root=$(dirname $(realpath "$0"))
monorepo_root=$(dirname "${crate_root}")
echo "Expecting nym-vpn-client repo at a sibling level of nym monorepo dir"
gateway_probe_src=$(dirname "${monorepo_root}")/nym-vpn-client/nym-vpn-core
echo "gateway_probe_src=$gateway_probe_src"
echo "crate_root=$crate_root"

set -a
source "${monorepo_root}/envs/${ENVIRONMENT}.env"
set +a

export RUST_LOG="info"
export NODE_STATUS_AGENT_SERVER_ADDRESS="http://127.0.0.1"
export NODE_STATUS_AGENT_SERVER_PORT="8000"
export NODE_STATUS_AGENT_PROBE_PATH="$crate_root/nym-gateway-probe"
export NODE_STATUS_AGENT_AUTH_KEY="BjyC9SsHAZUzPRkQR4sPTvVrp4GgaquTh5YfSJksvvWT"

workers=${1:-1}
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
        ../target/release/nym-node-status-agent run-probe &
    done

    wait

    echo "All agents completed"
}

copy_gw_probe
build_agent

swarm $workers
