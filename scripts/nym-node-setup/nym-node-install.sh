#!/bin/bash

# Create binaries dir & download the binary
echo "Checking if ~/nym-binaries directory exists..."

if [ ! -d "$HOME/nym-binaries" ]; then
    echo "Creating directory: ~/nym-binaries"
    mkdir "$HOME/nym-binaries"
else
    echo "Directory already exists: ~/nym-binaries"
fi

echo "Downloading latest binary of nym-node"

set -e
LATEST_BINARY=$(wget -qO - https://github.com/nymtech/nym/releases/latest \
  | grep -oP 'href="\/nymtech\/nym\/releases\/download\/[^"]+\/nym-node"' \
  | head -n 1 | cut -d'"' -f2)

curl -L "https://github.com$LATEST_BINARY" -o $HOME/nym-binaries/

# Make executable
NYM_NODE="$HOME/nym-binaries/nym-node"
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

# Determine public IP
PUBLIC_IP=$(curl -s -4 https://ifconfig.me)

# Default wireguard setting
WIREGUARD="false"

# Prompt for WireGuard if mode is gateway
if [[ "$MODE" == "entry-gateway" || "$MODE" == "exit-gateway" ]]; then
  echo "Gateways can also route WireGuard in NymVPN."
  echo "Do you want to enable this function?"
  echo "Please note that a node routing WireGuard will be listed as both entry and exit in the application."
  read -rp "Enable WireGuard support? (y/n): " answer
  case "$answer" in
    [Yy]* ) WIREGUARD="true";;
    [Nn]* ) WIREGUARD="false";;
    * ) echo "Invalid input. Defaulting to disabled."; WIREGUARD="false";;
  esac
fi

# Initialize node config
if [[ "$MODE" == "mixnode" ]]; then
  echo "Initialising nym-node in mode mixnode..."
  "$NYM_NODE" run \
    --mode mixnode \
    --public-ips "$PUBLIC_IP" \
    --hostname "$HOSTNAME" \
    --location "$LOCATION" \
    -w \
    --init-only

elif [[ "$MODE" == "entry-gateway" ]]; then
  echo "Initialising nym-node in mode entry-gateway..."
  "$NYM_NODE" run \
    --mode entry-gateway \
    --public-ips "$PUBLIC_IP" \
    --hostname "$HOSTNAME" \
    --location "$LOCATION" \
    --wireguard-enabled "$WIREGUARD" \
    --landing-page-assets-path "/var/www/${HOSTNAME}" \
    -w \
    --init-only

elif [[ "$MODE" == "exit-gateway" ]]; then
  echo "Initialising nym-node in mode exit-gateway..."

  if [[ -z "$HOSTNAME" || -z "$LOCATION" ]]; then
    echo "ERROR: HOSTNAME and LOCATION must be exported for exit-gateway."
    exit 1
  fi

  "$NYM_NODE" run \
    --mode exit-gateway \
    --public-ips "$PUBLIC_IP" \
    --hostname "$HOSTNAME" \
    --location "$LOCATION" \
    --wireguard-enabled "$WIREGUARD" \
    --announce-wss-port 9001 \
    --landing-page-assets-path "/var/www/${HOSTNAME}" \
    -w \
    --init-only

else
  echo "ERROR: Unsupported MODE: '$MODE'"
  echo "Valid values: mixnode, entry-gateway, exit-gateway"
  exit 1
fi


echo "nym-node installed succesfully! All configuration is stored at ~/.nym/nym-nodes/default-nym-node/"

# Setup description.toml

cat > $HOME/.nym/nym-nodes/default-nym-node/data/description.toml <<EOF
moniker = "$MONIKER"
website = "$HOSTNAME"
security_contact = "$EMAIL"
details = "$DESCRIPTION"
EOF

echo "Node description saved."
cat $HOME/.nym/nym-nodes/default-nym-node/data/description.toml
echo "You can always change it later on by editing ~/.nym/nym-nodes/default-nym-node/data/description.toml and restarting the node."
