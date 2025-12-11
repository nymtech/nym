#!/bin/bash
# nym tunnel and wireguard exit policy manager
# run this script as root

set -euo pipefail
set +o errtrace


###############################################################################
# colors (no emojis)
###############################################################################
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m'
CYAN='\033[0;36m'
RESET='\033[0m'

info() {
  printf "%b\n" "${YELLOW}[INFO] $*${NC}"
}

ok() {
  printf "%b\n" "${GREEN}[OK] $*${NC}"
}

error() {
  printf "%b\n" "${RED}[ERROR] $*${NC}"
}

###############################################################################
# safety: must run as root, jq
###############################################################################
if [ "$(id -u)" -ne 0 ]; then
   error "This script must be run as root"
  exit 1
fi

###############################################################################
# Logging
###############################################################################
LOG_FILE="/var/log/nym/network_tunnel_manager.log"
mkdir -p "$(dirname "$LOG_FILE")"
touch "$LOG_FILE"
chmod 640 "$LOG_FILE"

# rotate log if >10MB
if [[ -f "$LOG_FILE" && $(stat -c%s "$LOG_FILE") -gt 10485760 ]]; then
  mv "$LOG_FILE" "${LOG_FILE}.1"
  touch "$LOG_FILE"
  chmod 640 "$LOG_FILE"
fi

echo "----- $(date '+%Y-%m-%d %H:%M:%S') START network-tunnel-manager -----" | tee -a "$LOG_FILE"
echo -e "${CYAN}Logs are being saved locally to:${RESET} $LOG_FILE"
echo -e "${CYAN}These logs never leave your machine.${RESET}"
echo "" | tee -a "$LOG_FILE"

# safe logger
log() {
  echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" | tee -a "$LOG_FILE"
}

# global redirection, strip ANSI before writing to log
add_log_redirection() {
  exec > >(tee >(sed -u 's/\x1b\[[0-9;]*m//g' >> "$LOG_FILE"))
  exec 2> >(tee >(sed -u 's/\x1b\[[0-9;]*m//g' >> "$LOG_FILE") >&2)
}
add_log_redirection


trap 'log "ERROR: exit=$? line=$LINENO cmd=$(printf "%q" "$BASH_COMMAND")"' ERR




START_TIME=$(date +%s)

###############################################################################
# basic config
###############################################################################

NYM_CHAIN="NYM-EXIT"
POLICY_FILE="/etc/nym/exit-policy.txt"
EXIT_POLICY_LOCATION="https://nymtech.net/.wellknown/network-requester/exit-policy.txt"

TUNNEL_INTERFACE="${TUNNEL_INTERFACE:-nymtun0}"
WG_INTERFACE="${WG_INTERFACE:-nymwg}"

# Function to detect and validate uplink interface
detect_uplink_interface() {
  local cmd="$1"
  local dev

  dev="$(eval "$cmd" 2>/dev/null | awk '{print $5}' | head -n1 || true)"

  if [[ -n "$dev" && "$dev" =~ ^[a-zA-Z0-9._-]+$ ]]; then
    echo "$dev"
  else
    echo ""
  fi
}

# uplink device detection, can be overridden
NETWORK_DEVICE="${NETWORK_DEVICE:-}"
if [[ -z "$NETWORK_DEVICE" ]]; then
  NETWORK_DEVICE="$(detect_uplink_interface "ip -o route show default")"
fi
if [[ -z "$NETWORK_DEVICE" ]]; then
  NETWORK_DEVICE="$(detect_uplink_interface "ip -o route show default table all")"
fi
if [[ -z "$NETWORK_DEVICE" ]]; then
  error "cannot determine uplink interface. set NETWORK_DEVICE or UPLINK_DEV"
  exit 1
fi

###############################################################################
# shared helpers
###############################################################################

ensure_jq() {
  info "checking for jq..."

  if command -v jq >/dev/null 2>&1; then
    ok "jq is already installed"
  else
    info "jq not found, installing..."
    apt-get update -y
    DEBIAN_FRONTEND=noninteractive apt-get install -y jq

    if command -v jq >/dev/null 2>&1; then
      ok "jq installed successfully"
    else
      error "failed to install jq"
      exit 1
    fi
  fi
}

install_iptables_persistent() {
  if ! dpkg -s iptables-persistent >/dev/null 2>&1; then
    info "installing iptables-persistent"
    apt-get update -y
    DEBIAN_FRONTEND=noninteractive apt-get install -y iptables-persistent
  else
    ok "iptables-persistent is already installed"
  fi
}

adjust_ip_forwarding() {
  info "configuring ip forwarding via /etc/sysctl.d/99-nym-forwarding.conf"
  install -m 0644 /dev/null /etc/sysctl.d/99-nym-forwarding.conf
  cat > /etc/sysctl.d/99-nym-forwarding.conf <<EOF
net.ipv4.ip_forward=1
net.ipv6.conf.all.forwarding=1
EOF
  sysctl --system

  local v4 v6
  v4=$(cat /proc/sys/net/ipv4/ip_forward 2>/dev/null || echo 0)
  v6=$(cat /proc/sys/net/ipv6/conf/all/forwarding 2>/dev/null || echo 0)

  if [[ "$v4" == "1" && "$v6" == "1" ]]; then
    ok "ipv4 and ipv6 forwarding enabled"
  else
    error "warning: ip forwarding not fully enabled (ipv4=$v4 ipv6=$v6)"
  fi
}

save_iptables_rules() {
  info "saving iptables rules to /etc/iptables$"
  mkdir -p /etc/iptables
  iptables-save > /etc/iptables/rules.v4
  ip6tables-save > /etc/iptables/rules.v6
  ok "iptables rules saved"
}

###############################################################################
# part 1: network tunnel manager (nymtun0 + nymwg base nat/forwarding)
###############################################################################

fetch_ipv6_address() {
  local interface=$1
  local ipv6_global_address
  ipv6_global_address=$(ip -6 addr show "$interface" scope global | awk '/inet6/ {print $2}' | head -n 1)

  if [[ -z "$ipv6_global_address" ]]; then
    error "no globally routable ipv6 address found on $interface. please configure ipv6 or check your network settings"
    exit 1
  else
    info "using ipv6 address: $ipv6_global_address"
  fi
}

fetch_and_display_ipv6() {
  local ipv6_address
  ipv6_address=$(ip -6 addr show "$NETWORK_DEVICE" scope global | awk '/inet6/ {print $2}')
  if [[ -z "$ipv6_address" ]]; then
    error "no global ipv6 address found on $NETWORK_DEVICE"
  else
    ok "ipv6 address on $NETWORK_DEVICE: $ipv6_address"
  fi
}

