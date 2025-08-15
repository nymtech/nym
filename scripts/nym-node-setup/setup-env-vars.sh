#!/usr/bin/env bash
# setup_env.sh

# set -euo pipefail

echo -e "\n* * * Setting up environmental variables to ./env.sh * * *"

# Detect if we're being sourced
if [[ "${BASH_SOURCE[0]}" != "$0" ]]; then
  __SOURCED=1
else
  __SOURCED=0
fi

# Prompt user
read -rp "Enter hostname (if you don't use a DNS, press enter): " HOSTNAME
read -rp "Enter node location (country code or name): " LOCATION
read -rp "Enter your email: " EMAIL
read -rp "Enter node public moniker (visible in the explorer and NymVPN app): " MONIKER
read -rp "Enter node public description: " DESCRIPTION

# Try to get a latest binary URL, but DO NOT fail if it can’t be parsed
LATEST_BINARY=$(
  curl -fsSL https://github.com/nymtech/nym/releases/latest \
    | grep -Eo 'href="/nymtech/nym/releases/download/[^"]+/nym-node"' \
    | head -n1 \
    | cut -d'"' -f2
)
if [[ -z "${LATEST_BINARY:-}" ]]; then
  echo "WARNING: Could not determine latest nym-node binary URL right now. The installer will resolve it later."
fi

PUBLIC_IP=$(curl -fsS -4 https://ifconfig.me || true)
PUBLIC_IP=${PUBLIC_IP:-""}

# Write env.sh even if LATEST_BINARY is missing
{
  [[ -n "${LATEST_BINARY:-}" ]] && echo "export LATEST_BINARY=\"https://github.com${LATEST_BINARY}\""
  echo "export HOSTNAME=\"${HOSTNAME}\""
  echo "export LOCATION=\"${LOCATION}\""
  echo "export EMAIL=\"${EMAIL}\""
  echo "export MONIKER=\"${MONIKER}\""
  echo "export DESCRIPTION=\"${DESCRIPTION}\""
  echo "export PUBLIC_IP=\"${PUBLIC_IP}\""
} > env.sh

echo -e "\nVariables saved to ./env.sh"

if [[ $__SOURCED -eq 1 ]]; then
  # shellcheck disable=SC1091
  . ./env.sh
  echo "Loaded into current shell (because you sourced this script)."
else
  echo "To load them into your current shell, run:  source ./env.sh"
fi
