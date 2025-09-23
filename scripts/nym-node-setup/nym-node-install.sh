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

# -----------------------------------------------------------------------------
# Robust downloader for nym-node: probes platform-specific assets; if none exist,
# asks user for a direct downloadable URL and validates it.
# -----------------------------------------------------------------------------
download_nym_node() {
  local latest_tag_url="$1"          # e.g. "https://github.com/nymtech/nym/releases/tag/nym-binaries-v2025.13-emmental"
  local dest_path="$2"               # e.g. "$HOME/nym-binaries/nym-node"
  local base_download_url
  local os arch exe_ext=""
  local candidates=()
  local found_url=""
  local http_code=""

  if [[ -z "$latest_tag_url" || "$latest_tag_url" != *"/releases/tag/"* ]]; then
    echo "ERROR: Invalid latest tag URL: $latest_tag_url" >&2
    return 1
  fi

  base_download_url="${latest_tag_url/tag/download}"

  # Detect OS / ARCH
  case "$(uname -s | tr '[:upper:]' '[:lower:]')" in
    linux*)  os="linux" ;;
    darwin*) os="darwin" ;;
    msys*|cygwin*|mingw*) os="windows"; exe_ext=".exe" ;;
    *) echo "WARNING: Unknown OS; defaulting to linux." ; os="linux" ;;
  esac

  case "$(uname -m)" in
    x86_64|amd64) arch="x86_64" ;;
    aarch64|arm64) arch="aarch64" ;;
    armv7*|armv6*|armv5*) arch="arm" ;;
    *) arch="$(uname -m)";;
  esac

  # Candidate asset names to probe (no assumptions about compression)
  if [[ "$os" == "linux" ]]; then
    candidates+=(
      "nym-node"
      "nym-node-${arch}-unknown-linux-gnu"
      "nym-node-${arch}-unknown-linux-musl"
    )
  elif [[ "$os" == "darwin" ]]; then
    candidates+=(
      "nym-node-${arch}-apple-darwin"
      "nym-node"
    )
  elif [[ "$os" == "windows" ]]; then
    candidates+=(
      "nym-node-${arch}-pc-windows-msvc${exe_ext}"
      "nym-node${exe_ext}"
    )
  fi

  # Helper: return 0 if URL exists (HTTP 200)
  url_exists() {
    local url="$1"
    http_code="$(curl -sI -L -o /dev/null -w '%{http_code}' "$url")"
    [[ "$http_code" == "200" ]]
  }

  echo -e "\n* * * Probing release assets for your platform ($os/$arch) * * *"

  # Try candidate assets in order
  for name in "${candidates[@]}"; do
    local try_url="${base_download_url}/${name}"
    if url_exists "$try_url"; then
      found_url="$try_url"
      break
    fi
  done

  # If nothing found, prompt the user for a URL
  if [[ -z "$found_url" ]]; then
    echo
    echo "⚠️  Could not find a 'nym-node' asset for your platform in the latest release:"
    echo "    $latest_tag_url"
    echo
    echo "HTTP check for first candidate (${base_download_url}/${candidates[0]}): ${http_code:-n/a}"
    echo
    echo "Please paste a direct, downloadable URL for the 'nym-node' binary for your platform."
    echo "Tip: Open the GitHub release page, right-click the correct asset, and copy link address."
    read -r -p "Custom download URL: " user_url

    if [[ -z "${user_url// }" ]]; then
      echo "ERROR: No URL provided. Aborting."
      return 1
    fi
    if ! url_exists "$user_url"; then
      echo "ERROR: The provided URL does not appear downloadable (HTTP $http_code). Aborting."
      return 1
    fi
    found_url="$user_url"
  fi

  echo -e "\n* * * Downloading nym-node from: $found_url * * *"
  mkdir -p "$(dirname "$dest_path")"

  # Remove any existing file to avoid 'text file busy'
  if [[ -e "$dest_path" ]]; then
    echo "Removing existing binary at $dest_path ..."
    rm -f "$dest_path"
  fi

  if ! curl -fL "$found_url" -o "$dest_path"; then
    echo "ERROR: Download failed from $found_url" >&2
    return 1
  fi

  chmod +x "$dest_path" 2>/dev/null || true

  echo "---------------------------------------------------"
  echo "Nym node binary downloaded to: $dest_path"
  "$dest_path" --version || true
  echo "---------------------------------------------------"
}
# -----------------------------------------------------------------------------


echo -e "\n* * * Resolving latest release tag URL * * *"
LATEST_TAG_URL="$(curl -sI -L -o /dev/null -w '%{url_effective}' https://github.com/nymtech/nym/releases/latest)"
# expected example: https://github.com/nymtech/nym/releases/tag/nym-binaries-v2025.13-emmental

if [[ -z "${LATEST_TAG_URL}" || "${LATEST_TAG_URL}" != *"/releases/tag/"* ]]; then
  echo "ERROR: Could not resolve latest tag URL from GitHub." >&2
  exit 1
fi

NYM_NODE="$HOME/nym-binaries/nym-node"

# if binary already exists, ask to overwrite; if yes, remove first
if [[ -e "${NYM_NODE}" ]]; then
  echo
  echo -e "\n* * * A nym-node binary already exists at: ${NYM_NODE}"
  read -r -p "Overwrite with the latest release? (y/n): " ow_ans
  if [[ "${ow_ans}" =~ ^[Yy]$ ]]; then
    echo "Removing existing binary to avoid 'text file busy'..."
    rm -f "${NYM_NODE}"
  else
    echo "Keeping existing binary."
  fi
fi

# Use robust downloader (prompts for URL if platform asset is missing)
download_nym_node "$LATEST_TAG_URL" "$NYM_NODE"

echo -e "\n * * * Making binary executable * * *"
chmod +x "${NYM_NODE}"

echo "---------------------------------------------------"
echo "Nym node binary downloaded:"
"${NYM_NODE}" --version || true
echo "---------------------------------------------------"

# check that MODE is set (after sourcing env.sh or other scripts)
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
  echo "* * * Node description saved * * *"
  echo "You can edit it later at: $DESC_FILE (restart node to apply)."
else
  echo "NOTE: Description directory not found yet ($DESC_DIR)."
  echo "      It will exist after a full init; you can create the file later."
fi
