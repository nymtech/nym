#!/bin/bash

# Create binaries dir & download the binary
mkdir $HOME/nym-binaries

set -e
LATEST_BINARY=$(wget -qO - https://github.com/nymtech/nym/releases/latest \
  | grep -oP 'href="\/nymtech\/nym\/releases\/download\/[^"]+\/nym-node"' \
  | head -n 1 | cut -d'"' -f2)

curl -L "https://github.com$LATEST_BINARY" -o $HOME/nym-binaries/

# Make executable
NYM_BIN="$HOME/nym-binaries/nym-node"
chmod +x $NYM_NODE
echo "---------------------------------------------------"
echo "Nym node binary was downloaded and made executable."
$NYM_NODE --version
echo "---------------------------------------------------"

# Check that MODE is set
if [[ -z "$MODE" ]]; then
  echo "ERROR: Environment variable MODE is not set."
  echo "Please export MODE as one of: mixnode, entry-gateway, exit-gateway"
  exit 1
fi

# Initialiuse nym-node config based on MODE

PUBLIC_IP=$(curl -s -4 https://ifconfig.me)

if [[ "$MODE" == "mixnode" ]]; then
  echo "▶ Running in MIXNODE mode..."
  "$NYM_NODE" run
    --mode mixnode \
    --public-ips "$PUBLIC_IP" \
    --hostname "$HOSTNAME" \
    --location "$LOCATION" \
    -w \
    --init-only

elif [[ "$MODE" == "entry-gateway" ]]; then
  echo "▶ Running in ENTRY-GATEWAY mode..."
  "$NYM_BIN" run --mode entry-gateway
    --public-ips "$PUBLIC_IP" \
    --hostname "$HOSTNAME" \
    --location "$LOCATION" \
    --wireguard-enabled true \
    --landing-page-assets-path "/var/www/${HOSTNAME}" \
    -w \
    --init-only

elif [[ "$MODE" == "exit-gateway" ]]; then
  echo "▶ Running in EXIT-GATEWAY mode..."

  # Ensure required env vars
  if [[ -z "$HOSTNAME" || -z "$LOCATION" ]]; then
    echo "❌ ERROR: HOSTNAME and LOCATION must be exported for exit-gateway."
    exit 1
  fi


  "$NYM_BIN" run \
    --mode exit-gateway \
    --public-ips "$PUBLIC_IP" \
    --hostname "$HOSTNAME" \
    --location "$LOCATION" \
    --wireguard-enabled true \
    --announce-wss-port 9001 \
    --landing-page-assets-path "/var/www/${HOSTNAME}" \
    -w \
    --init-only

else
  echo "ERROR: Unsupported MODE: '$MODE'"
  echo "Valid values: mixnode, entry-gateway, exit-gateway"
  exit 1
fi
