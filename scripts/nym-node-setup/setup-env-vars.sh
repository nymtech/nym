#!/usr/bin/env bash
# setup_env.sh

# set -euo pipefail

echo -e "\n* * * Setting up environmental variables to ./env.sh * * *"

# Detect if we're being sourced
# (when sourced: BASH_SOURCE[0] != $0)
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

# Follow redirects and parse the latest nym-node binary URL
LATEST_BINARY=$(
  curl -fsSL https://github.com/nymtech/nym/releases/latest \
    | grep -Eo 'href="/nymtech/nym/releases/download/[^"]+/nym-node"' \
    | head -n1 \
    | cut -d'"' -f2
)

if [[ -z "${LATEST_BINARY:-}" ]]; then
  echo "ERROR: Could not determine latest nym-node binary URL." >&2
  return 1 2>/dev/null || exit 1
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

if [[ $__SOURCED -eq 1 ]]; then
  # We're sourced -> load into the current shell now
  # shellcheck source=/dev/null
  . ./env.sh
  echo "Loaded into current shell (because you sourced this script)."
else
  echo "To load them into your current shell, run:  source ./env.sh"
fi
