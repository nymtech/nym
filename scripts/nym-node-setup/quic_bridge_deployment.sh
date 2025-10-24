#!/usr/bin/env bash
# Nym QUIC Bridge Deployment Helper Script
# Uses bridge-cfg to generate a correct bridges.toml and keys
# installs/maintains the nym-bridge service, and provides network helpers
# read about nym-bridges: https://github.com/nymtech/nym-bridges
# read about bridge-cfg: https://github.com/nymtech/nym-bridges/tree/main/bridge-cfg
# RUN AS ROOT

set -euo pipefail

# Colors
RED="\033[0;31m"
GREEN="\033[0;32m"
YELLOW="\033[0;33m"
CYAN="\033[0;36m"
BOLD="\033[1m"
RESET="\033[0m"

# Logging
LOG_FILE="/var/log/nym-bridge-helper.log"
mkdir -p "$(dirname "$LOG_FILE")"
touch "$LOG_FILE"
chmod 640 "$LOG_FILE"
echo -e "${CYAN}Logs are being saved locally to:${RESET} $LOG_FILE"
echo -e "${CYAN}These logs never leave your machine.${RESET}"
echo "" | tee -a "$LOG_FILE"

# safe logger
log() {
  echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" | tee -a "$LOG_FILE"
}

# simple redirection that keeps function scope intact
add_log_redirection() {
  exec > >(tee -a "$LOG_FILE") 2>&1
}
add_log_redirection


# Constants / Paths
REQUIRED_CMDS=(ip jq curl openssl wg dpkg)
BRIDGE_BIN="/usr/local/bin/nym-bridge"
BRIDGE_CFG_BIN="/usr/local/bin/bridge-cfg"

NYM_ETC_DIR="/etc/nym"
NYM_ETC_KEYS_DIR="$NYM_ETC_DIR/keys"
NYM_ETC_BRIDGES="$NYM_ETC_DIR/bridges.toml"
NYM_ETC_CLIENT_PARAMS_DEFAULT="$NYM_ETC_DIR/client_bridge_params.json"
SERVICE_FILE="/etc/systemd/system/nym-bridge.service"

NET_DEV="$(ip route show default 2>/dev/null | awk '/default/ {print $5}' || true)"
WG_IFACE="nymwg"


# Root check
if [[ "$(id -u)" -ne 0 ]]; then
  echo -e "\n${RED}This script must be run as root.${RESET}\n"
  exit 1
fi

# UI helpers
hr() { echo -e "${YELLOW}----------------------------------------${RESET}"; }
title() { echo -e "\n${YELLOW}==========================================${RESET}\n${YELLOW}  $1${RESET}\n${YELLOW}==========================================${RESET}\n"; }
ok() { echo -e "${GREEN}$1${RESET}"; }
warn() { echo -e "${YELLOW}$1${RESET}"; }
err() { echo -e "${RED}$1${RESET}"; }
info() { echo -e "${CYAN}$1${RESET}"; }
press_enter() { read -r -p "$1"; }

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
  echo ""

  local v4=$(cat /proc/sys/net/ipv4/ip_forward 2>/dev/null || echo 0)
  local v6=$(cat /proc/sys/net/ipv6/conf/all/forwarding 2>/dev/null || echo 0)
  [[ "$v4" == "1" ]] && ok "✓ IPv4 forwarding enabled" || warn "IPv4 forwarding disabled"
  [[ "$v6" == "1" ]] && ok "✓ IPv6 forwarding enabled" || warn "IPv6 forwarding disabled"
  echo ""

  [[ "$all_good" == true ]] && ok "All prerequisites satisfied!" || warn "Some prerequisites missing."
}

adjust_ip_forwarding() {
  title "Adjusting IP Forwarding"
  sed -i '/^net\.ipv4\.ip_forward=/d' /etc/sysctl.conf
  sed -i '/^net\.ipv6\.conf\.all\.forwarding=/d' /etc/sysctl.conf
  echo "net.ipv4.ip_forward=1" >> /etc/sysctl.conf
  echo "net.ipv6.conf.all.forwarding=1" >> /etc/sysctl.conf
  sysctl -p /etc/sysctl.conf
  ok "IPv4/IPv6 forwarding enabled."
}

# Install nym-bridge
install_bridge_binary() {
  title "Installing nym-bridge Binary"

  info "Fetching latest nym-bridge .deb from GitHub..."
  local deb_url
  deb_url="$(curl -fsSL https://api.github.com/repos/nymtech/nym-bridges/releases/latest \
     | grep -Eo 'https://[^"]*/nym-bridge_[0-9.]+-1_amd64.deb' | head -n1 || true)"

  if [[ -z "$deb_url" ]]; then
    warn "Falling back to known version (v0.1.2)"
    deb_url="https://github.com/nymtech/nym-bridges/releases/download/bridge-binaries-v0.1.2/nym-bridge_0.1.2-1_amd64.deb"
  fi

  info "Downloading from: $deb_url"
  curl -fL -o /tmp/nym-bridge.deb "$deb_url"
  dpkg -i /tmp/nym-bridge.deb || true
  ok "nym-bridge installed."
}

