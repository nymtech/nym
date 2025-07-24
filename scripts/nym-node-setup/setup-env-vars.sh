#!/bin/bash

# Prompt user to enter env vars & save them to env.sh
read -p "Enter latest binary URL or name: " LATEST_BINARY && \
read -p "Enter hostname: " HOSTNAME && \
read -p "Enter location (country code or name): " LOCATION && \
read -p "Enter email: " EMAIL && \
MONIKER=${HOSTNAME#nym-exit.} \
echo -e "export LATEST_BINARY=\"$LATEST_BINARY\"\nexport HOSTNAME=\"$HOSTNAME\"\nexport LOCATION=\"$LOCATION\"\nexport EMAIL=\"$EMAIL\"" > env.sh && \
echo "Variables saved to env.sh — run 'source ./env.sh' to load them."
source ./env.sh
