#!/bin/bash

set -eu

cargo build --package nym-node-status-agent --release
cp ../target/release/nym-node-status-agent .

crate_root=$(dirname $(realpath "$0"))
gateway_probe_src=$(dirname $(dirname "$crate_root"))/nym-vpn-client/nym-vpn-core
echo "$gateway_probe_src"

pushd "$gateway_probe_src" || exit
cargo build --release --package nym-gateway-probe
cp target/release/nym-gateway-probe "$crate_root"
popd

./nym-gateway-probe --version

docker build -t agent .

docker run --rm --name agent -it agent
