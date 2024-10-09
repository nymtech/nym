#!/bin/bash

set -e

export RUST_LOG=${RUST_LOG:-debug}

export NYM_API_CLIENT_TIMEOUT=60;
export EXPLORER_CLIENT_TIMEOUT=60;

cargo run --package nym-node-status-api --release -- --config-env-file ../envs/mainnet.env
