#!/bin/bash

set -e
clear

function usage() {
    echo "Usage: $0 [-c]"
    echo "  -c if present, recreate DB"
    exit 0
}

function init_db() {
    rm -rf data/*
    cargo sqlx database drop -y

    cargo sqlx database create
    cargo sqlx migrate run

    echo "Fresh database ready!"
}

export RUST_LOG=trace
export DATABASE_URL=sqlite://data/nym-node-status-api.sqlite

clear_db=false

while getopts "c" opt; do
    case ${opt} in
    c)
        clear_db=true
        ;;
    \?)
        usage
        ;;
    esac
done

if [ "$clear_db" = true ]; then
    init_db
fi


cd ..
cargo run --package nym-node-status-api -- --config-env-file envs/canary.env
