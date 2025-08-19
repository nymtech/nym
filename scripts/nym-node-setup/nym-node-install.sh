#!/bin/bash
set -euo pipefail

echo -e "\n* * * Ensuring ~/nym-binaries exists * * *"
mkdir -p "$HOME/nym-binaries"

# Load env.sh via absolute path if provided, else try ./env.sh
if [[ -n "${ENV_FILE:-}" && -f "${ENV_FILE}" ]]; then
  set -a
  # shellcheck disable=SC1090
  . "${ENV_FILE}"
  set +a
elif [[ -f "./env.sh" ]]; then
  set -a
  # shellcheck disable=SC1091
  . ./env.sh
  set +a
fi

# check for existing node config and optionally reset
NODE_CONFIG_DIR="$HOME/.nym/nym-nodes/default-nym-node"

check_existing_config() {
  # proceed only if dir exists AND has any entries inside
  if [[ -d "$NODE_CONFIG_DIR" ]] && find "$NODE_CONFIG_DIR" -mindepth 1 -maxdepth 1 | read -r _; then
    echo
    echo "Nym node configuration already exist at $NODE_CONFIG_DIR"
    echo
    echo "Initialising nym-node again will NOT overwrite your existing private keys, only adjust your preferences (like mode, wireguard optionality etc)."
    echo
    echo "If you want to remove your current node configuration and all data files including nodes keys type 'RESET' and press enter."
    echo
    read -r -p "To keep your existing node and just change its preferences press enter: " resp

    if [[ "${resp}" =~ ^([Rr][Ee][Ss][Ee][Tt])$ ]]; then
      echo
      read -r -p "We are going to remove the existing node with configuration $NODE_CONFIG_DIR and replace it with a fresh one, do you want to back up the old one first? (y/n) " backup_ans
      if [[ "${backup_ans}" =~ ^[Yy]$ ]]; then
        ts="$(date +%Y%m%d-%H%M%S)"
        backup_dir="$HOME/.nym/backup/$(basename "$NODE_CONFIG_DIR")-$ts"
        echo "Backing up to: $backup_dir"
        mkdir -p "$(dirname "$backup_dir")"
        cp -a "$NODE_CONFIG_DIR" "$backup_dir"
      fi
      echo "Removing $NODE_CONFIG_DIR ..."
      rm -rf "$NODE_CONFIG_DIR"
      echo "Old node removed. Proceeding with fresh initialization..."
    else
      echo "Keeping existing node configuration. Proceeding to re-configure."
      export ASK_WG="1"
    fi
  fi
}

# run the check before any initialization
check_existing_config

echo -e "\n* * * Resolving latest release tag URL * * *"
LATEST_TAG_URL="$(curl -sI -L -o /dev/null -w '%{url_effective}' https://github.com/nymtech/nym/releases/latest)"
# expected example: https://github.com/nymtech/nym/releases/tag/nym-binaries-v2025.13-emmental

if [[ -z "${LATEST_TAG_URL}" || "${LATEST_TAG_URL}" != *"/releases/tag/"* ]]; then
  echo "ERROR: Could not resolve latest tag URL from GitHub." >&2
  exit 1
fi

DOWNLOAD_URL="${LATEST_TAG_URL/tag/download}/nym-node"
NYM_NODE="$HOME/nym-binaries/nym-node"

# if binary already exists, ask to overwrite; if yes, remove first
if [[ -e "${NYM_NODE}" ]]; then
  echo
  echo "A nym-node binary already exists at: ${NYM_NODE}"
  read -r -p "Overwrite with the latest release? (y/n): " ow_ans
  if [[ "${ow_ans}" =~ ^[Yy]$ ]]; then
    echo "Removing existing binary to avoid 'text file busy'..."
    rm -f "${NYM_NODE}"
  else
    echo "Keeping existing binary."
  fi
fi

echo -e "\n* * * Downloading nym-node from:"
echo "    ${DOWNLOAD_URL}"
# only download if file is missing (or we just removed it)
if [[ ! -e "${NYM_NODE}" ]]; then
  curl -fL "${DOWNLOAD_URL}" -o "${NYM_NODE}"
