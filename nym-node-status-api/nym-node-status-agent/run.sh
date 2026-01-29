#!/bin/bash

# used primarily for local testing

set -eu
export ENVIRONMENT=${ENVIRONMENT:-"mainnet"}

crate_root=$(dirname $(realpath "$0"))
echo crate_root=${crate_root}
monorepo_root=$(realpath "${crate_root}/../..")
echo monorepo_root=${monorepo_root}

gateway_probe_src="${monorepo_root}/nym-gateway-probe"
echo "gateway_probe_src=$gateway_probe_src"

set -a
source "${monorepo_root}/envs/${ENVIRONMENT}.env"
set +a

if [ -z "$NYM_NODE_MNEMONICS" ]; then
  echo "NYM_NODE_MNEMONICS is required to run an agent"
  exit 1
fi

export RUST_LOG="info"
NODE_STATUS_AGENT_SERVER_ADDRESS="http://127.0.0.1"
NODE_STATUS_AGENT_SERVER_PORT="8000"
SERVER="${NODE_STATUS_AGENT_SERVER_ADDRESS}|${NODE_STATUS_AGENT_SERVER_PORT}"
# hardcoded key used only for LOCAL TESTING
export NODE_STATUS_AGENT_AUTH_KEY=${NODE_STATUS_AGENT_AUTH_KEY_STAGING:-"BjyC9SsHAZUzPRkQR4sPTvVrp4GgaquTh5YfSJksvvWT"}
export NODE_STATUS_AGENT_PROBE_PATH="$crate_root/nym-gateway-probe"
export NODE_STATUS_AGENT_PROBE_EXTRA_ARGS="netstack-download-timeout-sec=30,netstack-num-ping=2,netstack-send-timeout-sec=1,netstack-recv-timeout-sec=1"

workers=${1:-1}
echo "Running $workers workers in parallel"

# build & copy over GW probe
function copy_gw_probe() {
    pushd $gateway_probe_src

    cargo build --release --package nym-gateway-probe
    cp "${monorepo_root}/target/release/nym-gateway-probe" "$crate_root"
    $crate_root/nym-gateway-probe --version

    popd
}

function build_agent() {
    cargo build --package nym-node-status-agent --release
}

function swarm() {
    local workers=$1

    for ((i = 1; i <= workers; i++)); do
        ${monorepo_root}/target/release/nym-node-status-agent run-probe --server ${SERVER} &
    done

    wait

    echo "All agents completed"
}

copy_gw_probe
build_agent

swarm $workers