# dedupe / clean-up rules for an interface in FORWARD and NYM-EXIT
# keeps a single copy of each rule

remove_duplicate_rules() {
  local interface="$1"

  if [[ -z "$interface" ]]; then
    error "Error: No interface specified. Usage: $0 remove_duplicate_rules <interface>"
    exit 1
  fi

  info "detecting and removing duplicate rules for $interface in FORWARD and ${NYM_CHAIN}"

  #
  # ipv4
  #
  local rules_v4
  rules_v4=$(iptables-save | grep -E "(-A FORWARD|-A $NYM_CHAIN)" | grep -F -- "$interface" || true)

  if [[ -n "$rules_v4" ]]; then
    info "processing ipv4 rules"

    local tmp4
    tmp4=$(mktemp)
    printf "%s\n" "$rules_v4" | sort | uniq > "$tmp4"

    local rule count cleaned chain rest match index
    while IFS= read -r rule; do
      [[ -z "$rule" ]] && continue

      # FIX: protect grep from rule content becoming flags
      count=$(printf "%s\n" "$rules_v4" | grep -F -- "$rule" | wc -l)

      if [[ "$count" -gt 1 ]]; then
        info "removing $((count - 1)) duplicate(s) of ipv4 rule: $rule"

        for ((i=1; i<count; i++)); do
          cleaned="${rule#-A }"
          chain=$(echo "$cleaned" | awk '{print $1}')
          rest=$(echo "$cleaned" | cut -d' ' -f2-)

          read -ra RULE_ARR <<<"$rest"

          if iptables -t filter -C "$chain" "${RULE_ARR[@]}" 2>/dev/null; then
            iptables -t filter -D "$chain" "${RULE_ARR[@]}" && continue
          fi

          match=$(iptables -S | grep -F -- "$cleaned" | head -n1 || true)

          if [[ -n "$match" ]]; then
            chain=$(echo "$match" | awk '{print $2}')
            index=$(iptables -L "$chain" --line-numbers | grep -F "$interface" | awk 'NR==1{print $1}')

            if [[ -n "$index" ]]; then
              iptables -D "$chain" "$index" 2>/dev/null || \
                error "warning: failed deleting ipv4 duplicate via index ($chain $index)"
            else
              error "warning: unable to locate ipv4 duplicate index for: $rule"
            fi
          else
            error "warning: could not reliably match ipv4 duplicate rule: $rule"
          fi
        done
      fi

    done < "$tmp4"

    rm -f "$tmp4"

  else
    ok "no ipv4 rules found for $interface to deduplicate"
  fi



  #
  # ipv6
  #
  local rules_v6
  rules_v6=$(ip6tables-save | grep -E "(-A FORWARD|-A $NYM_CHAIN)" | grep -F -- "$interface" || true)

  if [[ -n "$rules_v6" ]]; then
    info "processing ipv6 rules"

    local tmp6
    tmp6=$(mktemp)
    printf "%s\n" "$rules_v6" | sort | uniq > "$tmp6"

    local rule count cleaned chain rule_spec match index
    while IFS= read -r rule; do
      [[ -z "$rule" ]] && continue

      # FIX: protect grep from interpreting rule as flags
      count=$(printf "%s\n" "$rules_v6" | grep -F -- "$rule" | wc -l)

      if [[ "$count" -gt 1 ]]; then
        info "removing $((count - 1)) duplicate(s) of ipv6 rule: $rule"

        for ((i=1; i<count; i++)); do
          cleaned="${rule#-A }"
          chain="${cleaned%% *}"
          rule_spec="${cleaned#"$chain" }"

          read -ra RULE6_ARR <<<"$rule_spec"

          if ip6tables -t filter -C "$chain" "${RULE6_ARR[@]}" 2>/dev/null; then
            ip6tables -t filter -D "$chain" "${RULE6_ARR[@]}" && continue
          fi

          match=$(ip6tables -S | grep -F -- "$cleaned" | head -n1 || true)

          if [[ -n "$match" ]]; then
            chain=$(echo "$match" | awk '{print $2}')

            index=$(ip6tables -L "$chain" --line-numbers | grep -F "$interface" | awk 'NR==1{print $1}')

            if [[ -n "$index" ]]; then
              ip6tables -D "$chain" "$index" 2>/dev/null || \
                error "warning: failed deleting ipv6 duplicate via index ($chain $index)"
            else
              error "warning: unable to locate ipv6 duplicate index for: $rule"
            fi
          else
            error "warning: could not match ipv6 duplicate rule reliably: $rule"
          fi
        done
      fi

    done < "$tmp6"

    rm -f "$tmp6"

  else
    ok "no ipv6 rules found for $interface to deduplicate"
  fi

  ok "duplicate rule scan completed for $interface"
}

apply_iptables_rules() {
  local interface=$1
  info "applying iptables rules for $interface using uplink $NETWORK_DEVICE"
  sleep 1

  # ipv4 nat and forwarding
  iptables -t nat -C POSTROUTING -o "$NETWORK_DEVICE" -j MASQUERADE 2>/dev/null || \
    iptables -t nat -A POSTROUTING -o "$NETWORK_DEVICE" -j MASQUERADE

  iptables -C FORWARD -i "$interface" -o "$NETWORK_DEVICE" -j ACCEPT 2>/dev/null || \
    iptables -I FORWARD 1 -i "$interface" -o "$NETWORK_DEVICE" -j ACCEPT

  iptables -C FORWARD -i "$NETWORK_DEVICE" -o "$interface" -m state --state RELATED,ESTABLISHED -j ACCEPT 2>/dev/null || \
    iptables -I FORWARD 2 -i "$NETWORK_DEVICE" -o "$interface" -m state --state RELATED,ESTABLISHED -j ACCEPT

  # ipv6 nat and forwarding
  ip6tables -t nat -C POSTROUTING -o "$NETWORK_DEVICE" -j MASQUERADE 2>/dev/null || \
    ip6tables -t nat -A POSTROUTING -o "$NETWORK_DEVICE" -j MASQUERADE

  ip6tables -C FORWARD -i "$interface" -o "$NETWORK_DEVICE" -j ACCEPT 2>/dev/null || \
    ip6tables -I FORWARD 1 -i "$interface" -o "$NETWORK_DEVICE" -j ACCEPT

  ip6tables -C FORWARD -i "$NETWORK_DEVICE" -o "$interface" -m state --state RELATED,ESTABLISHED -j ACCEPT 2>/dev/null || \
    ip6tables -I FORWARD 2 -i "$NETWORK_DEVICE" -o "$interface" -m state --state RELATED,ESTABLISHED -j ACCEPT

  save_iptables_rules
}

