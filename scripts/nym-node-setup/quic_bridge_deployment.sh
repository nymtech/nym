#!/usr/bin/env bash
# Nym Bridge Deployment & Upgrade Helper
#
# One script for BOTH new and existing operators:
#   - fresh host            -> installs nym-bridge, generates bridges.toml (QUIC + optional TLS/SSH)
#   - existing QUIC bridge   -> upgrades the binary and ADDS the TLS/SSH entry transports,
#                               never touching the published QUIC identity key
#
# Design notes (0.3.0+):
#   - nym-bridge 0.3.0 adds two TCP entry transports, TLS (tcp/4443) and SSH (tcp/4422),
#     that NymVPN falls back to when QUIC/UDP is blocked. QUIC (udp/4443) is unchanged.
#   - Binaries come from the GitHub release .deb ONLY. There is no source build / no cargo.
#     An unsupported host is a hard stop, not a compile.
#   - This script makes NO firewall changes at all (no ufw/firewalld/iptables, and it does NOT
#     run NTM). Host ports are owned by the Network Tunnel Manager (NTM), run as a separate step.
#     At the end this script PRINTS the exact NTM command sequence to open the bridge ports.
#   - Everything bridge-related is self-contained here: it does not call the nym-bridges repo
#     scripts. The only artifact fetched from that repo is the release .deb (the binary).
#
# read about nym-bridges: https://github.com/nymtech/nym-bridges
# read about bridge-cfg:   https://github.com/nymtech/nym-bridges/tree/main/bridge-cfg
# RUN AS ROOT

set -Eeuo pipefail

# --------------------------------------------------------------------------------------
# Colors / logging
# --------------------------------------------------------------------------------------
RED="\033[0;31m"; GREEN="\033[0;32m"; YELLOW="\033[0;33m"; CYAN="\033[0;36m"; RESET="\033[0m"

LOG_FILE="/var/log/nym/quic_bridge_deployment.log"
mkdir -p "$(dirname "$LOG_FILE")"
if [[ -f "$LOG_FILE" && $(stat -c%s "$LOG_FILE") -gt 10485760 ]]; then
  mv "$LOG_FILE" "${LOG_FILE}.1"
fi
touch "$LOG_FILE"; chmod 640 "$LOG_FILE"

log() { echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" | tee -a "$LOG_FILE"; }

add_log_redirection() {
  exec > >(tee >(sed -u 's/\x1b\[[0-9;]*m//g' >> "$LOG_FILE"))
  exec 2> >(tee >(sed -u 's/\x1b\[[0-9;]*m//g' >> "$LOG_FILE") >&2)
}
add_log_redirection

echo "----- $(date '+%Y-%m-%d %H:%M:%S') START nym-bridge-manager -----"
echo -e "${CYAN}Logs are being saved locally to:${RESET} $LOG_FILE"
echo -e "${CYAN}These logs never leave your machine.${RESET}\n"

START_TIME=$(date +%s)

hr()    { echo -e "${YELLOW}----------------------------------------${RESET}"; }
title() { echo -e "\n${YELLOW}==========================================${RESET}\n${YELLOW}  $1${RESET}\n${YELLOW}==========================================${RESET}\n"; }
ok()    { echo -e "${GREEN}$1${RESET}"; }
warn()  { echo -e "${YELLOW}$1${RESET}"; }
err()   { echo -e "${RED}$1${RESET}"; }
info()  { echo -e "${CYAN}$1${RESET}"; }

# --------------------------------------------------------------------------------------
# Tunables (env-overridable; flags below win)
# --------------------------------------------------------------------------------------
REPO="nymtech/nym-bridges"
# Branch of nymtech/nym used to fetch the Network Tunnel Manager, if it isn't already present.
BRANCH="${BRANCH:-develop}"
# Release tag to install. Bump this when a new stable is cut; pre-releases work too.
# For rc testing pass --tag v0.3.0-rc.1 or export BRIDGE_TAG=v0.3.0-rc.1
BRIDGE_TAG="${BRIDGE_TAG:-v0.3.0}"
# Add the TLS + SSH entry transports (1) or keep QUIC only (0, via --no-tls-ssh)
ADD_TLS_SSH="${ADD_TLS_SSH:-1}"

# Entry transports introduced in 0.3.0: "<transport_type>:<tcp_port>"
NEW_TRANSPORTS=("tls_plain:4443" "ssh_plain:4422")

# Paths
NYM_ETC_DIR="/etc/nym"
NYM_ETC_KEYS_DIR="$NYM_ETC_DIR/keys"
CONF="$NYM_ETC_DIR/bridges.toml"
CLIENT_PARAMS_DEFAULT="$NYM_ETC_DIR/client_bridge_params.json"
SERVICE="nym-bridge.service"
SERVICE_FILE="/etc/systemd/system/$SERVICE"
BRIDGE_BIN="/usr/bin/nym-bridge"      # resolved after install
BRIDGE_CFG_BIN="/usr/bin/bridge-cfg"  # resolved after install

REQUIRED_CMDS=(curl jq dpkg systemctl ss awk ip openssl)

# Runtime state
MODE=""                 # fresh | upgrade
NODE_CFG="${NODE_CFG:-}"
BAK=""                  # /etc/nym backup dir (upgrade only)
QUIC_BEFORE=""
LAST_BACKUP_FILE=""
SUCCESS=0

# --------------------------------------------------------------------------------------
# Root + arg parsing (keeps backward-compatible `full_bridge_setup` subcommand)
# --------------------------------------------------------------------------------------
if [[ "$(id -u)" -ne 0 ]]; then err "\nThis script must be run as root.\n"; exit 1; fi

COMMAND=""
for arg in "$@"; do
  case "$arg" in
    --no-tls-ssh)      ADD_TLS_SSH=0 ;;
    --tls-ssh)         ADD_TLS_SSH=1 ;;
    --tag=*)           BRIDGE_TAG="${arg#*=}" ;;
    --tag)             err "--tag needs a value, e.g. --tag v0.3.0-rc.1"; exit 1 ;;
    v[0-9]*)           BRIDGE_TAG="$arg" ;;   # bare tag, matches orchestrator/PR convention
    -h|--help|help)    COMMAND="help" ;;
    --*)               err "unknown option: $arg"; exit 1 ;;
    *)                 COMMAND="${COMMAND:-$arg}" ;;
  esac
done
COMMAND="${COMMAND:-full_bridge_setup}"

# --------------------------------------------------------------------------------------
# Preflight: binary-only means an unsupported host is a hard stop (no compile fallback)
# --------------------------------------------------------------------------------------
verify_bridge_prerequisites() {
  title "Checking prerequisites"
  local missing=0 c
  for c in "${REQUIRED_CMDS[@]}"; do
    if ! command -v "$c" >/dev/null 2>&1; then err "missing required command: $c"; missing=1; fi
  done
  [[ "$missing" -eq 0 ]] || { err "install the missing commands and re-run (Debian/Ubuntu: apt-get install -y curl jq iproute2 gawk openssl)"; exit 1; }

  [[ "$(dpkg --print-architecture)" == "amd64" ]] || { err "the release .deb is amd64 only; this host is $(dpkg --print-architecture)"; exit 1; }

  local libc
  libc="$(dpkg-query -W -f='${Version}' libc6 2>/dev/null || true)"
  if ! dpkg --compare-versions "${libc:-0}" ge 2.34; then
    err "nym-bridge ${BRIDGE_TAG} needs glibc >= 2.34 (Ubuntu 22.04+, Debian 12+); found ${libc:-none}."
    err "This script installs the prebuilt binary only and will not compile from source. Upgrade the host."
    exit 1
  fi
  ok "host is amd64 with glibc ${libc} (>= 2.34)"
}

resolve_node_cfg() {
  if [[ -z "$NODE_CFG" ]]; then
    local home_dir="${HOME:-/root}"
    NODE_CFG="$(find "$home_dir/.nym/nym-nodes" -type f -name config.toml 2>/dev/null | head -n1 || true)"
    [[ -z "$NODE_CFG" ]] && NODE_CFG="$home_dir/.nym/nym-nodes/default-nym-node/config/config.toml"
  fi
  if [[ "${NONINTERACTIVE:-0}" != "1" ]]; then
    echo -n "Path to your nym-node config.toml [default: $NODE_CFG]: "
    read -r input || true
    [[ -n "${input:-}" ]] && NODE_CFG="$input"
  fi
  [[ -f "$NODE_CFG" ]] || { err "nym-node config not found: $NODE_CFG (set NODE_CFG=/path/to/config.toml)"; exit 1; }
  info "using nym-node config: $NODE_CFG"
}

