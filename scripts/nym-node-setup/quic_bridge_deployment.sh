#!/usr/bin/env bash
# Nym QUIC Bridge Deployment Helper Script
# Uses bridge-cfg to generate a correct bridges.toml and keys
# installs/maintains the nym-bridge service, and provides network helpers
# read about nym-bridges: https://github.com/nymtech/nym-bridges
# read about bridge-cfg: https://github.com/nymtech/nym-bridges/tree/main/bridge-cfg
# RUN AS ROOT

set -euo pipefail
set +o errtrace

# Colors
RED="\033[0;31m"
GREEN="\033[0;32m"
YELLOW="\033[0;33m"
CYAN="\033[0;36m"
BOLD="\033[1m"
RESET="\033[0m"

# Logging
LOG_FILE="/var/log/nym/quic_bridge_deployment.log"
mkdir -p "$(dirname "$LOG_FILE")"

# rotate log if >10MB BEFORE writing START header
if [[ -f "$LOG_FILE" && $(stat -c%s "$LOG_FILE") -gt 10485760 ]]; then
  mv "$LOG_FILE" "${LOG_FILE}.1"
fi

touch "$LOG_FILE"
chmod 640 "$LOG_FILE"

echo "----- $(date '+%Y-%m-%d %H:%M:%S') START quic-bridge-manager -----" | tee -a "$LOG_FILE"
echo -e "${CYAN}Logs are being saved locally to:${RESET} $LOG_FILE"
echo -e "${CYAN}These logs never leave your machine.${RESET}"
echo "" | tee -a "$LOG_FILE"

# safe logger function
log() {
  echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" | tee -a "$LOG_FILE"
}

# global redirection, strip ANSI before writing to log
add_log_redirection() {
  exec > >(tee >(sed -u 's/\x1b\[[0-9;]*m//g' >> "$LOG_FILE"))
  exec 2> >(tee >(sed -u 's/\x1b\[[0-9;]*m//g' >> "$LOG_FILE") >&2)
}
add_log_redirection

START_TIME=$(date +%s)


# Constants / Paths
REQUIRED_CMDS=(ip jq curl openssl dpkg)
BRIDGE_BIN="/usr/local/bin/nym-bridge"
BRIDGE_CFG_BIN="/usr/local/bin/bridge-cfg"

NYM_ETC_DIR="/etc/nym"
NYM_ETC_KEYS_DIR="$NYM_ETC_DIR/keys"
NYM_ETC_BRIDGES="$NYM_ETC_DIR/bridges.toml"
NYM_ETC_CLIENT_PARAMS_DEFAULT="$NYM_ETC_DIR/client_bridge_params.json"
SERVICE_FILE="/etc/systemd/system/nym-bridge.service"

NET4_DEV="${IPV4_UPLINK_DEV:-}"
if [[ -z "$NET4_DEV" ]]; then
  NET4_DEV="$(ip -o route show default 2>/dev/null | awk '{print $5}' | head -n1)"
  [[ -z "$NET4_DEV" ]] && NET4_DEV="$(ip -4 -o route get "$(getent ahostsv4 "ifconfig.co" | awk '$2=="STREAM" {print $1}' | head -n1)" 2>/dev/null | awk '{print $5}')"
fi
if [[ -z "$NET4_DEV" ]]; then
  echo -e "${RED}Cannot determine uplink interface. Set IPV4_UPLINK_DEV.${RESET}" | tee -a "$LOG_FILE"
  exit 1
fi
echo "Using ipv4 uplink device: $NET4_DEV"

NET6_DEV="${IPV6_UPLINK_DEV:-}"
if [[ -z "$NET6_DEV" ]]; then
  NET6_DEV="$(ip -o route show default 2>/dev/null | awk '{print $5}' | head -n1)"
  [[ -z "$NET6_DEV" ]] && NET6_DEV="$(ip -6 -o route get "$(getent ahostsv6 "ifconfig.co" | awk '$2=="STREAM" {print $1}' | head -n1)" 2>/dev/null | awk '{print $5}')"
