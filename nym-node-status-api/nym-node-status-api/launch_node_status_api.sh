#!/bin/bash

set -e

user_rust_log_preference=$RUST_LOG
export ENVIRONMENT=${ENVIRONMENT:-"mainnet"}
export NYM_API_CLIENT_TIMEOUT=60
export NODE_STATUS_API_TESTRUN_REFRESH_INTERVAL=120

# public counterpart of the agent's private key.
# For TESTING only. NOT used in any other environment
export NODE_STATUS_API_AGENT_KEY_LIST="H4z8kx5Kkf5JMQHhxaW1MwYndjKCDHC7HsVhHTFfBZ4J"

script_dir=$(dirname $(realpath "$0"))
monorepo_root=$(realpath "${script_dir}/../..")

function run_bare() {
    # export necessary env vars
    set -a
    source "${monorepo_root}/envs/${ENVIRONMENT}.env"
    set +a
    export RUST_LOG=${user_rust_log_preference:-debug}
    echo "RUST_LOG=${RUST_LOG}"

    # --conection-url is provided in build.rs
    cargo run --package nym-node-status-api --features pg --no-default-features
}

function run_docker() {
    cargo build --package nym-node-status-api --release
    cp ../target/release/nym-node-status-api .

    cd ..
    docker build -t node-status-api -f nym-node-status-api/Dockerfile.dev .
    docker run --env-file envs/${ENVIRONMENT} \
        -e EXPLORER_CLIENT_TIMEOUT=$EXPLORER_CLIENT_TIMEOUT \
        -e NYM_API_CLIENT_TIMEOUT=$NYM_API_CLIENT_TIMEOUT \
        -e DATABASE_URL="sqlite://node-status-api.sqlite?mode=rwc" \
        -e RUST_LOG=${RUST_LOG} node-status-api

}

run_bare

# run_docker