fi

echo -e "\n * * * Making binary executable * * *"
chmod +x "${NYM_NODE}"

echo "---------------------------------------------------"
echo "Nym node binary downloaded:"
"${NYM_NODE}" --version || true
echo "---------------------------------------------------"

# check that MODE is set (after sourcing env.sh)
if [[ -z "${MODE:-}" ]]; then
  echo "ERROR: Environment variable MODE is not set."
  echo "Please export MODE as one of: mixnode, entry-gateway, exit-gateway"
  exit 1
fi

# determine public IP (fallback if ifconfig.me fails)
echo -e "\n* * * Discovering public IP (IPv4) * * *"
if ! PUBLIC_IP="$(curl -fsS -4 https://ifconfig.me)"; then
  PUBLIC_IP="$(curl -fsS https://api.ipify.org || echo '')"
fi
if [[ -z "${PUBLIC_IP}" ]]; then
  echo "WARNING: Could not determine public IP automatically."
fi

# respect existing WIREGUARD; for gateways: prompt if unset OR if we kept config and ASK_WG=1
WIREGUARD="${WIREGUARD:-}"
if [[ ( "$MODE" == "entry-gateway" || "$MODE" == "exit-gateway" ) && ( -n "${ASK_WG:-}" || -z "$WIREGUARD" ) ]]; then
  echo
  echo "Gateways can also route WireGuard in NymVPN."
  echo "Enabling it means your node may be listed as both entry and exit in the app."
  # show current default in the prompt if present
  def_hint=""
  [[ -n "${WIREGUARD}" ]] && def_hint=" [current: ${WIREGUARD}]"
  read -r -p "Enable WireGuard support? (y/n)${def_hint}: " answer || true
  case "${answer:-}" in
    [Yy]* ) WIREGUARD="true" ;;
    [Nn]* ) WIREGUARD="false" ;;
    * )     : ;;  # keep existing value if user just pressed enter
  esac
fi
# final default only if still empty
WIREGUARD="${WIREGUARD:-false}"

# persist WIREGUARD to the same env file Python CLI uses
ENV_PATH="${ENV_FILE:-./env.sh}"
if [[ -n "$ENV_PATH" ]]; then
  mkdir -p "$(dirname "$ENV_PATH")"
  if [[ -f "$ENV_PATH" ]]; then
    # replace existing export or append
    if grep -qE '^[[:space:]]*export[[:space:]]+WIREGUARD=' "$ENV_PATH"; then
      sed -i -E 's|^[[:space:]]*export[[:space:]]+WIREGUARD=.*$|export WIREGUARD="'"$WIREGUARD"'"|' "$ENV_PATH"
    else
      printf '\nexport WIREGUARD="%s"\n' "$WIREGUARD" >> "$ENV_PATH"
    fi
  else
    printf 'export WIREGUARD="%s"\n' "$WIREGUARD" > "$ENV_PATH"
  fi
  echo "WIREGUARD=${WIREGUARD} persisted to $ENV_PATH"
fi

# helpers: ensure optional env vars exist (avoid -u issues)
HOSTNAME="${HOSTNAME:-}"
LOCATION="${LOCATION:-}"
EMAIL="${EMAIL:-}"
MONIKER="${MONIKER:-}"
DESCRIPTION="${DESCRIPTION:-}"

# initialize node config
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
    echo -e "\n* * * Initialising nym-node in mode: exit-gateway * * *"
    if [[ -z "${HOSTNAME:-}" || -z "${LOCATION:-}" ]]; then
      echo "ERROR: HOSTNAME and LOCATION must be exported for exit-gateway."
      exit 1
    fi
    "${NYM_NODE}" run \
      --mode exit-gateway \
      ${PUBLIC_IP:+--public-ips "$PUBLIC_IP"} \
      --hostname "$HOSTNAME" \
      --location "$LOCATION" \
      --wireguard-enabled "${WIREGUARD:-false}" \
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

# setup description.toml (if init created the dir)
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
