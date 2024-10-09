#!/bin/bash

set -e

export RUST_LOG=${RUST_LOG:-debug}

export NYM_API_CLIENT_TIMEOUT=60
export EXPLORER_CLIENT_TIMEOUT=60
crate_root=$(dirname $(realpath "$0"))
export NYM_GATEWAY_PROBE="$crate_root/nym-gateway-probe"

gateway_probe_src=$(dirname $(dirname "$crate_root"))/nym-vpn-client/nym-vpn-core
echo "$gateway_probe_src"

pushd $gateway_probe_src
cargo build --release --package nym-gateway-probe
cp target/release/nym-gateway-probe "$crate_root"
$NYM_GATEWAY_PROBE --version
popd

cargo run --package nym-node-status-api --release -- --config-env-file ../envs/mainnet.env