detect_mode() {
  if [[ -f "$CONF" ]]; then
    MODE="upgrade"
    info "existing bridge detected ($CONF) -> UPGRADE mode (QUIC key will be preserved)"
  else
    MODE="fresh"
    info "no existing bridge config -> FRESH install"
  fi
}

# --------------------------------------------------------------------------------------
# Rollback (armed only once a backup exists, i.e. upgrade mode)
# --------------------------------------------------------------------------------------
rollback() {
  [[ "${SUCCESS:-0}" -eq 1 ]] && return 0
  [[ -z "$BAK" || ! -d "$BAK" ]] && return 0
  err "FAILED: restoring $BAK and restarting $SERVICE"
  cp -a "$BAK"/. "$NYM_ETC_DIR"/ 2>/dev/null || true
  local b
  for b in nym-bridge bridge-cfg; do
    [[ -e "/usr/local/bin/$b.old" ]] && mv -f "/usr/local/bin/$b.old" "/usr/local/bin/$b" 2>/dev/null || true
  done
  systemctl restart "$SERVICE" 2>/dev/null || true
}

# --------------------------------------------------------------------------------------
# Binary install (release .deb only; keeps any /usr/local/bin path working via symlink)
# --------------------------------------------------------------------------------------
resolve_deb_url() {
  local tag="$1" url="" tries=0
  while [[ -z "$url" && "$tries" -lt 3 ]]; do
    url="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/tags/$tag" 2>/dev/null \
            | jq -r '.assets[]?|select(.name|endswith(".deb"))|.browser_download_url' | head -n1 || true)"
    [[ -n "$url" && "$url" != "null" ]] && break
    tries=$((tries+1)); sleep 2
  done
  [[ -n "$url" && "$url" != "null" ]] || { err "no .deb asset on release $tag for $REPO (GitHub API rate limit? try later or set BRIDGE_TAG)"; return 1; }
  printf '%s' "$url"
}

install_bridge_binary() {
  title "Installing nym-bridge ${BRIDGE_TAG} (.deb)"
  local url tmp
  url="$(resolve_deb_url "$BRIDGE_TAG")" || exit 1
  info "downloading: $url"
  tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' RETURN
  curl -fL -o "$tmp/nym-bridge.deb" "$url"

  # An older install may run the service from /usr/local/bin. Preserve that path on the new binary.
  local b
  for b in nym-bridge bridge-cfg; do
    if [[ -e "/usr/local/bin/$b" && ! -L "/usr/local/bin/$b" ]]; then
      mv "/usr/local/bin/$b" "/usr/local/bin/$b.old"
    fi
  done

  dpkg -i "$tmp/nym-bridge.deb"

  for b in nym-bridge bridge-cfg; do
    [[ -e "/usr/local/bin/$b.old" ]] && ln -sf "/usr/bin/$b" "/usr/local/bin/$b"
  done

  [[ -x /usr/bin/nym-bridge ]] && BRIDGE_BIN="/usr/bin/nym-bridge"
  [[ -x /usr/bin/bridge-cfg ]] && BRIDGE_CFG_BIN="/usr/bin/bridge-cfg"
  command -v "$BRIDGE_BIN" >/dev/null || { err "nym-bridge not found after install"; exit 1; }
  command -v "$BRIDGE_CFG_BIN" >/dev/null || { err "bridge-cfg not found after install"; exit 1; }

  mkdir -p /var/log/nym
  echo "[Bridge Install] $(date '+%F %T') $("$BRIDGE_BIN" --version 2>/dev/null || echo 'nym-bridge (unknown)') tag=$BRIDGE_TAG" \
    >> /var/log/nym/nym-bridge-version.log
  ok "nym-bridge installed from .deb (${BRIDGE_TAG})"
}