fi
if [[ -z "$NET6_DEV" ]]; then
  echo -e "${RED}Cannot determine uplink interface. Set IPV6_UPLINK_DEV.${RESET}" | tee -a "$LOG_FILE"
  exit 1
fi
echo "Using ipv6 uplink device: $NET6_DEV"

WG_IFACE="nymwg"

# Root check
if [[ "$(id -u)" -ne 0 ]]; then
  echo -e "\n${RED}This script must be run as root.${RESET}\n"
  exit 1
fi

# UI helpers
hr() { echo -e "${YELLOW}----------------------------------------${RESET}" ; }
title() { echo -e "\n${YELLOW}==========================================${RESET}\n${YELLOW}  $1${RESET}\n${YELLOW}==========================================${RESET}\n"; }
ok() { echo -e "${GREEN}$1${RESET}"; }
warn() { echo -e "${YELLOW}$1${RESET}"; }
err() { echo -e "${RED}$1${RESET}"; }
info() { echo -e "${CYAN}$1${RESET}"; }
press_enter() {
  echo -n "$1" > /dev/tty
  read -r _ < /dev/tty
}


# Disable pauses and interactive prompts for noninteractive mode
if [[ "${NONINTERACTIVE:-0}" == "1" ]]; then
    press_enter() { :; }
    echo_prompt() { :; }
else
    press_enter() {
        echo -n "$1" > /dev/tty
        read -r _ < /dev/tty
    }
    echo_prompt() { echo -n "$1"; }
fi

# Helper: detect dpkg dependency failure for libc6>=2.34
deb_depends_libc_too_old() {
  local v
  v="$(dpkg-query -W -f='${Version}\n' libc6 2>/dev/null || true)"
  if [[ -z "$v" ]]; then return 0; fi
  dpkg --compare-versions "$v" ge "2.34" && return 1 || return 0
}

# Helper: ensure rust toolchain (for local build fallback)
ensure_rustup() {
  if ! command -v cargo >/dev/null 2>&1; then
    info "Installing Rust toolchain (rustup)..."
    apt-get update -y
    DEBIAN_FRONTEND=noninteractive apt-get install -y ca-certificates curl build-essential pkg-config libssl-dev git
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    export PATH="$HOME/.cargo/bin:$PATH"
  else
    export PATH="$HOME/.cargo/bin:$PATH"
  fi
}

# Helper: clone and build from source at latest release tag
build_from_source_latest() {
  local repo_url="https://github.com/nymtech/nym-bridges.git"
  local workdir="/tmp/nym-bridges"
  local tag
  info "Determining latest release tag from GitHub..."
  tag="$(curl -fsSL https://api.github.com/repos/nymtech/nym-bridges/releases/latest | jq -r .tag_name 2>/dev/null || true)"
  if [[ -z "$tag" || "$tag" == "null" ]]; then
    warn "Could not detect tag automatically. Falling back to 'main'."
    tag="main"
  fi

  info "Cloning $repo_url (tag/branch: $tag) into $workdir ..."
  rm -rf "$workdir"
  git clone --depth 1 --branch "$tag" "$repo_url" "$workdir"
  (cd "$workdir" && cargo fetch)

  info "Building from source (release)..."
  (
    cd "$workdir"
    cargo build --release -p nym-bridge
    cargo build --release -p bridge-cfg
  )
}

