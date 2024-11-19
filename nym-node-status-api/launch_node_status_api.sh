#!/bin/bash

set -e

export RUST_LOG=${RUST_LOG:-debug}

export NYM_API_CLIENT_TIMEOUT=60
export EXPLORER_CLIENT_TIMEOUT=60
export NODE_STATUS_API_TESTRUN_REFRESH_INTERVAL=120
# public keys corresponding to the agents NS API is expecting to be contacted from
export NODE_STATUS_API_AGENT_KEY_LIST="H4z8kx5Kkf5JMQHhxaW1MwYndjKCDHC7HsVhHTFfBZ4J,
5c2GW61135DEr73DxGrR4DR22BLEujvm1k8GYEjRB9at,
3PSFDH2iSJ61KoDNyJpAiw42xS5smV5iBXWnRGTmk2du,
2AH7pJL5PErbSFhZdu3uH8cKa1h1tyCUfSRUm6E5EBz8,
6wQ9ifPFm2EB73BrwpGSd3Ek7GFA5kiAMQDP2ox6JKZw,
G1tevJBnzaQ6zCUsFsxtGJf45BqCTDgzpEz6Sgxks8EH,
FwjL2nGrtgQQ48fPqAUzUZ8UkQZtMtgehqTqj4PQopvh,
Eujj4GmvwQBgHZaNSyqUbjMFSsnXWPSjEYUPgAsKmx1A,
5ZnfSGxW6EKcFxB8jftb9V3f897VpwpZtf7kCPYzB595,
H9kuRd8BGjEUD8Grh5U9YUPN5ZaQmSYz8U44R72AffKM"

export ENVIRONMENT=${ENVIRONMENT:-"sandbox"}

function run_bare() {
    # export necessary env vars
    set -a
    source ../envs/${ENVIRONMENT}.env
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
