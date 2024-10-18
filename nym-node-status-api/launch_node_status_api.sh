#!/bin/bash

set -e

export CONFIG_ENV_FILE="../envs/mainnet.env"
export NYM_API_CLIENT_TIMEOUT=60
export EXPLORER_CLIENT_TIMEOUT=60

cargo run --package nym-node-status-api --release -- --config-env-file ${CONFIG_ENV_FILE}
