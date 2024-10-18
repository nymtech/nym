#!/bin/bash

set -eu

export RUST_LOG=${RUST_LOG:-debug}

crate_root=$(dirname $(realpath "$0"))
gateway_probe_src=$(dirname $(dirname "$crate_root"))/nym-vpn-client/nym-vpn-core
echo "$gateway_probe_src"
export NYM_GATEWAY_PROBE="$crate_root/nym-gateway-probe"

# build & copy over GW probe
function copy_gw_probe() {
    pushd $gateway_probe_src
    cargo build --release --package nym-gateway-probe
    cp target/release/nym-gateway-probe "$crate_root"
    $NYM_GATEWAY_PROBE --version
    popd
}

export NODE_STATUS_AGENT_SERVER_ADDRESS="http://127.0.0.1"
export NODE_STATUS_AGENT_SERVER_PORT="8000"

copy_gw_probe

cargo run -- run-probe --probe-path "$NYM_GATEWAY_PROBE"