check_tunnel_iptables() {
  local interface=$1
  info "inspecting iptables rules for $interface"
  info "ipv4 forward chain:"
  iptables -L FORWARD -v -n | awk -v dev="$interface" '/^Chain FORWARD/ || $0 ~ dev || $0 ~ "ufw-reject-forward"'
  echo
  info "ipv6 forward chain:"
  ip6tables -L FORWARD -v -n | awk -v dev="$interface" '/^Chain FORWARD/ || $0 ~ dev || $0 ~ "ufw6-reject-forward"'
}

check_ipv6_ipv4_forwarding() {
  local result_ipv4 result_ipv6
  result_ipv4=$(cat /proc/sys/net/ipv4/ip_forward)
  result_ipv6=$(cat /proc/sys/net/ipv6/conf/all/forwarding)
  ok "ipv4 forwarding is $([ "$result_ipv4" == "1" ] && ok enabled || error not enabled)"
  ok "ipv6 forwarding is $([ "$result_ipv6" == "1" ] && ok enabled || error not enabled)"
}

check_ip_routing() {
  info "ipv4 routing table:"
  ip route
  info "---------------------------"
  info "ipv6 routing table:"
  ip -6 route
}

perform_pings() {
  info "performing ipv4 ping to google.com"
  ping -4 -c 4 google.com || error "ipv4 ping failed"
  echo "---------------------------"
  info "performing ipv6 ping to google.com"
  ping6 -6 -c 4 google.com || error "ipv6 ping failed"
}

