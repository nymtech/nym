#!/usr/bin/env bash

# set -euo pipefail

echo -e "\n* * * Setting up environmental variables to ./env.sh * * *"

# Prompt user
read -rp "Enter hostname (if you don't use a DNS, press enter): " HOSTNAME
read -rp "Enter node location (country code or name): " LOCATION
read -rp "Enter your email: " EMAIL
read -rp "Enter node public moniker (name in the explorer and NymVPN app): " MONIKER
read -rp "Enter node public description: " DESCRIPTION

# Follow redirects and parse the latest nym-node binary URL
LATEST_BINARY=$(
  curl -fsSL https://github.com/nymtech/nym/releases/latest \
    | grep -Eo 'href="/nymtech/nym/releases/download/[^"]+/nym-node"' \
    | head -n1 \
    | cut -d'"' -f2
)

if [[ -z "${LATEST_BINARY:-}" ]]; then
  echo "ERROR: Could not determine latest nym-node binary URL." >&2
  exit 1
fi

PUBLIC_IP=$(curl -fsS -4 https://ifconfig.me || true)
PUBLIC_IP=${PUBLIC_IP:-""}

cat > env.sh <<EOF
export LATEST_BINARY="https://github.com${LATEST_BINARY}"
export HOSTNAME="${HOSTNAME}"
export LOCATION="${LOCATION}"
export EMAIL="${EMAIL}"
export MONIKER="${MONIKER}"
export DESCRIPTION="${DESCRIPTION}"
export PUBLIC_IP="${PUBLIC_IP}"
EOF

echo -e "\nVariables saved to ./env.sh"
echo "To load them into your current shell, run:  source ./env.sh"