# Helper: robustly locate and install a built binary
install_built_binary() {
  local name="$1"
  local preferred="/tmp/nym-bridges/target/release/$name"

  if [[ -x "$preferred" ]]; then
    install -m 0755 "$preferred" "/usr/local/bin/$name"
    ok "Installed $name from $preferred to /usr/local/bin/$name"
    return 0
  fi

  local alt1="/tmp/nym-bridges/$name/target/release/$name"
  if [[ -x "$alt1" ]]; then
    install -m 0755 "$alt1" "/usr/local/bin/$name"
    ok "Installed $name from $alt1 to /usr/local/bin/$name"
    return 0
  fi

  local found
  found="$(find /tmp/nym-bridges -maxdepth 8 -type f -name "$name" -perm -111 2>/dev/null | head -n1 || true)"
  if [[ -n "$found" ]]; then
    install -m 0755 "$found" "/usr/local/bin/$name"
    ok "Installed $name from $found to /usr/local/bin/$name"
    return 0
  fi

  err "Built $name not found under /tmp/nym-bridges after build."
  return 1
}

# Prerequisites
verify_bridge_prerequisites() {
  title "Verifying Bridge Prerequisites"
  local all_good=true

  for c in "${REQUIRED_CMDS[@]}"; do
    if command -v "$c" >/dev/null 2>&1; then ok "✓ $c installed"; else err "$c missing"; all_good=false; fi
  done

  echo ""
  if ! dpkg -s iptables-persistent >/dev/null 2>&1; then
    warn "iptables-persistent not installed"
    info "Installing iptables-persistent..."
    apt-get update -y && DEBIAN_FRONTEND=noninteractive apt-get install -y iptables-persistent
  else
    ok "✓ iptables-persistent installed"
  fi

  # Ensure /etc/nym exists and has correct permissions for Ubuntu 24+
  mkdir -p "$NYM_ETC_DIR"
  chgrp nym "$NYM_ETC_DIR" 2>/dev/null || true
  chmod 750 "$NYM_ETC_DIR"
  ok "✓ Ensured /etc/nym exists with group 'nym' and mode 750"

  echo ""
  local v4=$(cat /proc/sys/net/ipv4/ip_forward 2>/dev/null || echo 0)
  local v6=$(cat /proc/sys/net/ipv6/conf/all/forwarding 2>/dev/null || echo 0)
  [[ "$v4" == "1" ]] && ok "✓ IPv4 forwarding enabled" || warn "IPv4 forwarding disabled"
  [[ "$v6" == "1" ]] && ok "✓ IPv6 forwarding enabled" || warn "IPv6 forwarding disabled"
  echo ""

  [[ "$all_good" == true ]] && ok "All prerequisites satisfied!" || warn "Some prerequisites missing."
}

adjust_ip_forwarding() {
  title "Checking IP forwarding"
  local v4 v6
  v4="$(cat /proc/sys/net/ipv4/ip_forward 2>/dev/null || echo 0)"
  v6="$(cat /proc/sys/net/ipv6/conf/all/forwarding 2>/dev/null || echo 0)"

  if [[ "$v4" == "1" ]]; then
    ok "IPv4 forwarding is enabled."
  else
    warn "IPv4 forwarding is not enabled."
  fi

  if [[ "$v6" == "1" ]]; then
    ok "IPv6 forwarding is enabled."
  else
    warn "IPv6 forwarding is not enabled."
  fi

  if [[ "$v4" != "1" || "$v6" != "1" ]]; then
    echo
    echo "To enable forwarding and routing consistently, run the network tunnel manager script as root."
    echo "For example:"
    echo "  ./network-tunnel-manager.sh complete_networking_configuration"
    echo "or:"
    echo "  ./network-tunnel-manager.sh adjust_ip_forwarding"
  fi
}