joke_through_tunnel() {
  ensure_jq
  local interface=$1

  sleep 1
  echo
  info "checking tunnel connectivity and fetching a joke for $interface"
  info "if this test succeeds, it confirms your machine can reach the outside world via ipv4 and ipv6"
  info "probes and external clients may still see different connectivity to your nym node"

  local ipv4_address ipv6_address joke
  ipv4_address=$(ip addr show "$interface" | awk '/inet / {print $2}' | cut -d'/' -f1)
  ipv6_address=$(ip addr show "$interface" | awk '/inet6 / && $2 !~ /^fe80/ {print $2}' | cut -d'/' -f1)

  if [[ -z "$ipv4_address" && -z "$ipv6_address" ]]; then
    error "no ip address found on $interface. unable to fetch a joke"
    error "please verify your tunnel configuration and ensure the interface is up"
    return 1
  fi

  if [[ -n "$ipv4_address" ]]; then
    echo
    echo "------------------------------------"
    info "detected ipv4 address: $ipv4_address"
    info "testing ipv4 connectivity"
    echo

    if ping -c 1 -I "$ipv4_address" google.com >/dev/null 2>&1; then
      ok "ipv4 connectivity is working. fetching a joke"
      joke=$(curl -s -H "Accept: application/json" --interface "$ipv4_address" https://icanhazdadjoke.com/ | jq -r .joke)
      [[ -n "$joke" && "$joke" != "null" ]] && ok "ipv4 joke: $joke" || echo "failed to fetch a joke via ipv4"
    else
      error "ipv4 connectivity is not working for $interface. verify your routing and nat settings"
    fi
  else
    error "no ipv4 address found on $interface. unable to fetch a joke via ipv4"
  fi

  if [[ -n "$ipv6_address" ]]; then
    echo
    echo "------------------------------------"
    info "detected ipv6 address: $ipv6_address"
    info "testing ipv6 connectivity"
    echo

    if ping6 -c 1 -I "$ipv6_address" google.com >/dev/null 2>&1; then
      ok "ipv6 connectivity is working. fetching a joke"
      joke=$(curl -s -H "Accept: application/json" --interface "$ipv6_address" https://icanhazdadjoke.com/ | jq -r .joke)
      [[ -n "$joke" && "$joke" != "null" ]] && ok "ipv6 joke: $joke" || error "failed to fetch a joke via ipv6"
    else
      error "ipv6 connectivity is not working for $interface. verify your routing and nat settings"
    fi
  else
    error "no ipv6 address found on $interface. unable to fetch a joke via ipv6"
  fi

  ok "joke fetching processes completed for $interface"
  echo "------------------------------------"

  sleep 3
  echo
  echo
  info "connectivity testing recommendations"
  info "- from another machine use wscat to test websocket connectivity on 9001"
  info "- test udp connectivity on port 51822 (wireguard)"
  info "- example: echo 'test' | nc -u <your-ip> 51822"
}

configure_dns_and_icmp_wg() {
  info "allowing ping (icmp) and dns on this host"
  iptables -C INPUT -p icmp --icmp-type echo-request -j ACCEPT 2>/dev/null || \
    iptables -A INPUT -p icmp --icmp-type echo-request -j ACCEPT
  iptables -C OUTPUT -p icmp --icmp-type echo-reply -j ACCEPT 2>/dev/null || \
    iptables -A OUTPUT -p icmp --icmp-type echo-reply -j ACCEPT

  iptables -C INPUT -p udp --dport 53 -j ACCEPT 2>/dev/null || \
    iptables -A INPUT -p udp --dport 53 -j ACCEPT
  iptables -C INPUT -p tcp --dport 53 -j ACCEPT 2>/dev/null || \
    iptables -A INPUT -p tcp --dport 53 -j ACCEPT

  save_iptables_rules
  ok "dns and icmp configuration completed"
}

###############################################################################
# part 2: wireguard exit policy manager
###############################################################################

add_port_rules() {
  local cmd="$1"    # iptables or ip6tables
  local port="$2"
  local protocol="${3:-tcp}"

  if [[ "$port" == *"-"* ]]; then
    local start_port end_port
    start_port=$(echo "$port" | cut -d'-' -f1)
    end_port=$(echo "$port" | cut -d'-' -f2)

    if ! $cmd -C "$NYM_CHAIN" -p "$protocol" --dport "$start_port:$end_port" -j ACCEPT 2>/dev/null; then
      $cmd -A "$NYM_CHAIN" -p "$protocol" --dport "$start_port:$end_port" -j ACCEPT
      ok "added $cmd $NYM_CHAIN $protocol port range $start_port:$end_port"
    fi
  else
    if ! $cmd -C "$NYM_CHAIN" -p "$protocol" --dport "$port" -j ACCEPT 2>/dev/null; then
      $cmd -A "$NYM_CHAIN" -p "$protocol" --dport "$port" -j ACCEPT
      ok "added $cmd $NYM_CHAIN $protocol port $port"
    fi
  fi
}

exit_policy_install_deps() {
  install_iptables_persistent

  for item in iptables ip6tables ip grep sed awk wget curl; do
    if ! command -v "$item" >/dev/null 2>&1; then
      info "installing dependency: $item"
      apt-get install -y "$item"
    fi
  done
}

create_nym_chain() {
  info "creating nym exit policy chain $NYM_CHAIN"

  if iptables -S "$NYM_CHAIN" >/dev/null 2>&1; then
    iptables -F "$NYM_CHAIN"
  else
    iptables -N "$NYM_CHAIN"
  fi

  if ip6tables -S "$NYM_CHAIN" >/dev/null 2>&1; then
    ip6tables -F "$NYM_CHAIN"
  else
    ip6tables -N "$NYM_CHAIN"
  fi

  if ! iptables -C FORWARD -i "$WG_INTERFACE" -o "$NETWORK_DEVICE" -j "$NYM_CHAIN" 2>/dev/null; then
    iptables -I FORWARD 1 -i "$WG_INTERFACE" -o "$NETWORK_DEVICE" -j "$NYM_CHAIN"
  fi

  if ! ip6tables -C FORWARD -i "$WG_INTERFACE" -o "$NETWORK_DEVICE" -j "$NYM_CHAIN" 2>/dev/null; then
    ip6tables -I FORWARD 1 -i "$WG_INTERFACE" -o "$NETWORK_DEVICE" -j "$NYM_CHAIN"
  fi
}

setup_nat_rules() {
  info "setting up nat and forwarding rules for $WG_INTERFACE via $NETWORK_DEVICE"

  if ! iptables -t nat -C POSTROUTING -o "$NETWORK_DEVICE" -j MASQUERADE 2>/dev/null; then
    iptables -t nat -A POSTROUTING -o "$NETWORK_DEVICE" -j MASQUERADE
  fi
  if ! ip6tables -t nat -C POSTROUTING -o "$NETWORK_DEVICE" -j MASQUERADE 2>/dev/null; then
    ip6tables -t nat -A POSTROUTING -o "$NETWORK_DEVICE" -j MASQUERADE
  fi

  if ! iptables -C FORWARD -i "$WG_INTERFACE" -o "$NETWORK_DEVICE" -j ACCEPT 2>/dev/null; then
    iptables -I FORWARD 1 -i "$WG_INTERFACE" -o "$NETWORK_DEVICE" -j ACCEPT
  fi
  if ! iptables -C FORWARD -i "$NETWORK_DEVICE" -o "$WG_INTERFACE" -m state --state RELATED,ESTABLISHED -j ACCEPT 2>/dev/null; then
    iptables -I FORWARD 2 -i "$NETWORK_DEVICE" -o "$WG_INTERFACE" -m state --state RELATED,ESTABLISHED -j ACCEPT
  fi

  if ! ip6tables -C FORWARD -i "$WG_INTERFACE" -o "$NETWORK_DEVICE" -j ACCEPT 2>/dev/null; then
    ip6tables -I FORWARD 1 -i "$WG_INTERFACE" -o "$NETWORK_DEVICE" -j ACCEPT
  fi
  if ! ip6tables -C FORWARD -i "$NETWORK_DEVICE" -o "$WG_INTERFACE" -m state --state RELATED,ESTABLISHED -j ACCEPT 2>/dev/null; then
    ip6tables -I FORWARD 2 -i "$NETWORK_DEVICE" -o "$WG_INTERFACE" -m state --state RELATED,ESTABLISHED -j ACCEPT
  fi
}

configure_exit_dns_and_icmp() {
  info "ensuring dns and icmp are allowed inside nym exit chain"

  # Remove any existing DNS/ICMP rules first to avoid duplicates
  iptables -D "$NYM_CHAIN" -p udp --dport 53 -j ACCEPT 2>/dev/null || true
  iptables -D "$NYM_CHAIN" -p tcp --dport 53 -j ACCEPT 2>/dev/null || true
  iptables -D "$NYM_CHAIN" -p icmp --icmp-type echo-request -j ACCEPT 2>/dev/null || true
  iptables -D "$NYM_CHAIN" -p icmp --icmp-type echo-reply -j ACCEPT 2>/dev/null || true
  ip6tables -D "$NYM_CHAIN" -p udp --dport 53 -j ACCEPT 2>/dev/null || true
  ip6tables -D "$NYM_CHAIN" -p tcp --dport 53 -j ACCEPT 2>/dev/null || true
  ip6tables -D "$NYM_CHAIN" -p ipv6-icmp -j ACCEPT 2>/dev/null || true

  # Insert rules at the beginning in correct order: DNS first, then ICMP
  iptables -I "$NYM_CHAIN" 1 -p udp --dport 53 -j ACCEPT
  iptables -I "$NYM_CHAIN" 2 -p tcp --dport 53 -j ACCEPT
  iptables -I "$NYM_CHAIN" 3 -p icmp --icmp-type echo-request -j ACCEPT
  iptables -I "$NYM_CHAIN" 4 -p icmp --icmp-type echo-reply -j ACCEPT
  ip6tables -I "$NYM_CHAIN" 1 -p udp --dport 53 -j ACCEPT
  ip6tables -I "$NYM_CHAIN" 2 -p tcp --dport 53 -j ACCEPT
  ip6tables -I "$NYM_CHAIN" 3 -p ipv6-icmp -j ACCEPT
}

apply_port_allowlist() {
  echo "applying allowed port list into ${NYM_CHAIN}"

  configure_exit_dns_and_icmp

  declare -A PORT_MAPPINGS=(
    ["FTP"]="20-21"
    ["SSH"]="22"
    ["WHOIS"]="43"
    ["DNS"]="53"
    ["Finger"]="79"
    ["HTTP"]="80-81"
    ["Kerberos"]="88"
    ["POP3"]="110"
    ["NTP"]="123"
    ["IMAP"]="143"
    ["IMAP3"]="220"
    ["LDAP"]="389"
    ["HTTPS"]="443"
    ["SMBWindowsFileShare"]="445"
    ["Kpasswd"]="464"
    ["RTSP"]="554"
    ["SMTPSubmission"]="587"
    ["LDAPS"]="636"
    ["SILC"]="706"
    ["KerberosAdmin"]="749"
    ["DNSOverTLS"]="853"
    ["Rsync"]="873"
    ["VMware"]="902-904"
    ["RemoteHTTPS"]="981"
    ["FTPOverTLS"]="989-990"
    ["NetnewsAdmin"]="991"
    ["TelnetOverTLS"]="992"
    ["IMAPOverTLS"]="993"
    ["POP3OverTLS"]="995"
    ["OpenVPN"]="1194"
    ["WireGuardPeer"]="51820-51822"
    ["QTServerAdmin"]="1220"
    ["PKTKRB"]="1293"
    ["MSSQL"]="1433"
    ["VLSILicenseManager"]="1500"
    ["OracleDB"]="1521"
    ["Sametime"]="1533"
    ["GroupWise"]="1677"
    ["PPTP"]="1723"
    ["RTSPAlt"]="1755"
    ["MSNP"]="1863"
    ["NFS"]="2049"
    ["CPanel"]="2082-2083"
    ["GNUnet"]="2086-2087"
    ["NBX"]="2095-2096"
    ["Zephyr"]="2102-2104"
    ["XboxLive"]="3074"
    ["MySQL"]="3306"
    ["SVN"]="3690"
    ["RWHOIS"]="4321"
    ["Virtuozzo"]="4643"
    ["RTPVOIP"]="5000-5005"
    ["MMCC"]="5050"
    ["ICQ"]="5190"
    ["XMPP"]="5222-5223"
    ["AndroidMarket"]="5228"
    ["PostgreSQL"]="5432"
    ["MongoDBDefault"]="27017"
    ["Electrum"]="8082"
    ["SimplifyMedia"]="8087-8088"
    ["Zcash"]="8232-8233"
    ["Bitcoin"]="8332-8333"
    ["HTTPSALT"]="8443"
    ["TeamSpeak"]="8767"
    ["MQTTS"]="8883"
    ["HTTPProxy"]="8888"
    ["TorORPort"]="9001"
    ["TorDirPort"]="9030"
    ["Tari"]="9053"
    ["Gaming"]="9339"
    ["Git"]="9418"
    ["HTTPSALT2"]="9443"
    ["Lightning"]="9735"
    ["NDMP"]="10000"
    ["OpenPGP"]="11371"
    ["Monero"]="18080-18081"
    ["MoneroRPC"]="18089"
    ["GoogleVoice"]="19294"
    ["EnsimControlPanel"]="19638"
    ["Session"]="22021"
    ["DarkFiTor"]="25551"
    ["Minecraft"]="25565"
    ["DarkFi"]="26661"
    ["Steam"]="27000-27050"
    ["WhatsAppRange"]="3478-3484"
    ["ElectrumSSL"]="50002"
    ["MOSH"]="60000-61000"
    ["Mumble"]="64738"
    ["Metadata"]="51830"
  )

  local port
  for service in "${!PORT_MAPPINGS[@]}"; do
    port="${PORT_MAPPINGS[$service]}"
    echo "adding rules for $service (ports $port)"
    add_port_rules iptables "$port" "tcp"
    add_port_rules ip6tables "$port" "tcp"
    add_port_rules iptables "$port" "udp"
    add_port_rules ip6tables "$port" "udp"
  done
}

apply_spamhaus_blocklist() {
  info "applying spamhaus-like blocklist from $EXIT_POLICY_LOCATION"

  mkdir -p "$(dirname "$POLICY_FILE")"

  if ! wget -q "$EXIT_POLICY_LOCATION" -O "$POLICY_FILE" 2>/dev/null; then
    error "failed to download exit policy, using minimal blocklist"
    cat >"$POLICY_FILE" <<EOF
ExitPolicy reject 5.188.10.0/23:*
ExitPolicy reject 31.132.36.0/22:*
ExitPolicy reject 37.9.42.0/24:*
ExitPolicy reject 45.43.128.0/18:*
ExitPolicy reject *:*
EOF
  fi

  local tmpfile
  tmpfile=$(mktemp)

  grep "^ExitPolicy reject" "$POLICY_FILE" | grep -v "\*:\*" > "$tmpfile"

  local total_rules
  total_rules=$(wc -l < "$tmpfile")
  info "processing $total_rules blocklist rules"
  local line ip_range
  while IFS= read -r line; do
    [[ -z "$line" ]] && continue

    ip_range=$(echo "$line" | sed -E 's/ExitPolicy reject ([^:]+):.*/\1/')

    if [[ -n "$ip_range" ]]; then

      # ipv4 reject
      if ! iptables -C "$NYM_CHAIN" -d "$ip_range" -j REJECT 2>/dev/null; then
        iptables -A "$NYM_CHAIN" -d "$ip_range" -j REJECT --reject-with icmp-port-unreachable \
          || error "warning: failed adding ipv4 reject for $ip_range"
      fi

      # ipv6 reject
      if [[ "$ip_range" == *":"* ]]; then
        if ! ip6tables -C "$NYM_CHAIN" -d "$ip_range" -j REJECT 2>/dev/null; then
          ip6tables -A "$NYM_CHAIN" -d "$ip_range" -j REJECT \
            || error "warning: failed adding ipv6 reject for $ip_range"
        fi
      fi

    fi
  done < "$tmpfile"

  rm -f "$tmpfile"
}



add_default_reject_rule() {
  info "ensuring default reject rule at end of ${NYM_CHAIN}"

  iptables -D "$NYM_CHAIN" -j REJECT 2>/dev/null || true
  iptables -D "$NYM_CHAIN" -j REJECT --reject-with icmp-port-unreachable 2>/dev/null || true
  ip6tables -D "$NYM_CHAIN" -j REJECT 2>/dev/null || true
  ip6tables -D "$NYM_CHAIN" -j REJECT --reject-with icmp6-port-unreachable 2>/dev/null || true

  iptables -A "$NYM_CHAIN" -j REJECT --reject-with icmp-port-unreachable
  ip6tables -A "$NYM_CHAIN" -j REJECT --reject-with icmp6-port-unreachable
}

clear_exit_policy_rules() {
  info "clearing nym exit policy rules ..."

  iptables -F "$NYM_CHAIN" 2>/dev/null || true
  ip6tables -F "$NYM_CHAIN" 2>/dev/null || true

  iptables -D FORWARD -i "$WG_INTERFACE" -o "$NETWORK_DEVICE" -j "$NYM_CHAIN" 2>/dev/null || true
  ip6tables -D FORWARD -i "$WG_INTERFACE" -o "$NETWORK_DEVICE" -j "$NYM_CHAIN" 2>/dev/null || true

  iptables -X "$NYM_CHAIN" 2>/dev/null || true
  ip6tables -X "$NYM_CHAIN" 2>/dev/null || true
}

show_exit_policy_status() {
  info "nym exit policy status"
  info "network device: $NETWORK_DEVICE"
  info "wireguard interface: $WG_INTERFACE"
  echo

  if ! ip link show "$WG_INTERFACE" >/dev/null 2>&1; then
    error "warning: wireguard interface $WG_INTERFACE not found"
  else
    info "interface details:"
    ip link show "$WG_INTERFACE"
    echo
    info "ipv4 addresses:"
    ip -4 addr show dev "$WG_INTERFACE"
    echo
    info "ipv6 addresses:"
    ip -6 addr show dev "$WG_INTERFACE"
  fi

  echo
  info "iptables chains for ${NYM_CHAIN}:"
  iptables -L "$NYM_CHAIN" -n -v 2>/dev/null || echo "ipv4 chain not found"
  echo
  ip6tables -L "$NYM_CHAIN" -n -v 2>/dev/null || echo "ipv6 chain not found"
  echo
  info "ip forwarding:"
  echo "ipv4: $(cat /proc/sys/net/ipv4/ip_forward 2>/dev/null || echo 0)"
  echo "ipv6: $(cat /proc/sys/net/ipv6/conf/all/forwarding 2>/dev/null || echo 0)"
}

test_exit_policy_connectivity() {
  info "testing connectivity through $WG_INTERFACE"

  local iface_info
  iface_info=$(ip link show "$WG_INTERFACE" 2>/dev/null || true)
  if [[ -z "$iface_info" ]]; then
    error "interface $WG_INTERFACE not found"
    return 1
  fi

  ok "interface:"
  ok "$iface_info"

  local ipv4_address ipv6_address
  ipv4_address=$(ip -4 addr show dev "$WG_INTERFACE" | awk '/inet / {print $2}' | cut -d'/' -f1 | head -n1)
  ipv6_address=$(ip -6 addr show dev "$WG_INTERFACE" scope global | awk '/inet6/ {print $2}' | cut -d'/' -f1 | head -n1)

  ok "ipv4 address: ${ipv4_address:-none}"
  ok "ipv6 address: ${ipv6_address:-none}"

  if [[ -n "$ipv4_address" ]]; then
     echo -e "${NC}testing ipv4 ping to 8.8.8.8 ..."
    timeout 5 ping -c 3 -I "$ipv4_address" 8.8.8.8 >/dev/null 2>&1 && \
      ok "ipv4 ping ok" || error "ipv4 ping failed"

    echo -e "${NC}testing ipv4 dns resolution ..."
    timeout 5 ping -c 3 -I "$ipv4_address" google.com >/dev/null 2>&1 && \
      ok "ipv4 dns ok" || error "ipv4 dns failed"
  fi

  if [[ -n "$ipv6_address" ]]; then
    echo -e "${NC}testing ipv6 ping to google dns ..."
    timeout 5 ping6 -c 3 -I "$ipv6_address" 2001:4860:4860::8888 >/dev/null 2>&1 && \
      ok "ipv6 ping ok" || error "ipv6 ping failed"

    echo -e "${NC}testing ipv6 dns resolution ..."
    timeout 5 ping6 -c 3 -I "$ipv6_address" google.com >/dev/null 2>&1 && \
      ok "ipv6 dns ok" || error "ipv6 dns failed"
  fi

  ok "connectivity tests finished"
}


###############################################################################
# part 3: check the firewall setup
###############################################################################

firewall_rule_line() {
  local chain=$1
  local rule_idx=$2
  # this is because thefirst rule appears on line 3
  iptables -L "$chain" -n --line-numbers | sed -n "$((rule_idx + 2))p"
}

check_forward_chain() {
  local output
  output=$(iptables -L FORWARD -n --line-numbers)

  if ! echo "$output" | grep -q "^1[[:space:]]\+$NYM_CHAIN"; then
    error "FORWARD rule 1 is not ${NYM_CHAIN}; re-run network-tunnel-manager.sh exit_policy_install"
    return 1
  fi

  if ! echo "$output" | grep -q "ACCEPT.*state RELATED,ESTABLISHED"; then
    error "FORWARD chain missing RELATED,ESTABLISHED accepts; re-run network-tunnel-manager.sh apply_iptables_rules_wg"
    return 1
  fi

  ok "FORWARD chain ordering looks good"
  return 0
}

check_nym_exit_chain() {
  local errors=0
  local patterns=("udp.*dpt:53" "tcp.*dpt:53" "icmp.*type 8" "icmp.*type 0")

  for idx in "${!patterns[@]}"; do
    local line
    line=$(firewall_rule_line "$NYM_CHAIN" $((idx + 1)))
    if [[ "$line" =~ ${patterns[$idx]} ]]; then
      ok "${NYM_CHAIN} rule $((idx + 1)) ok (${patterns[$idx]})"
    else
      error "${NYM_CHAIN} rule $((idx + 1)) is not ${patterns[$idx]}; re-run network-tunnel-manager.sh exit_policy_install"
      errors=1
    fi
  done

  local last_rule
  last_rule=$(iptables -L "$NYM_CHAIN" -n --line-numbers | awk 'NR>2 {line=$0} END {print line}')
  if [[ -z "${last_rule:-}" ]]; then
    error "${NYM_CHAIN} chain is empty; re-run network-tunnel-manager.sh exit_policy_install"
    errors=1
  elif [[ "$last_rule" =~ REJECT ]] && [[ "$last_rule" =~ 0\.0\.0\.0/0 ]]; then
    ok "${NYM_CHAIN} ends with the catch-all REJECT"
  else
    error "${NYM_CHAIN} final rule is not the catch-all REJECT (got: $last_rule)"
    errors=1
  fi

  return $errors
}

check_iptables_default_policies() {
  info "checking base iptables default policies (INPUT/FORWARD)"

  local issues=0
  local input_policy forward_policy output_policy

  input_policy=$(iptables -S INPUT 2>/dev/null | awk 'NR==1 && $1=="-P" {print $3}')
  forward_policy=$(iptables -S FORWARD 2>/dev/null | awk 'NR==1 && $1=="-P" {print $3}')
  output_policy=$(iptables -S OUTPUT 2>/dev/null | awk 'NR==1 && $1=="-P" {print $3}')

  if [[ -z "${input_policy:-}" ]]; then
    error "unable to read INPUT default policy (iptables -S INPUT failed?)"
    issues=1
  elif [[ "${input_policy^^}" != "DROP" ]]; then
    error "INPUT default policy is ${input_policy^^}; expected DROP so traffic is only allowed by explicit rules."
    issues=1
  else
    ok "INPUT default policy is DROP"
  fi

  if [[ -z "${forward_policy:-}" ]]; then
    error "unable to read FORWARD default policy (iptables -S FORWARD failed?)"
    issues=1
  elif [[ "${forward_policy^^}" != "DROP" ]]; then
    error "FORWARD default policy is ${forward_policy^^}; expected DROP to ensure traffic only flows via NYM-EXIT rules."
    issues=1
  else
    ok "FORWARD default policy is DROP"
  fi

  if [[ -z "${output_policy:-}" ]]; then
    error "unable to read OUTPUT default policy (iptables -S OUTPUT failed?)"
    issues=1
  elif [[ "${output_policy^^}" != "ACCEPT" ]]; then
    error "OUTPUT default policy is ${output_policy^^}; expected ACCEPT"
    issues=1
  else
    ok "OUTPUT default policy is ACCEPT"
  fi

  return $issues
}

check_firewall_setup() {
  info "checking ipv4 firewall ordering…"
  local errors=0

  check_iptables_default_policies || errors=1
  check_forward_chain || errors=1
  check_nym_exit_chain || errors=1

  if command -v ip6tables >/dev/null 2>&1; then
    info "checking ipv6 firewall ordering…"
    if ip6tables -L "$NYM_CHAIN" -n --line-numbers >/dev/null 2>&1; then
      if ! ip6tables -L "$NYM_CHAIN" -n --line-numbers | sed -n '3p' | grep -q "udp.*dpt:53"; then
        error "ip6tables ${NYM_CHAIN} rule 1 is not UDP 53"
        errors=1
      fi
    fi
  fi

  if [[ $errors -ne 0 ]]; then
    error "There may be some ordering issues, it is recommended to re-run network-tunnel-manager.sh exit_policy_install after configuring UFW."
    return 1
  fi

  ok "It's looking good!"
  return 0
}


###############################################################################
# part 4: full exit policy verification tests
###############################################################################

test_port_range_rules() {
  info "testing port range rules in ${NYM_CHAIN}"

  local port_ranges=(
    "20-21:tcp:ftp"
    "80-81:tcp:http"
    "2082-2083:tcp:cpanel"
    "5222-5223:tcp:xmpp"
    "27000-27050:tcp:steam-sample"
    "989-990:tcp:ftp-tls"
    "5000-5005:tcp:rtp-voip"
    "8087-8088:tcp:simplify-media"
    "8232-8233:tcp:zcash"
    "8332-8333:tcp:bitcoin"
    "18080-18081:tcp:monero"
    "3478-3484:tcp:whatsapp"
  )

  local failures=0
  local start end
  for entry in "${port_ranges[@]}"; do
    IFS=':' read -r range proto name <<<"$entry"
    start=$(echo "$range" | cut -d'-' -f1)
    end=$(echo "$range" | cut -d'-' -f2)

    if iptables -t filter -C "$NYM_CHAIN" -p "$proto" --dport "$start:$end" -j ACCEPT 2>/dev/null; then
      ok "rule ok: $name $proto $range"
    else
      error "missing rule: $name $proto $range"
      ((failures++))
    fi
  done

  return "$failures"
}

test_critical_services() {
  info "testing critical service rules in ${NYM_CHAIN}"

  local tcp_ports=(22 53 443 853 1194)
  local udp_ports=(53 123 1194)
  local failures=0

  for port in "${tcp_ports[@]}"; do
    if iptables -t filter -C "$NYM_CHAIN" -p tcp --dport "$port" -j ACCEPT 2>/dev/null; then
      ok "tcp port $port allowed"
    else
      if iptables-save | grep -E "^-A $NYM_CHAIN.*tcp.*dpts:" | grep -q "$port"; then
        ok "tcp port $port allowed by range"
      else
        error "tcp port $port missing"
        ((failures++))
      fi
    fi
  done

  for port in "${udp_ports[@]}"; do
    if iptables -t filter -C "$NYM_CHAIN" -p udp --dport "$port" -j ACCEPT 2>/dev/null; then
      ok "udp port $port allowed"
    else
      if iptables-save | grep -E "^-A $NYM_CHAIN.*udp.*dpts:" | grep -q "$port"; then
        ok "udp port $port allowed by range"
      else
        error "udp port $port missing"
        ((failures++))
      fi
    fi
  done

  return "$failures"
}

test_forward_chain_hook() {
  info "testing forward chain hook direction for ${NYM_CHAIN}"

  local failures=0

  if iptables -C FORWARD -i "$WG_INTERFACE" -o "$NETWORK_DEVICE" -j "$NYM_CHAIN" 2>/dev/null; then
    ok "ipv4 forward hook ok: -i $WG_INTERFACE -o $NETWORK_DEVICE -> $NYM_CHAIN"
  else
    error "ipv4 forward hook missing or wrong"
    ((failures++))
  fi

  if ip6tables -C FORWARD -i "$WG_INTERFACE" -o "$NETWORK_DEVICE" -j "$NYM_CHAIN" 2>/dev/null; then
    ok "ipv6 forward hook ok: -i $WG_INTERFACE -o $NETWORK_DEVICE -> $NYM_CHAIN"
  else
    error "ipv6 forward hook missing or wrong"
    ((failures++))
  fi

  return "$failures"
}

test_default_reject_rule() {
  info "testing default reject rule position in ${NYM_CHAIN}"

  local last_rule_v4
  last_rule_v4=$(iptables -S "$NYM_CHAIN" | awk '/^-A /{rule=$0} END{print rule}')
  if [[ "$last_rule_v4" != "-A $NYM_CHAIN -j REJECT --reject-with icmp-port-unreachable" ]]; then
    error "default reject missing or not last in ipv4 chain"
    return 1
  fi

  local last_rule_v6
  last_rule_v6=$(ip6tables -S "$NYM_CHAIN" | awk '/^-A /{rule=$0} END{print rule}')
  if [[ "$last_rule_v6" != "-A $NYM_CHAIN -j REJECT --reject-with icmp6-port-unreachable" ]]; then
    error "default reject missing or not last in ipv6 chain"
    return 1
  fi

  ok "default reject confirmed at end of ${NYM_CHAIN}"
}

exit_policy_run_tests() {
  local skip_default=0
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --skip-default-reject) skip_default=1; shift ;;
      *) error "unknown test option: $1"; return 1 ;;
    esac
  done

  local total=0
  local failed=0

  test_forward_chain_hook || ((failed += 1))
  ((total += 1))

  test_port_range_rules || ((failed += 1))
  ((total += 1))

  test_critical_services || ((failed += 1))
  ((total += 1))

  if [[ $skip_default -eq 0 ]]; then
    test_default_reject_rule || ((failed += 1))
    ((total += 1))
  fi

  info "tests run: ${GREEN}$total${YELLOW}, test failed: ${RED}$failed${NC}"
  if [[ $failed -eq 0 ]]; then
    ok "all exit policy tests passed"
  else
    error "some exit policy tests failed"
  fi

  return "$failed"
}

