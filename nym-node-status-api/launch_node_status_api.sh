#!/bin/bash

set -e

export CONFIG_ENV_FILE="../envs/mainnet.env"
export NYM_API_CLIENT_TIMEOUT=60
export EXPLORER_CLIENT_TIMEOUT=60
#export NYXD=https://rpc.nymtech.net
#export NYM_API=https://validator.nymtech.net/api/
#export EXPLORER_API=https://explorer.nymtech.net/api/
#export NETWORK_NAME=mainnet

#cargo run --package nym-node-status-api --release -- --connection-url "sqlite://node-status-api.sqlite?mode=rwc"

cd ..
docker build -t node-status-api -f nym-node-status-api/Dockerfile .
docker run --env-file envs/mainnet.env -e NYM_NODE_STATUS_API_CONNECTION_URL="sqlite://node-status-api.sqlite?mode=rwc" node-status-api
