#!/bin/bash

# Prompt user to enter env vars & save them to env.sh
read -p "Enter latest binary URL: " LATEST_BINARY && \
read -p "Enter hostname: " HOSTNAME && \
read -p "Enter location (country code or name): " LOCATION && \
read -p "Enter email: " EMAIL && \
read -p "Enter node moniker: " MONIKER && \
read -p "Enter node description: " DESCRIPTION && \

set -e

LATEST_BINARY=$(wget -qO - https://github.com/nymtech/nym/releases/latest \
  | grep -oP 'href="\/nymtech\/nym\/releases\/download\/[^"]+\/nym-node"' \
  | head -n 1 | cut -d'"' -f2)

set -e

PUBLIC_IP=$(curl -s -4 https://ifconfig.me)

echo -e "export LATEST_BINARY=\"$LATEST_BINARY\"\nexport HOSTNAME=\"$HOSTNAME\"\nexport LOCATION=\"$LOCATION\"\nexport EMAIL=\"$EMAIL\"\nexport MONIKER=\"$MONIKER\"\nexport DESCRIPTION=\"$DESCRIPTION\"\nexport PUBLIC_IP=\"$PUBLIC_IP\" > env.sh && \

echo "Variables saved to env.sh — run 'source ./env.sh' to load them."
source ./env.sh