# --------------------------------------------------------------------------------------
# Config: bridges.toml + keys  (bridge-cfg is the only thing that mints keys)
#   fresh   -> bridge-cfg --gen            (creates QUIC)
#   add T/S -> append blocks, then bridge-cfg --gen -i   (keeps QUIC key, mints only new ones)
# --------------------------------------------------------------------------------------
client_params_path() {
  local p=""
  [[ -f "$CONF" ]] && p="$(awk -F'"' '/^[[:space:]]*client_params_path/ {print $2}' "$CONF" | head -n1 || true)"
  printf '%s' "${p:-$CLIENT_PARAMS_DEFAULT}"
}

quic_key() {
  local params; params="$(client_params_path)"
  [[ -f "$params" ]] || { printf ''; return 0; }
  jq -r '.transports[]?|select(.transport_type=="quic_plain").args.id_pubkey // empty' "$params" 2>/dev/null | head -n1 || true
}

# prints "<transport_type> <port>" for every transport in bridges.toml
list_ports() {
  awk -F"[\"']" '/^\[\[transports\]\]/ {t=""} /^[[:space:]]*transport_type[[:space:]]*=/ {t=$2} /^[[:space:]]*listen[[:space:]]*=/ && t != "" {n=split($2,a,":"); print t, a[n]}' "$CONF"
}