###############################################################################
# part 5: high level workflows
###############################################################################

nym_tunnel_setup() {
  # this mirrors your previous chain of calls but inside one script
  info "running full tunnel setup for ${TUNNEL_INTERFACE} and ${WG_INTERFACE}"

  check_tunnel_iptables "$TUNNEL_INTERFACE"
  remove_duplicate_rules "$TUNNEL_INTERFACE"
  remove_duplicate_rules "$WG_INTERFACE"
  check_tunnel_iptables "$TUNNEL_INTERFACE"

  adjust_ip_forwarding

  apply_iptables_rules "$TUNNEL_INTERFACE"
  check_tunnel_iptables "$TUNNEL_INTERFACE"

  apply_iptables_rules "$WG_INTERFACE"

  configure_dns_and_icmp_wg
  adjust_ip_forwarding
  check_ipv6_ipv4_forwarding

  joke_through_tunnel "$TUNNEL_INTERFACE"
  joke_through_tunnel "$WG_INTERFACE"

  ok "full tunnel setup completed"
}

exit_policy_install() {
  info "installing nym wireguard exit policy for ${WG_INTERFACE} via ${NETWORK_DEVICE}"
  exit_policy_install_deps
  adjust_ip_forwarding
  create_nym_chain
  setup_nat_rules
  apply_port_allowlist
  apply_spamhaus_blocklist
  add_default_reject_rule
  save_iptables_rules
  ok "nym exit policy installed"
}

