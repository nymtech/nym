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
    # https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md
    cargo sqlx database drop -y

    cargo sqlx database create
    cargo sqlx migrate run
    cargo sqlx prepare

    echo "Fresh database ready!"
}

# export DATABASE_URL as absolute path due to this
# https://github.com/launchbadge/sqlx/issues/3099
db_filename="nym-node-status-api.sqlite"
script_abs_path=$(realpath "$0")
package_dir=$(dirname "$script_abs_path")
db_abs_path="$package_dir/data/$db_filename"
dotenv_file="$package_dir/.env"
echo "DATABASE_URL=sqlite://$db_abs_path" >"$dotenv_file"

export RUST_LOG=trace

# export DATABASE_URL from .env file
set -a && source "$dotenv_file" && set +a

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

cargo run --package nym-node-status-api -- --config-env-file ../envs/mainnet.env