harden_perms() {
  mkdir -p "$NYM_ETC_DIR" "$NYM_ETC_KEYS_DIR"
  chgrp nym "$NYM_ETC_DIR" 2>/dev/null || true
  chmod 750 "$NYM_ETC_DIR"; chmod 700 "$NYM_ETC_KEYS_DIR"
  [[ -f "$CONF" ]] && chmod 600 "$CONF"
  chmod 600 "$NYM_ETC_KEYS_DIR"/* 2>/dev/null || true
}

generate_base_config() {
  title "Generating bridges.toml (QUIC base)"
  harden_perms
  touch "$CLIENT_PARAMS_DEFAULT" 2>/dev/null || true
  info "running: bridge-cfg --gen -n \"$NODE_CFG\" -d \"$NYM_ETC_DIR\" -o \"$CONF\""
  "$BRIDGE_CFG_BIN" --gen -n "$NODE_CFG" -d "$NYM_ETC_DIR" -o "$CONF"
  harden_perms
  ok "base QUIC bridges.toml generated at $CONF"
}

add_entry_transports() {
  [[ "$ADD_TLS_SSH" -eq 1 ]] || { info "--no-tls-ssh: keeping QUIC only, not adding TLS/SSH"; return 0; }
  title "Adding TLS/SSH entry transports"

  local spec name port added=0
  for spec in "${NEW_TRANSPORTS[@]}"; do
    name="${spec%%:*}"; port="${spec##*:}"
    if grep -Eq "^[[:space:]]*transport_type[[:space:]]*=[[:space:]]*\"$name\"" "$CONF"; then
      info "$name already present in bridges.toml, leaving as is"
      continue
    fi
    if ss -ltn "sport = :$port" 2>/dev/null | grep -q LISTEN; then
      warn "TCP $port already in use on this host; NOT adding $name"
      continue
    fi
    printf '\n[[transports]]\ntransport_type = "%s"\n[transports.args]\nlisten = "[::]:%s"\n' "$name" "$port" >> "$CONF"
    ok "appended $name on tcp/$port"
    added=1
  done

  # Mint keys for any transport that lacks one; -i preserves the existing QUIC key.
  info "running: bridge-cfg --gen -i \"$CONF\" -n \"$NODE_CFG\" -d \"$NYM_ETC_DIR\" -o \"$CONF\""
  "$BRIDGE_CFG_BIN" --gen -i "$CONF" -n "$NODE_CFG" -d "$NYM_ETC_DIR" -o "$CONF"
  harden_perms
  [[ "$added" -eq 1 ]] && ok "entry transports added" || info "no new transports needed"
}

verify_quic_key() {
  local after; after="$(quic_key)"
  if [[ "$MODE" == "upgrade" ]]; then
    [[ -n "$QUIC_BEFORE" ]] || { err "could not read QUIC key before upgrade"; return 1; }
    [[ "$QUIC_BEFORE" == "$after" ]] || { err "QUIC identity key changed ($QUIC_BEFORE -> $after). This would break existing users."; return 1; }
    ok "QUIC identity key unchanged"
  else
    [[ -n "$after" ]] || { err "no QUIC key present after generation"; return 1; }
    ok "QUIC identity key established"
  fi
}

# --------------------------------------------------------------------------------------
# systemd service (respect a .deb-provided unit; otherwise create one)
# --------------------------------------------------------------------------------------
create_bridge_service() {
  title "Configuring $SERVICE"
  if systemctl list-unit-files 2>/dev/null | grep -q "^$SERVICE"; then
    warn "existing $SERVICE detected (package-provided); not overwriting"
    systemctl daemon-reload || true
    systemctl enable "$SERVICE" >/dev/null 2>&1 || true
    ok "service managed by package"
    return 0
  fi
  [[ -x "$BRIDGE_BIN" ]] || { err "missing $BRIDGE_BIN"; exit 1; }
  [[ -f "$CONF" ]]       || { err "missing $CONF"; exit 1; }

  cat > "$SERVICE_FILE" <<EOF
[Unit]
Description=Nym Bridge (QUIC/TLS/SSH entry)
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=root
ExecStart=$BRIDGE_BIN --config $CONF
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
  systemctl enable "$SERVICE" >/dev/null 2>&1 || true
  ok "$SERVICE created and enabled"
}

# --------------------------------------------------------------------------------------
# Firewall: NONE here. Host ports are owned by the Network Tunnel Manager (NTM), which the
# operator runs as a separate step. We only PRINT the exact command sequence to do that.
# --------------------------------------------------------------------------------------
print_ntm_recommendation() {
  local t port proto
  echo
  hr
  info "Host ports are managed by the Network Tunnel Manager (NTM) — a SEPARATE step."
  echo  "This script changed no firewall rules. Open the bridge ports by running NTM yourself:"
  echo
  echo  "  curl -fsSL https://raw.githubusercontent.com/nymtech/nym/refs/heads/${BRANCH}/scripts/nym-node-setup/network-tunnel-manager.sh -o network-tunnel-manager.sh"
  echo  "  chmod +x network-tunnel-manager.sh"
  echo  "  bash network-tunnel-manager.sh configure_network_firewall"
  echo
  echo  "NTM's TCP allow-list must include 4443 (TLS) and 4422 (SSH); UDP 4443 (QUIC) is already covered."
  echo  "Ports this bridge listens on (open these inbound):"
  while read -r t port; do
    [[ -n "$port" ]] || continue
    proto=TCP; [[ "$t" == quic_plain ]] && proto=UDP
    echo "  $proto $port  ($t)"
  done < <(list_ports)
  hr
}

# --------------------------------------------------------------------------------------
# Restart + verify (service active, every configured port listening, QUIC key stable)
# --------------------------------------------------------------------------------------
restart_and_verify() {
  title "Restarting and verifying $SERVICE"
  systemctl restart "$SERVICE"

  local missing="" _ t port flag listening
  for _ in $(seq 15); do
    sleep 2
    missing=""
    systemctl is-active --quiet "$SERVICE" || missing="$SERVICE is not active"
    while read -r t port; do
      [[ -n "$port" ]] || continue
      flag=t; [[ "$t" == quic_plain ]] && flag=u
      listening="$(ss -ln"$flag"p 2>/dev/null || true)"
      grep -q ":$port " <<< "$listening" || missing="not listening on $t port $port"
    done < <(list_ports)
    [[ -z "$missing" ]] && break
  done
  [[ -z "$missing" ]] || { err "$missing"; return 1; }

  verify_quic_key || return 1
  ok "service active and listening on all configured ports"
}

# --------------------------------------------------------------------------------------
# Orchestration
# --------------------------------------------------------------------------------------
full_bridge_setup() {
  title "Nym Bridge - setup / upgrade"

  verify_bridge_prerequisites
  resolve_node_cfg
  detect_mode

  if [[ "$MODE" == "upgrade" ]]; then
    QUIC_BEFORE="$(quic_key)"
    [[ -n "$QUIC_BEFORE" ]] || { err "cannot read current QUIC key from $(client_params_path); aborting before any change"; exit 1; }
    info "QUIC key before: $QUIC_BEFORE"
    BAK="/etc/nym.bak-$(date +%F-%H%M%S)"
    cp -a "$NYM_ETC_DIR" "$BAK"
    LAST_BACKUP_FILE="$BAK"
    ok "backed up $NYM_ETC_DIR -> $BAK"
    trap 'rollback' ERR
    trap 'rollback; exit 1' INT TERM HUP
  fi

  install_bridge_binary

  if [[ "$MODE" == "fresh" ]]; then
    generate_base_config
    add_entry_transports
  else
    # upgrade: binary is new; only add transports if requested, preserving QUIC key
    add_entry_transports
  fi

  verify_quic_key
  create_bridge_service
  restart_and_verify

  SUCCESS=1
  trap - ERR INT TERM HUP
  # rollback copies are no longer needed
  rm -f /usr/local/bin/nym-bridge.old /usr/local/bin/bridge-cfg.old 2>/dev/null || true

  finalize_summary
}

finalize_summary() {
  title "Done"
  local params; params="$(client_params_path)"

  if systemctl --quiet is-active "$SERVICE"; then ok "$SERVICE is running."; else warn "$SERVICE is not active; check: journalctl -u $SERVICE -n 50 --no-pager"; fi

  echo; info "Published transports (new apps read transports_v2):"
  jq -r '.transports_v2[]? | "  \(.transport_type)  \(.args.addresses[0] // "")"' "$params" 2>/dev/null || true

  # Recommended NEXT step (not run here): open the ports with NTM.
  print_ntm_recommendation

  echo; warn "Also allow the SAME ports inbound (IPv4 + IPv6) in your cloud provider's firewall /"
  warn "security group. That is outside this host and cannot be changed from here."
  echo "  Then, from ANOTHER machine:  nc -vz <your public IP> 4443   &&   nc -vz <your public IP> 4422"
  [[ -n "$LAST_BACKUP_FILE" ]] && echo -e "\n${GREEN}Backup kept at:${RESET} $LAST_BACKUP_FILE"

  echo; hr
  echo "Verify:  systemctl status $SERVICE --no-pager   |   journalctl -u $SERVICE -n 30 --no-pager"
  echo "         ss -lntup | grep -E ':4443|:4422'"
  hr
}

print_help() {
  cat <<EOF

Usage: $0 [full_bridge_setup] [TAG] [--no-tls-ssh]

Runs setup on a fresh host and in-place upgrade on an existing bridge (auto-detected).
Installs the release .deb (no source build). Makes NO firewall changes: it prints the exact
Network Tunnel Manager (NTM) command sequence for you to open the ports as a separate step.

Arguments / flags:
  full_bridge_setup     default action (setup or upgrade)
  vX.Y.Z[-rc.N]         release tag to install (default: $BRIDGE_TAG); e.g. v0.3.0-rc.1
  --tag <tag>           same as above
  --no-tls-ssh          upgrade/install binaries + QUIC only; do not add TLS/SSH

Environment:
  BRIDGE_TAG            release tag (overridden by a tag argument/--tag)
  NODE_CFG             path to nym-node config.toml (skips autodetect/prompt)
  NONINTERACTIVE=1      no prompts
  BRANCH               nymtech/nym branch used in the printed NTM command (default: $BRANCH)

Safety: the published QUIC identity key is never regenerated; on any failure during an
upgrade the previous /etc/nym and binaries are restored and the service is restarted.
EOF
}

graceful_exit() {
  local rc=$?
  local elapsed=$(( $(date +%s) - START_TIME ))
  [[ $rc -eq 0 ]] && echo "Operation '${COMMAND}' completed."
  echo "----- $(date '+%Y-%m-%d %H:%M:%S') END operation ${COMMAND} (status $rc, duration ${elapsed}s) -----" >> "$LOG_FILE"
  exit $rc
}
trap 'log "ERROR: exit=$? line=$LINENO cmd=$(printf "%q" "$BASH_COMMAND")"' ERR
trap graceful_exit EXIT

case "$COMMAND" in
  full_bridge_setup)   full_bridge_setup ;;
  help)                print_help ;;
  *)                   err "unknown command: $COMMAND"; print_help; exit 1 ;;
esac