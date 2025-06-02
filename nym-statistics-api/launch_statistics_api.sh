#!/bin/bash

set -e

user_rust_log_preference=$RUST_LOG


function run_bare() {
    # export necessary env vars
    set -a
    source .env.dev
    set +a
    export RUST_LOG=${user_rust_log_preference:-debug}
    echo "RUST_LOG=${RUST_LOG}"

    cargo run --package nym-statistics-api
}

run_bare
