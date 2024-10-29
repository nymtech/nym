#!/bin/bash

set -e

export RUST_LOG=${RUST_LOG:-debug}

export NYM_API_CLIENT_TIMEOUT=60
export EXPLORER_CLIENT_TIMEOUT=60

export ENVIRONMENT="qa.env"

function run_bare() {
    # export necessary env vars
    set -a
    source ../envs/$ENVIRONMENT
    set +a
    export RUST_LOG=debug

    # --conection-url is provided in build.rs
    cargo run --package nym-node-status-api
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