# Install nym-bridge
install_bridge_binary() {
  title "Installing nym-bridge Binary"

  # Handle Ubuntu 24+ case: system-installed path
  if [[ -x /usr/bin/nym-bridge && ! -x /usr/local/bin/nym-bridge ]]; then
    cp /usr/bin/nym-bridge /usr/local/bin/nym-bridge
    chmod +x /usr/local/bin/nym-bridge
    ok "Copied nym-bridge from /usr/bin to /usr/local/bin/"
  fi

  info "Fetching latest nym-bridge .deb from GitHub..."
  local deb_url
  deb_url="$(curl -fsSL https://api.github.com/repos/nymtech/nym-bridges/releases/latest \
     | grep -Eo 'https://[^"]*/nym-bridge_[0-9.]+-1_amd64.deb' | head -n1 || true)"

  if [[ -z "$deb_url" ]]; then
    warn "Falling back to known version (v0.1.2)"
    deb_url="https://github.com/nymtech/nym-bridges/releases/download/bridge-binaries-v0.1.2/nym-bridge_0.1.2-1_amd64.deb"
  fi

  info "Downloading from: $deb_url"
  curl -fL -o /tmp/nym-bridge.deb "$deb_url" || true

  if [[ -s /tmp/nym-bridge.deb ]]; then
    set +e
    dpkg -i /tmp/nym-bridge.deb
    local dpkg_rc=$?
    set -e
    if [[ $dpkg_rc -ne 0 ]]; then
      warn "dpkg reported errors; checking for libc6>=2.34 requirement..."
      if deb_depends_libc_too_old; then
        warn "System libc6 appears older than 2.34. Building nym-bridge from source."
        ensure_rustup
        build_from_source_latest
        install_built_binary "nym-bridge"
      else
        err "Failed to install nym-bridge .deb for non-libc reason; attempting source build."
        ensure_rustup
        build_from_source_latest
        install_built_binary "nym-bridge"
      fi
    else
      ok "nym-bridge installed via .deb."
    fi
  else
    warn "Download failed or empty. Building nym-bridge from source."
    ensure_rustup
    build_from_source_latest
    install_built_binary "nym-bridge"
  fi

  # Detect alternate binary location (Ubuntu 24+)
  if [[ -x /usr/bin/nym-bridge ]]; then
    BRIDGE_BIN="/usr/bin/nym-bridge"
    ok "Detected nym-bridge binary in /usr/bin"
  fi
}

# Install bridge-cfg
install_bridge_cfg_tool() {
  title "Installing bridge-cfg Tool"

  # Fix for Ubuntu 24+
  if [[ -x /usr/bin/bridge-cfg && ! -x /usr/local/bin/bridge-cfg ]]; then
    cp /usr/bin/bridge-cfg /usr/local/bin/bridge-cfg
    chmod +x /usr/local/bin/bridge-cfg
    ok "Copied bridge-cfg from /usr/bin to /usr/local/bin/"
  fi

  info "Attempting to fetch latest bridge-cfg from GitHub..."
  local cfg_url
  cfg_url="$(curl -fsSL https://api.github.com/repos/nymtech/nym-bridges/releases/latest \
     | grep -Eo 'https://[^"]*/bridge-cfg' | head -n1 || true)"

  if [[ -z "$cfg_url" ]]; then
    warn "Falling back to v0.1.2"
    cfg_url="https://github.com/nymtech/nym-bridges/releases/download/bridge-binaries-v0.1.2/bridge-cfg"
  fi

  info "Downloading: $cfg_url"
  if curl -fL -o "$BRIDGE_CFG_BIN" "$cfg_url"; then
    chmod +x "$BRIDGE_CFG_BIN"
    if "$BRIDGE_CFG_BIN" --help >/dev/null 2>&1; then
      ok "bridge-cfg installed at $BRIDGE_CFG_BIN"
      return 0
    else
      warn "Prebuilt bridge-cfg is incompatible (likely GLIBC too old). Building locally..."
    fi
  else
    warn "Failed to download bridge-cfg; building locally..."
  fi

  ensure_rustup
  build_from_source_latest
  install_built_binary "bridge-cfg"

  if [[ -x /usr/bin/bridge-cfg ]]; then
    BRIDGE_CFG_BIN="/usr/bin/bridge-cfg"
    ok "Detected bridge-cfg binary in /usr/bin"
  fi
}