complete_networking_configuration() {
  info "starting complete networking configuration: tunnels + exit policy"

  nym_tunnel_setup
  exit_policy_install
  check_firewall_setup || error "firewall order checks reported problems, please review output"
  exit_policy_run_tests || error "exit policy tests reported problems, please review output"

  ok "complete networking configuration finished"
}

###############################################################################
# cli
###############################################################################

cmd="${1:-help}"
log "COMMAND: $cmd ARGS: $*"

case "$cmd" in
  nym_tunnel_setup)
    nym_tunnel_setup
    status=$?
    ;;
  exit_policy_install)
    exit_policy_install
    status=$?
    ;;
  complete_networking_configuration)
    complete_networking_configuration
    status=$?
    ;;

  # tunnel manager cmds
  fetch_ipv6_address_nym_tun)
    fetch_ipv6_address "$TUNNEL_INTERFACE"
    status=$?
    ;;
  fetch_and_display_ipv6)
    fetch_and_display_ipv6
    status=$?
    ;;
  apply_iptables_rules)
    apply_iptables_rules "$TUNNEL_INTERFACE"
    status=$?
    ;;
  apply_iptables_rules_wg)
    apply_iptables_rules "$WG_INTERFACE"
    status=$?
    ;;
  check_nymtun_iptables)
    check_tunnel_iptables "$TUNNEL_INTERFACE"
    status=$?
    ;;
  check_nym_wg_tun)
    check_tunnel_iptables "$WG_INTERFACE"
    status=$?
    ;;
  check_ipv6_ipv4_forwarding)
    check_ipv6_ipv4_forwarding
    status=$?
    ;;
  check_ip_routing)
    check_ip_routing
    status=$?
    ;;
  perform_pings)
    perform_pings
    status=$?
    ;;
  joke_through_the_mixnet)
    joke_through_tunnel "$TUNNEL_INTERFACE"
    status=$?
    ;;
  joke_through_wg_tunnel)
    joke_through_tunnel "$WG_INTERFACE"
    status=$?
    ;;
  configure_dns_and_icmp_wg)
    configure_dns_and_icmp_wg
    status=$?
    ;;
  adjust_ip_forwarding)
    adjust_ip_forwarding
    status=$?
    ;;
  remove_duplicate_rules)
    remove_duplicate_rules "${2:-}"
    status=$?
    ;;

  # exit policy manager cmds
  exit_policy_status)
    show_exit_policy_status
    status=$?
    ;;
  check_firewall_setup)
    check_firewall_setup
    status=$?
    ;;
  exit_policy_test_connectivity)
    test_exit_policy_connectivity
    status=$?
    ;;
  exit_policy_clear)
    clear_exit_policy_rules
    status=$?
    ;;
  exit_policy_tests)
    shift
    exit_policy_run_tests "$@"
    status=$?
    ;;

  help|--help|-h)
    cat <<EOF