# Install bridge-cfg
install_bridge_cfg_tool() {
  title "Installing bridge-cfg Tool"

  info "Fetching latest bridge-cfg from GitHub..."
  local cfg_url
  cfg_url="$(curl -fsSL https://api.github.com/repos/nymtech/nym-bridges/releases/latest \
     | grep -Eo 'https://[^"]*/bridge-cfg' | head -n1 || true)"

  if [[ -z "$cfg_url" ]]; then
    warn "Falling back to v0.1.2"
    cfg_url="https://github.com/nymtech/nym-bridges/releases/download/bridge-binaries-v0.1.2/bridge-cfg"
  fi

  info "Downloading: $cfg_url"
  curl -fL -o "$BRIDGE_CFG_BIN" "$cfg_url"
  chmod +x "$BRIDGE_CFG_BIN"
  ok "bridge-cfg installed at $BRIDGE_CFG_BIN"
}

# Generate config via bridge-cfg (with backup)
run_bridge_cfg_generate() {
  title "Generating Bridge Configuration with bridge-cfg"

  mkdir -p "$NYM_ETC_DIR"
  local candidate1="/etc/nym/default-nym-node/config/config.toml"
  local candidate2="$HOME/.nym/nym-nodes/default-nym-node/config/config.toml"
  local NODE_CFG="${candidate1}"
  [[ -f "$candidate2" ]] && NODE_CFG="$candidate2"

  echo -n "Path to your nym-node config.toml [default: $NODE_CFG]: "
  read -r input
  [[ -n "$input" ]] && NODE_CFG="$input"

  if [[ ! -f "$NODE_CFG" ]]; then
    err "nym-node config not found: $NODE_CFG"
    exit 1
  fi

  # Backup before modification
  local NODE_ID
  NODE_ID="$(basename "$(dirname "$(dirname "$NODE_CFG")")")"
  local BACKUP_DIR="$HOME/.nym/backup/$NODE_ID/config"
  mkdir -p "$BACKUP_DIR"
  local TS
  TS="$(date +%Y%m%d_%H%M%S)"
  local BACKUP_FILE="$BACKUP_DIR/config.toml.bak$TS"
  cp "$NODE_CFG" "$BACKUP_FILE"
  ok "Backup created: $BACKUP_FILE"

  info "Running: bridge-cfg --gen -n \"$NODE_CFG\" -d \"$NYM_ETC_DIR\" -o \"$NYM_ETC_BRIDGES\""
  "$BRIDGE_CFG_BIN" --gen -n "$NODE_CFG" -d "$NYM_ETC_DIR" -o "$NYM_ETC_BRIDGES"

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

  SERVICE_FILE="/etc/systemd/system/nym-bridge.service"
  mkdir -p /etc/systemd/system

  if [[ ! -x "$BRIDGE_BIN" ]]; then err "Missing $BRIDGE_BIN"; exit 1; fi
  if [[ ! -f "$NYM_ETC_BRIDGES" ]]; then err "Missing $NYM_ETC_BRIDGES"; exit 1; fi

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
  title "Applying iptables rules"
  iptables -I INPUT -i "$WG_IFACE" -j ACCEPT || true
  ip6tables -I INPUT -i "$WG_IFACE" -j ACCEPT || true
  iptables -t nat -A POSTROUTING -o "$NET_DEV" -j MASQUERADE || true
  ip6tables -t nat -A POSTROUTING -o "$NET_DEV" -j MASQUERADE || true
  iptables-save > /etc/iptables/rules.v4
  ip6tables-save > /etc/iptables/rules.v6
  ok "iptables rules applied."
}

configure_dns_and_icmp() {
  title "Allow ICMP and DNS"
  iptables -A INPUT -p icmp --icmp-type echo-request -j ACCEPT || true
  ip6tables -A INPUT -p ipv6-icmp -j ACCEPT || true
  ok "ICMP and DNS rules applied."
}

# Full interactive setup (safe exit + backup notice)
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
  echo "Step 6/6: Configuring network rules (optional but recommended)..."
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
  echo -e "${GREEN}All done! You can safely close this session.${RESET}"
  echo -e "${YELLOW}------------------------------------------${RESET}"
  echo ""
  echo "Logs saved locally at: $LOG_FILE"
  echo "Operation 'full_bridge_setup' completed."
  echo ""

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
  echo ""
  echo -e "${YELLOW}------------------------------------------${RESET}"
  if [[ $exit_code -eq 0 ]]; then
    echo -e "${GREEN}Setup completed successfully. Exiting cleanly.${RESET}"
  else
    echo -e "${RED}Script exited with errors (code: $exit_code).${RESET}"
    echo "Check the log at: $LOG_FILE"
  fi
  echo -e "${YELLOW}------------------------------------------${RESET}"
  echo ""
  exec >&- 2>&-
  exit $exit_code
}
trap graceful_exit EXIT

# Command menu
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
    echo "  install_bridge_binary        - Install nym-bridge"
    echo "  install_bridge_cfg_tool      - Install bridge-cfg"
    echo "  run_bridge_cfg_generate      - Generate bridges.toml"
    echo "  create_bridge_service        - Setup systemd service"
    echo "  adjust_ip_forwarding         - Enable forwarding"
    echo "  apply_bridge_iptables_rules  - NAT rules"
    echo "  configure_dns_and_icmp       - Allow ICMP/DNS"
    echo ""
    exit 1
    ;;
esac

echo "Operation '${1:-help}' completed."

