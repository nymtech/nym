#!/bin/bash

set -eu

environment="qa"

source ../envs/${environment}.env

probe_git_ref="0dd5dacdda92b1ddd51cd30a3399515e45613371"

export RUST_LOG="debug"

crate_root=$(dirname $(realpath "$0"))
gateway_probe_src=$(dirname $(dirname "$crate_root"))/nym-vpn-client/nym-vpn-core
echo "gateway_probe_src=$gateway_probe_src"
echo "crate_root=$crate_root"

export NODE_STATUS_AGENT_PROBE_PATH="$crate_root/nym-gateway-probe"

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
    echo "Running $workers in parallel"

    build_agent

    for ((i = 1; i <= $workers; i++)); do
        ../target/release/nym-node-status-agent run-probe &
    done

    wait

    echo "All agents completed"
}

export NODE_STATUS_AGENT_SERVER_ADDRESS="http://127.0.0.1"
export NODE_STATUS_AGENT_SERVER_PORT="8000"

# copy_gw_probe

swarm 8

# cargo run -- run-probe