usage: $0 <command> [args]

high level workflows:
  complete_networking_configuration Install tunnel interfaces, setup networking, iptables, wg exit policy & tests
  nym_tunnel_setup                 Install tunnel interfaces & setup networking
  exit_policy_install               Install and configure wireguard exit policy
tunnel and nat helpers:
  adjust_ip_forwarding              Enable ipv4/ipv6 forwarding via sysctl.d
  apply_iptables_rules              Apply nat/forward rules for ${TUNNEL_INTERFACE}
  apply_iptables_rules_wg           Apply nat/forward rules for ${WG_INTERFACE}
  check_ip_routing                  Show ipv4 and ipv6 routes
  check_ipv6_ipv4_forwarding        Show ipv4/ipv6 forwarding flags
  check_nym_wg_tun                  Inspect forward chain for ${WG_INTERFACE}
  check_nymtun_iptables             Inspect forward chain for ${TUNNEL_INTERFACE}
  configure_dns_and_icmp_wg         Allow ping and dns ports on this host
  fetch_and_display_ipv6            Show ipv6 on uplink ${NETWORK_DEVICE}
  fetch_ipv6_address_nym_tun        Show global ipv6 address on ${TUNNEL_INTERFACE}
  joke_through_the_mixnet           Test via ${TUNNEL_INTERFACE} with joke
  joke_through_wg_tunnel            Test via ${WG_INTERFACE} with joke
  perform_pings                     Test ipv4 and ipv6 pings
  remove_duplicate_rules <iface>    Deduplicate FORWARD and ${NYM_CHAIN} rules for <iface> (required).

