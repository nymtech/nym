#!/bin/bash

set -e

user_rust_log_preference=$RUST_LOG


function run_bare() {
    # export necessary env vars
    set -a
    source .env
    set +a
    export RUST_LOG=${user_rust_log_preference:-debug}
    echo "RUST_LOG=${RUST_LOG}"

    cargo run --package nym-statistics-api
}


# Requires pg_up.sh, or a running postres instance, with the correct parameters in an .env file
run_bare
