#!/bin/bash

# set -euo pipefail

echo -e "\n* * * Ensuring ~/nym-binaries exists * * *"
mkdir -p "$HOME/nym-binaries"

echo -e "\n* * * Resolving latest release tag URL * * *"
LATEST_TAG_URL="$(curl -sI -L -o /dev/null -w '%{url_effective}' https://github.com/nymtech/nym/releases/latest)"
# Example: https://github.com/nymtech/nym/releases/tag/nym-binaries-v2025.13-emmental

if [[ -z "${LATEST_TAG_URL}" || "${LATEST_TAG_URL}" != *"/releases/tag/"* ]]; then
  echo "ERROR: Could not resolve latest tag URL from GitHub." >&2
  exit 1
fi

DOWNLOAD_URL="${LATEST_TAG_URL/tag/download}/nym-node"
NYM_NODE="$HOME/nym-binaries/nym-node"

echo -e "\n * * * Downloading nym-node from:"
echo "    ${DOWNLOAD_URL}"
curl -fL "${DOWNLOAD_URL}" -o "${NYM_NODE}"

echo -e "\n * * * Making binary executable * * *"
chmod +x "${NYM_NODE}"

echo "---------------------------------------------------"
echo "Nym node binary downloaded:"
"${NYM_NODE}" --version || true
echo "---------------------------------------------------"

# Check that MODE is set
if [[ -z "${MODE:-}" ]]; then
  echo "ERROR: Environment variable MODE is not set."
  echo "Please export MODE as one of: mixnode, entry-gateway, exit-gateway"
  exit 1
fi

# Determine public IP (fallback if ifconfig.me fails)
echo -e "\n* * * Discovering public IP (IPv4) * * *"
if ! PUBLIC_IP="$(curl -fsS -4 https://ifconfig.me)"; then
  PUBLIC_IP="$(curl -fsS https://api.ipify.org || echo '')"
fi
if [[ -z "${PUBLIC_IP}" ]]; then
  echo "WARNING: Could not determine public IP automatically."
fi

# Default wireguard setting
WIREGUARD="false"

# Prompt for WireGuard if mode is gateway
if [[ "${MODE}" == "entry-gateway" || "${MODE}" == "exit-gateway" ]]; then
  echo
  echo "Gateways can also route WireGuard in NymVPN."
  echo "Enabling it means your node may be listed as both entry and exit in the app."
  read -r -p "Enable WireGuard support? (y/n) [n]: " answer || true
  case "${answer:-n}" in
    [Yy]* ) WIREGUARD="true";;
    * )     WIREGUARD="false";;
  esac
fi

# Helpers: ensure optional env vars exist (avoid unbound errors)
HOSTNAME="${HOSTNAME:-}"
LOCATION="${LOCATION:-}"
EMAIL="${EMAIL:-}"
MONIKER="${MONIKER:-}"
DESCRIPTION="${DESCRIPTION:-}"

# Initialize node config
case "${MODE}" in
  mixnode)
    echo -e "\n* * * Initialising nym-node in mode: mixnode * * *"
    "${NYM_NODE}" run \
      --mode mixnode \
      ${PUBLIC_IP:+--public-ips "$PUBLIC_IP"} \
      ${HOSTNAME:+--hostname "$HOSTNAME"} \
      ${LOCATION:+--location "$LOCATION"} \
      -w \
      --init-only
    ;;
  entry-gateway)
    echo -e "\n* * * Initialising nym-node in mode: entry-gateway * * *"
    "${NYM_NODE}" run \
      --mode entry-gateway \
      ${PUBLIC_IP:+--public-ips "$PUBLIC_IP"} \
      ${HOSTNAME:+--hostname "$HOSTNAME"} \
      ${LOCATION:+--location "$LOCATION"} \
      --wireguard-enabled "${WIREGUARD}" \
      ${HOSTNAME:+--landing-page-assets-path "/var/www/${HOSTNAME}"} \
      -w \
      --init-only
    ;;
  exit-gateway)
    echo -e "\n* * *Initialising nym-node in mode: exit-gateway * * *"
    if [[ -z "${HOSTNAME}" || -z "${LOCATION}" ]]; then
      echo "ERROR: HOSTNAME and LOCATION must be exported for exit-gateway."
      exit 1
    fi
    "${NYM_NODE}" run \
      --mode exit-gateway \
      ${PUBLIC_IP:+--public-ips "$PUBLIC_IP"} \
      --hostname "$HOSTNAME" \
      --location "$LOCATION" \
      --wireguard-enabled "${WIREGUARD}" \
      --announce-wss-port 9001 \
      --landing-page-assets-path "/var/www/${HOSTNAME}" \
      -w \
      --init-only
    ;;
  *)
    echo "ERROR: Unsupported MODE: '${MODE}'"
    echo "Valid values: mixnode, entry-gateway, exit-gateway"
    exit 1
    ;;
esac

echo
echo "* * * nym-node initialised. Config path should be:"
echo "    $HOME/.nym/nym-nodes/default-nym-node/"

# Setup description.toml (if init created the dir)
DESC_DIR="$HOME/.nym/nym-nodes/default-nym-node/data"
DESC_FILE="$DESC_DIR/description.toml"

if [[ -d "$DESC_DIR" ]]; then
  echo -e "\n* * * Writing node description: $DESC_FILE * * *"
  mkdir -p "$DESC_DIR"
  cat > "$DESC_FILE" <<EOF
moniker = "${MONIKER}"
website = "${HOSTNAME}"
security_contact = "${EMAIL}"
details = "${DESCRIPTION}"
EOF
  echo "Node description saved."
  echo "You can edit it later at: $DESC_FILE (restart node to apply)."
else
  echo "NOTE: Description directory not found yet ($DESC_DIR)."
  echo "      It will exist after a full init; you can create the file later."
fi