# Generate config via bridge-cfg (with backup)
run_bridge_cfg_generate() {
  title "Generating Bridge Configuration with bridge-cfg"

  local HOME_DIR="${HOME:-/root}"
  local NODE_CFG
  NODE_CFG="$(find "$HOME_DIR/.nym/nym-nodes" -type f -name config.toml 2>/dev/null | head -n1 || true)"
  if [[ -z "$NODE_CFG" ]]; then
    NODE_CFG="$HOME_DIR/.nym/nym-nodes/default-nym-node/config/config.toml"
  fi

  echo -n "Path to your nym-node config.toml [default: $NODE_CFG]: "
  read -r input
  [[ -n "$input" ]] && NODE_CFG="$input"

  if [[ ! -f "$NODE_CFG" ]]; then
    err "nym-node config not found: $NODE_CFG"
    exit 1
  fi

  local NODE_ID
  NODE_ID="$(basename "$(dirname "$(dirname "$NODE_CFG")")")"
  local BACKUP_DIR="$HOME/.nym/backup/$NODE_ID/config"
  mkdir -p "$BACKUP_DIR"
  local TS
  TS="$(date +%Y%m%d_%H%M%S)"
  local BACKUP_FILE="$BACKUP_DIR/config.toml.bak$TS"
  cp "$NODE_CFG" "$BACKUP_FILE"
  ok "Backup created: $BACKUP_FILE"

  mkdir -p "$NYM_ETC_DIR" "$NYM_ETC_KEYS_DIR"
  chgrp nym "$NYM_ETC_DIR" 2>/dev/null || true
  chmod 750 "$NYM_ETC_DIR"
  chmod 700 "$NYM_ETC_KEYS_DIR"
  touch "$NYM_ETC_CLIENT_PARAMS_DEFAULT" || true

  info "Running: bridge-cfg --gen -n \"$NODE_CFG\" -d \"$NYM_ETC_DIR\" -o \"$NYM_ETC_BRIDGES\""
  set +e
  "$BRIDGE_CFG_BIN" --gen -n "$NODE_CFG" -d "$NYM_ETC_DIR" -o "$NYM_ETC_BRIDGES"
  local rc=$?
  set -e
  if [[ $rc -ne 0 ]]; then
    err "bridge-cfg failed to generate config. Aborting."
    exit 1
  fi

  chmod 600 "$NYM_ETC_BRIDGES"
  mkdir -p "$NYM_ETC_KEYS_DIR" && chmod 700 "$NYM_ETC_KEYS_DIR"
  chmod 600 "$NYM_ETC_KEYS_DIR"/* 2>/dev/null || true

  ok "bridge-cfg completed. bridges.toml generated at $NYM_ETC_BRIDGES"
  echo ""
  hr
  head -n 50 "$NYM_ETC_BRIDGES"
  hr

  export LAST_BACKUP_FILE="$BACKUP_FILE"
}

# Systemd service
create_bridge_service() {
  title "Creating nym-bridge systemd Service"

  if systemctl list-unit-files | grep -q '^nym-bridge\.service'; then
    warn "Detected existing nym-bridge service (likely from .deb). Not overwriting."
    systemctl daemon-reload || true
    systemctl enable nym-bridge >/dev/null || true
    systemctl restart nym-bridge || true
    ok "nym-bridge service managed by package; restarted."
    return 0
  fi

  if [[ ! -x "$BRIDGE_BIN" ]]; then err "Missing $BRIDGE_BIN"; exit 1; fi
  if [[ ! -f "$NYM_ETC_BRIDGES" ]]; then err "Missing $NYM_ETC_BRIDGES"; exit 1; fi

  mkdir -p /etc/systemd/system

  cat > "$SERVICE_FILE" <<EOF
[Unit]
Description=Nym QUIC Bridge
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=root
ExecStart=$BRIDGE_BIN --config $NYM_ETC_BRIDGES
Restart=on-failure
RestartSec=5
LimitNOFILE=65535
ProtectSystem=full
ProtectHome=yes
PrivateTmp=yes


[Install]
WantedBy=multi-user.target
EOF

  systemctl daemon-reload
  systemctl enable nym-bridge >/dev/null || true
  systemctl restart nym-bridge || true
  ok "nym-bridge service deployed."
}

# IPTABLES & helpers
apply_bridge_iptables_rules() {
  title "Checking iptables rules for bridge routing"

  echo "Inspecting current iptables state for interface ${WG_IFACE} and uplink ${NET4_DEV}."
  echo

  echo "IPv4 FORWARD:"
  iptables -L FORWARD -n -v 2>/dev/null | sed -n '1,20p' || echo "iptables not available."
  echo
  echo "IPv4 NAT POSTROUTING:"
  iptables -t nat -L POSTROUTING -n -v 2>/dev/null | sed -n '1,20p' || true
  echo

  echo "Inspecting current ip6tables state for interface ${WG_IFACE} and uplink ${NET6_DEV}."
  echo

  echo "IPv6 FORWARD:"
  ip6tables -L FORWARD -n -v 2>/dev/null | sed -n '1,20p' || true
  echo
  echo "IPv6 NAT POSTROUTING:"
  ip6tables -t nat -L POSTROUTING -n -v 2>/dev/null | sed -n '1,20p' || true

  echo
  echo "This script no longer changes iptables. To configure routing and NAT for nym, use the network tunnel manager script."
  echo "For example (run as root):"
  echo "  ./network-tunnel-manager.sh complete_networking_configuration"
}

configure_dns_and_icmp() {
  title "Checking ICMP and DNS firewall rules"

  echo "IPv4 INPUT rules related to ICMP and DNS:"
  iptables -L INPUT -n -v 2>/dev/null | grep -E 'icmp|dpt:53' || echo "no matching IPv4 rules shown."
  echo
  echo "IPv6 INPUT rules related to ICMP and DNS:"
  ip6tables -L INPUT -n -v 2>/dev/null | grep -E 'icmp|dpt:53' || echo "no matching IPv6 rules shown."

  echo
  echo "If ping or DNS are blocked for bridge traffic, adjust your firewall using the network tunnel manager script or your chosen firewall tool."
}

# Full interactive setup
full_bridge_setup() {
  title "Nym QUIC Bridge - Full Setup"

  echo -e "This will guide you through complete bridge setup.\n"

  for fn in verify_bridge_prerequisites install_bridge_binary install_bridge_cfg_tool \
            run_bridge_cfg_generate create_bridge_service adjust_ip_forwarding \
            apply_bridge_iptables_rules configure_dns_and_icmp; do
    if ! declare -f "$fn" >/dev/null; then
      err "Internal error: required function '$fn' is missing."
      exit 1
    fi
  done

  echo "Step 1/6: Checking prerequisites..."
  verify_bridge_prerequisites
  press_enter "Press Enter to continue..."

  echo ""
  echo "Step 2/6: Installing bridge binary..."
  install_bridge_binary
  echo "[Bridge Install] $(date '+%F %T') $( $BRIDGE_BIN --version 2>/dev/null || echo 'nym-bridge (unknown)')" \
    >> /var/log/nym/nym-bridge-version.log
  press_enter "Press Enter to continue..."

  echo ""
  echo "Step 3/6: Installing bridge-cfg tool..."
  install_bridge_cfg_tool
  press_enter "Press Enter to continue..."

  echo ""
  echo "Step 4/6: Generating configuration with bridge-cfg..."
  run_bridge_cfg_generate
  press_enter "Press Enter to continue..."

  echo ""
  echo "Step 5/6: Creating and starting systemd service..."
  create_bridge_service
  press_enter "Press Enter to continue..."

  echo ""
  echo "Step 6/6: Checking network rules and forwarding status..."
  adjust_ip_forwarding
  apply_bridge_iptables_rules
  configure_dns_and_icmp

  title "Bridge Setup Complete!"

  if systemctl --quiet is-active nym-bridge; then
    ok "nym-bridge service is running."
  else
    warn "nym-bridge failed to start. Check logs with:"
    echo "  journalctl -u nym-bridge -n 50 --no-pager"
  fi

  echo ""
  ok "Setup completed successfully."

  echo ""
  echo -e "${YELLOW}------------------------------------------${RESET}"
  echo -e "All done! You can safely close this session."
  echo -e "${YELLOW}------------------------------------------${RESET}"

  hr
  echo -e "${CYAN}Next steps and verification:${RESET}"
  hr
  echo ""
  echo -e "${YELLOW}To verify that the Nym Bridge service is active:${RESET}"
  echo "  systemctl status nym-bridge --no-pager"
  echo "  journalctl -u nym-bridge -n 50 --no-pager"
  echo ""
  echo -e "${YELLOW}To view live logs in real time:${RESET}"
  echo "  journalctl -u nym-bridge -f"
  echo ""
  echo -e "${YELLOW}To restart or reload the bridge service later:${RESET}"
  echo "  systemctl restart nym-bridge"
  echo ""
  echo -e "${YELLOW}To ensure your nym-node is properly aligned with the bridge:${RESET}"
  echo "  systemctl restart nym-node"
  echo ""
  echo -e "${YELLOW}Optional network diagnostics:${RESET}"
  echo "  ip addr show nymwg"
  echo "  ping -c 3 google.com"
  echo "  ping6 -c 3 google.com"
  echo ""

  if [[ -n "${LAST_BACKUP_FILE:-}" ]]; then
    echo ""
    echo -e "${GREEN}Backup of your nym-node config created at:${RESET} ${LAST_BACKUP_FILE}"
  fi

  hr
  echo -e "${GREEN}Bridge and node setup complete. Both services are ready to use.${RESET}"
  hr
  echo ""
}

graceful_exit() {
  local exit_code=$?
  END_TIME=$(date +%s)
  ELAPSED=$((END_TIME - START_TIME))

  # Only print success message when there were NO errors
  if [[ $exit_code -eq 0 ]]; then
    echo "Operation '${COMMAND}' completed."
  fi

  # END footer always logged
  echo "----- $(date '+%Y-%m-%d %H:%M:%S') END operation ${COMMAND} (status $exit_code, duration ${ELAPSED}s) -----" >> "$LOG_FILE"

  exit $exit_code
}

# Command menu
COMMAND="${1:-help}"
trap 'log "ERROR: exit=$? line=$LINENO cmd=$(printf "%q" "$BASH_COMMAND")"' ERR

trap graceful_exit EXIT

case "${1:-}" in
  full_bridge_setup)          full_bridge_setup ;;
  install_bridge_binary)      install_bridge_binary ;;
  install_bridge_cfg_tool)    install_bridge_cfg_tool ;;
  run_bridge_cfg_generate)    run_bridge_cfg_generate ;;
  create_bridge_service)      create_bridge_service ;;
  adjust_ip_forwarding)       adjust_ip_forwarding ;;
  apply_bridge_iptables_rules) apply_bridge_iptables_rules ;;
  configure_dns_and_icmp)     configure_dns_and_icmp ;;
  *)
    echo -e "\nUsage: $0 [command]\n"
    echo "Commands:"
    echo "  full_bridge_setup            - Run full setup"
    echo "  install_bridge_binary        - Install nym-bridge (.deb; falls back to source build if libc too old)"
    echo "  install_bridge_cfg_tool      - Install bridge-cfg (prebuilt; falls back to source build if libc too old)"
    echo "  run_bridge_cfg_generate      - Generate bridges.toml"
    echo "  create_bridge_service        - Setup systemd service (respects .deb-provided service)"
    echo "  adjust_ip_forwarding         - Enable forwarding"
    echo "  apply_bridge_iptables_rules  - NAT rules"
    echo "  configure_dns_and_icmp       - Allow ICMP/DNS"
    echo ""
    exit 1
    ;;
esac