exit policy manager:
  check_firewall_setup              Run ordering sanity check (dns/icmp + FORWARD jump)
  exit_policy_clear                 Remove ${NYM_CHAIN} chains and hooks
  exit_policy_install               Install exit policy (iptables rules and blocklist)
  exit_policy_status                Show status of exit policy and forwarding
  exit_policy_test_connectivity     Test connectivity via ${WG_INTERFACE}
  exit_policy_tests [--skip-default-reject]
                                    Run verification tests on exit policy (options: --skip-default-reject).

environment overrides:
  NETWORK_DEVICE                    Auto-detected uplink (e.g., eth0). Set manually if detection fails.
  TUNNEL_INTERFACE                  Default: nymtun0. Requires root privileges (sudo) to manage.
  WG_INTERFACE                      Default: nymwg - Must match your WireGuard interface name.

EOF
    status=0
    ;;

  *)
    error "unknown command: $cmd"
    info "run with 'help' for usage"
    exit 1
    ;;
esac

if [[ "$cmd" != help && "$cmd" != "--help" && "$cmd" != "-h" && ${status:-1} -eq 0 ]]; then
    echo ""
    echo "Logs saved locally at: $LOG_FILE"
    ok "operation ${cmd} completed"
fi
END_TIME=$(date +%s)
ELAPSED=$((END_TIME - START_TIME))
echo "----- $(date '+%Y-%m-%d %H:%M:%S') END operation ${cmd} (status $status, duration ${ELAPSED}s) -----" >> "$LOG_FILE"
exit $status
