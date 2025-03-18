#!/bin/bash
#
# Nym Wireguard Exit Policy Manager
# Version: 1.0.0
#
# This script manages iptables rules for Nym Wireguard exit policies
# Features:
# - Implements the Nym exit policy from official documentation
# - Makes rules persistent across reboots
# - Provides commands to inspect and manage rules
# - Groups rules logically for easier management
# - Integrates with existing Nym node configuration
#
# Usage: ./nym-exit-policy.sh [command]

set -e

NETWORK_DEVICE=$(ip route show default | awk '/default/ {print $5}')
WG_INTERFACE="nymwg"
NYM_CHAIN="NYM-EXIT"
POLICY_FILE="/etc/nym/exit-policy.txt"
EXIT_POLICY_LOCATION="https://nymtech.net/.wellknown/network-requester/exit-policy.txt"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

add_port_rules() {
    local chain="$1"
    local port="$2"
    local protocol="${3:-tcp}"

    # Check if the port contains a range
    if [[ "$port" == *"-"* ]]; then
        # Port range handling - add as a single rule with a range
        local start_port=$(echo "$port" | cut -d'-' -f1)
        local end_port=$(echo "$port" | cut -d'-' -f2)

        if ! $chain -C "$NYM_CHAIN" -p "$protocol" --dport "$start_port:$end_port" -j ACCEPT 2>/dev/null; then
            $chain -A "$NYM_CHAIN" -p "$protocol" --dport "$start_port:$end_port" -j ACCEPT
            echo -e "  ${GREEN}Added: $NYM_CHAIN $protocol port range $start_port:$end_port${NC}"
        fi
    else
        # Single port handling
        if ! $chain -C "$NYM_CHAIN" -p "$protocol" --dport "$port" -j ACCEPT 2>/dev/null; then
            $chain -A "$NYM_CHAIN" -p "$protocol" --dport "$port" -j ACCEPT
            echo -e "  ${GREEN}Added: $NYM_CHAIN $protocol port $port${NC}"
        fi
    fi
}

install_dependencies() {
    if ! dpkg -s iptables-persistent >/dev/null 2>&1; then
        echo -e "${YELLOW}Installing iptables-persistent...${NC}"
        apt-get update
        DEBIAN_FRONTEND=noninteractive apt-get install -y iptables-persistent
        echo -e "${GREEN}iptables-persistent installed.${NC}"
    else
        echo -e "${GREEN}iptables-persistent is already installed.${NC}"
    fi

    # Check for other required dependencies
    for cmd in iptables ip6tables ip grep sed awk wget curl; do
        if ! command -v "$cmd" &>/dev/null; then
            echo -e "${YELLOW}Installing $cmd...${NC}"
            apt-get install -y "$cmd"
        fi
    done
}

configure_ip_forwarding() {
    echo -e "${YELLOW}Configuring IP forwarding...${NC}"

    # Remove any existing forwarding settings to avoid duplicates
    sed -i "/^net.ipv6.conf.all.forwarding=/d" /etc/sysctl.conf
    sed -i "/^net.ipv4.ip_forward=/d" /etc/sysctl.conf

    # Add forwarding settings
    echo "net.ipv6.conf.all.forwarding=1" | tee -a /etc/sysctl.conf
    echo "net.ipv4.ip_forward=1" | tee -a /etc/sysctl.conf

    # Apply changes
    sysctl -p /etc/sysctl.conf

    # Verify settings
    ipv4_forwarding=$(cat /proc/sys/net/ipv4/ip_forward)
    ipv6_forwarding=$(cat /proc/sys/net/ipv6/conf/all/forwarding)

    if [ "$ipv4_forwarding" == "1" ] && [ "$ipv6_forwarding" == "1" ]; then
        echo -e "${GREEN}IP forwarding configured successfully.${NC}"
    else
        echo -e "${RED}Failed to configure IP forwarding.${NC}"
        exit 1
    fi
}

create_nym_chain() {
    echo -e "${YELLOW}Creating Nym exit policy chain...${NC}"

    # Check if the chain already exists
    if iptables -L "$NYM_CHAIN" &>/dev/null; then
        echo -e "${YELLOW}Chain $NYM_CHAIN already exists. Flushing it...${NC}"
        iptables -F "$NYM_CHAIN"
    else
        echo -e "${YELLOW}Creating chain $NYM_CHAIN...${NC}"
        iptables -N "$NYM_CHAIN"
    fi

    # Do the same for IPv6
    if ip6tables -L "$NYM_CHAIN" &>/dev/null; then
        echo -e "${YELLOW}Chain $NYM_CHAIN already exists in ip6tables. Flushing it...${NC}"
        ip6tables -F "$NYM_CHAIN"
    else
        echo -e "${YELLOW}Creating chain $NYM_CHAIN in ip6tables...${NC}"
        ip6tables -N "$NYM_CHAIN"
    fi

    # Link it to the FORWARD chain if not already linked
    if ! iptables -C FORWARD -o "$WG_INTERFACE" -j "$NYM_CHAIN" 2>/dev/null; then
        echo -e "${YELLOW}Linking $NYM_CHAIN to FORWARD chain...${NC}"
        iptables -A FORWARD -o "$WG_INTERFACE" -j "$NYM_CHAIN"
    fi

    # Link IPv6 chain
    if ! ip6tables -C FORWARD -o "$WG_INTERFACE" -j "$NYM_CHAIN" 2>/dev/null; then
        echo -e "${YELLOW}Linking $NYM_CHAIN to IPv6 FORWARD chain...${NC}"
        ip6tables -A FORWARD -o "$WG_INTERFACE" -j "$NYM_CHAIN"
    fi
}

setup_nat_rules() {
    echo -e "${YELLOW}Setting up NAT rules...${NC}"

    # Check if NAT rule for IPv4 exists
    if ! iptables -t nat -C POSTROUTING -o "$NETWORK_DEVICE" -j MASQUERADE 2>/dev/null; then
        iptables -t nat -A POSTROUTING -o "$NETWORK_DEVICE" -j MASQUERADE
        echo -e "${GREEN}Added IPv4 NAT rule.${NC}"
    else
        echo -e "${GREEN}IPv4 NAT rule already exists.${NC}"
    fi

    # Check if NAT rule for IPv6 exists
    if ! ip6tables -t nat -C POSTROUTING -o "$NETWORK_DEVICE" -j MASQUERADE 2>/dev/null; then
        ip6tables -t nat -A POSTROUTING -o "$NETWORK_DEVICE" -j MASQUERADE
        echo -e "${GREEN}Added IPv6 NAT rule.${NC}"
    else
        echo -e "${GREEN}IPv6 NAT rule already exists.${NC}"
    fi

    # Setup forwarding rules for Wireguard interface
    if ! iptables -C FORWARD -i "$WG_INTERFACE" -o "$NETWORK_DEVICE" -j ACCEPT 2>/dev/null; then
        iptables -A FORWARD -i "$WG_INTERFACE" -o "$NETWORK_DEVICE" -j ACCEPT
        echo -e "${GREEN}Added IPv4 forwarding rule (WG → Internet).${NC}"
    fi

    if ! iptables -C FORWARD -i "$NETWORK_DEVICE" -o "$WG_INTERFACE" -m state --state RELATED,ESTABLISHED -j ACCEPT 2>/dev/null; then
        iptables -A FORWARD -i "$NETWORK_DEVICE" -o "$WG_INTERFACE" -m state --state RELATED,ESTABLISHED -j ACCEPT
        echo -e "${GREEN}Added IPv4 forwarding rule (Internet → WG for established connections).${NC}"
    fi

    # IPv6 forwarding rules
    if ! ip6tables -C FORWARD -i "$WG_INTERFACE" -o "$NETWORK_DEVICE" -j ACCEPT 2>/dev/null; then
        ip6tables -A FORWARD -i "$WG_INTERFACE" -o "$NETWORK_DEVICE" -j ACCEPT
        echo -e "${GREEN}Added IPv6 forwarding rule (WG → Internet).${NC}"
    fi

    if ! ip6tables -C FORWARD -i "$NETWORK_DEVICE" -o "$WG_INTERFACE" -m state --state RELATED,ESTABLISHED -j ACCEPT 2>/dev/null; then
        ip6tables -A FORWARD -i "$NETWORK_DEVICE" -o "$WG_INTERFACE" -m state --state RELATED,ESTABLISHED -j ACCEPT
        echo -e "${GREEN}Added IPv6 forwarding rule (Internet → WG for established connections).${NC}"
    fi
}

configure_dns_and_icmp() {
    echo -e "${YELLOW}Configuring DNS and ICMP rules...${NC}"

    # ICMP rules for ping
    if ! iptables -C INPUT -p icmp --icmp-type echo-request -j ACCEPT 2>/dev/null; then
        iptables -A INPUT -p icmp --icmp-type echo-request -j ACCEPT
        echo -e "${GREEN}Added IPv4 ICMP rule (allow ping requests).${NC}"
    fi

    if ! iptables -C OUTPUT -p icmp --icmp-type echo-reply -j ACCEPT 2>/dev/null; then
        iptables -A OUTPUT -p icmp --icmp-type echo-reply -j ACCEPT
        echo -e "${GREEN}Added IPv4 ICMP rule (allow ping replies).${NC}"
    fi

    # ICMPv6 rules for ping6
    if ! ip6tables -C INPUT -p ipv6-icmp -j ACCEPT 2>/dev/null; then
        ip6tables -A INPUT -p ipv6-icmp -j ACCEPT
        echo -e "${GREEN}Added IPv6 ICMP rule (allow ping6).${NC}"
    fi

    # DNS rules
    if ! iptables -C INPUT -p udp --dport 53 -j ACCEPT 2>/dev/null; then
        iptables -A INPUT -p udp --dport 53 -j ACCEPT
        echo -e "${GREEN}Added IPv4 DNS rule (UDP).${NC}"
    fi

    if ! iptables -C INPUT -p tcp --dport 53 -j ACCEPT 2>/dev/null; then
        iptables -A INPUT -p tcp --dport 53 -j ACCEPT
        echo -e "${GREEN}Added IPv4 DNS rule (TCP).${NC}"
    fi

    # IPv6 DNS rules
    if ! ip6tables -C INPUT -p udp --dport 53 -j ACCEPT 2>/dev/null; then
        ip6tables -A INPUT -p udp --dport 53 -j ACCEPT
        echo -e "${GREEN}Added IPv6 DNS rule (UDP).${NC}"
    fi

    if ! ip6tables -C INPUT -p tcp --dport 53 -j ACCEPT 2>/dev/null; then
        ip6tables -A INPUT -p tcp --dport 53 -j ACCEPT
        echo -e "${GREEN}Added IPv6 DNS rule (TCP).${NC}"
    fi
}

# Apply Spamhaus blocklist from the Nym exit policy
apply_spamhaus_blocklist() {
    echo -e "${YELLOW}Applying Spamhaus blocklist...${NC}"

    # Create directory if not exists
    mkdir -p "$(dirname "$POLICY_FILE")"

    # Try to download the policy file
    echo -e "${YELLOW}Downloading exit policy from $EXIT_POLICY_LOCATION${NC}"
    if ! wget -q "$EXIT_POLICY_LOCATION" -O "$POLICY_FILE" 2>/dev/null; then
        echo -e "${RED}Failed to download exit policy. Using minimal blocklist.${NC}"

        # Create a minimal policy file with a few entries
        cat >"$POLICY_FILE" <<EOF
ExitPolicy reject 5.188.10.0/23:*
ExitPolicy reject 31.132.36.0/22:*
ExitPolicy reject 37.9.42.0/24:*
ExitPolicy reject 45.43.128.0/18:*
ExitPolicy reject *:*
EOF
    fi

    # Count and process rules
    total_rules=$(grep -c "^ExitPolicy reject" "$POLICY_FILE" | grep -v "\*:\*")
    echo -e "${YELLOW}Processing $total_rules blocklist rules...${NC}"

    # Extract and apply IP block rules
    grep "^ExitPolicy reject" "$POLICY_FILE" | grep -v "\*:\*" |
        while read -r line; do
            # Extract IP range
            ip_range=$(echo "$line" | sed -E 's/ExitPolicy reject ([^:]+):.*/\1/')

            # Apply rule if it's a valid IP range
            if [[ -n "$ip_range" ]]; then
                # Skip if the rule already exists
                if ! iptables -C "$NYM_CHAIN" -d "$ip_range" -j REJECT 2>/dev/null; then
                    iptables -A "$NYM_CHAIN" -d "$ip_range" -j REJECT
                fi

                # Apply IPv6 rules for IPv6 addresses
                if [[ "$ip_range" == *":"* ]] && ! ip6tables -C "$NYM_CHAIN" -d "$ip_range" -j REJECT 2>/dev/null; then
                    ip6tables -A "$NYM_CHAIN" -d "$ip_range" -j REJECT
                fi
            fi
        done

    echo -e "${GREEN}Blocklist applied successfully.${NC}"
}

add_default_reject_rule() {
    echo -e "${YELLOW}Adding default reject rule...${NC}"

    # First remove any existing plain reject rules (without specific destinations)
    iptables -D "$NYM_CHAIN" -j REJECT 2>/dev/null || true
    iptables -D "$NYM_CHAIN" -j REJECT --reject-with icmp-port-unreachable 2>/dev/null || true
    ip6tables -D "$NYM_CHAIN" -j REJECT 2>/dev/null || true
    ip6tables -D "$NYM_CHAIN" -j REJECT --reject-with icmp6-port-unreachable 2>/dev/null || true

    # Add the default catch-all reject rule (must be the last rule in the chain)
    iptables -A "$NYM_CHAIN" -j REJECT --reject-with icmp-port-unreachable
    ip6tables -A "$NYM_CHAIN" -j REJECT --reject-with icmp6-port-unreachable

    echo -e "${GREEN}Default reject rule added successfully.${NC}"
}

apply_port_allowlist() {
    echo -e "${YELLOW}Applying allowed ports...${NC}"

    # Dictionary of services and their ports
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
        ["GoogleVoice"]="19294"
        ["EnsimControlPanel"]="19638"
        ["Minecraft"]="25565"
        ["Steam"]="27000-27050"
        ["ElectrumSSL"]="50002"
        ["MOSH"]="60000-61000"
        ["Mumble"]="64738"
    )

    # Add TCP and UDP rules for each allowed port
    for service in "${!PORT_MAPPINGS[@]}"; do
        port="${PORT_MAPPINGS[$service]}"
        echo -e "${YELLOW}Adding rules for $service (Port: $port)${NC}"

        # Add both TCP and UDP rules for all services
        add_port_rules iptables "$port" "tcp"
        add_port_rules ip6tables "$port" "tcp"
        add_port_rules iptables "$port" "udp"
        add_port_rules ip6tables "$port" "udp"
    done

    add_default_reject_rule

    echo -e "${GREEN}Port allowlist applied successfully.${NC}"
}

safe_iptables_rule_remove() {
    local chain="$1"
    local table="${2:-filter}"
    local interface="$3"

    # Remove rule if it exists
    if iptables -t "$table" -C "$chain" -o "$interface" -j "$NYM_CHAIN" 2>/dev/null; then
        iptables -t "$table" -D "$chain" -o "$interface" -j "$NYM_CHAIN"
    fi
}

safe_ip6tables_rule_remove() {
    local chain="$1"
    local table="${2:-filter}"
    local interface="$3"

    # Remove rule if it exists
    if ip6tables -t "$table" -C "$chain" -o "$interface" -j "$NYM_CHAIN" 2>/dev/null; then
        ip6tables -t "$table" -D "$chain" -o "$interface" -j "$NYM_CHAIN"
    fi
}

clear_rules() {
    echo -e "${YELLOW}Clearing Nym exit policy rules...${NC}"

    # Flush all rules in the NYM-EXIT chain
    iptables -F "$NYM_CHAIN" 2>/dev/null || true
    ip6tables -F "$NYM_CHAIN" 2>/dev/null || true

    # Remove the chain from FORWARD if it exists
    iptables -D FORWARD -o "$WG_INTERFACE" -j "$NYM_CHAIN" 2>/dev/null || true
    ip6tables -D FORWARD -o "$WG_INTERFACE" -j "$NYM_CHAIN" 2>/dev/null || true

    # Delete the chains
    iptables -X "$NYM_CHAIN" 2>/dev/null || true
    ip6tables -X "$NYM_CHAIN" 2>/dev/null || true

    echo -e "${GREEN}Nym exit policy rules cleared successfully.${NC}"
}

remove_duplicate_rules() {
    local interface="$1"

    if [[ -z "$interface" ]]; then
        echo -e "${RED}Error: No interface specified. Usage: $0 remove-duplicates <interface>${NC}" >&2
        exit 1
    fi

    echo -e "${YELLOW}Detecting and removing duplicate rules for $interface...${NC}"

    # Verbose duplicate rule detection for IPv4
    echo -e "${YELLOW}Checking IPv4 duplicate rules:${NC}"
    iptables-save | grep -E "(-A FORWARD|-A $NYM_CHAIN)" | grep "$interface" | sort | uniq -d && {
        echo -e "${RED}Duplicate IPv4 rules found! Removing...${NC}"
        # Remove duplicates by saving unique rules
        iptables-save | grep -E "(-A FORWARD|-A $NYM_CHAIN)" | grep "$interface" | sort | uniq | while read -r rule; do
            # Carefully remove duplicates
            full_rule=$(echo "$rule" | sed 's/^-A/iptables -D/')
            eval "$full_rule" 2>/dev/null
        done
    } || echo -e "${GREEN}No duplicate IPv4 rules found.${NC}"

    # Verbose duplicate rule detection for IPv6
    echo -e "${YELLOW}Checking IPv6 duplicate rules:${NC}"
    ip6tables-save | grep -E "(-A FORWARD|-A $NYM_CHAIN)" | grep "$interface" | sort | uniq -d && {
        echo -e "${RED}Duplicate IPv6 rules found! Removing...${NC}"
        # Remove duplicates by saving unique rules
        ip6tables-save | grep -E "(-A FORWARD|-A $NYM_CHAIN)" | grep "$interface" | sort | uniq | while read -r rule; do
            # Carefully remove duplicates
            full_rule=$(echo "$rule" | sed 's/^-A/ip6tables -D/')
            eval "$full_rule" 2>/dev/null
        done
    } || echo -e "${GREEN}No duplicate IPv6 rules found.${NC}"

    # Additional verification
    echo -e "\n${YELLOW}Rule verification:${NC}"
    echo "IPv4 Rules:"
    iptables -L FORWARD -v -n | grep "$interface"
    echo "IPv6 Rules:"
    ip6tables -L FORWARD -v -n | grep "$interface"

    echo -e "${GREEN}Duplicate rule removal process completed.${NC}"
}

save_rules() {
    echo -e "${YELLOW}Saving iptables rules to make them persistent...${NC}"

    if [ -d "/etc/iptables" ]; then
        # For Debian/Ubuntu with iptables-persistent
        iptables-save >/etc/iptables/rules.v4
        ip6tables-save >/etc/iptables/rules.v6
        echo -e "${GREEN}Rules saved to /etc/iptables/rules.v4 and /etc/iptables/rules.v6${NC}"
    else
        # Fallback method
        iptables-save >/etc/iptables.rules
        ip6tables-save >/etc/ip6tables.rules
        echo -e "${GREEN}Rules saved to /etc/iptables.rules and /etc/ip6tables.rules${NC}"

        # Add loading script to rc.local if it doesn't exist
        if [ ! -f "/etc/network/if-pre-up.d/iptables" ]; then
            cat >/etc/network/if-pre-up.d/iptables <<EOF
#!/bin/sh
iptables-restore < /etc/iptables.rules
ip6tables-restore < /etc/ip6tables.rules
EOF
            chmod +x /etc/network/if-pre-up.d/iptables
            echo -e "${GREEN}Created pre-up script to load rules at boot${NC}"
        fi
    fi
}

show_status() {
    echo -e "${YELLOW}Nym Exit Policy Status:${NC}"
    echo -e "${YELLOW}----------------------${NC}"

    # Network information
    echo -e "${GREEN}Network Device:${NC} $NETWORK_DEVICE"
    echo -e "${GREEN}Wireguard Interface:${NC} $WG_INTERFACE"

    # Interface check
    if ! ip link show "$WG_INTERFACE" &>/dev/null; then
        echo -e "${RED}WARNING: Wireguard interface $WG_INTERFACE not found!${NC}"
        return 1
    fi

    # Interface details
    echo -e "\n${YELLOW}Interface Details:${NC}"
    ip link show "$WG_INTERFACE"

    # IP Addresses
    echo -e "\n${YELLOW}IP Addresses:${NC}"
    ip -4 addr show dev "$WG_INTERFACE"
    ip -6 addr show dev "$WG_INTERFACE"

    # Iptables Chain Status
    echo -e "\n${YELLOW}Iptables Chains:${NC}"
    {
        echo "IPv4 Chain:"
        iptables -L "$NYM_CHAIN" -n -v
        echo -e "\nIPv6 Chain:"
        ip6tables -L "$NYM_CHAIN" -n -v
    } || echo "One or both chains not found"

    # Forwarding Status
    echo -e "\n${YELLOW}IP Forwarding:${NC}"
    echo "IPv4: $(cat /proc/sys/net/ipv4/ip_forward)"
    echo "IPv6: $(cat /proc/sys/net/ipv6/conf/all/forwarding)"
}

test_connectivity() {
    echo -e "${YELLOW}Testing connectivity through $WG_INTERFACE...${NC}"

    # More comprehensive interface check
    interface_info=$(ip link show "$WG_INTERFACE" 2>/dev/null)

    if [ -z "$interface_info" ]; then
        echo -e "${RED}Interface $WG_INTERFACE not found!${NC}"
        return 1
    fi

    # Check for multiple possible interface states
    if ! echo "$interface_info" | grep -qE "state (UP|UNKNOWN|DORMANT)"; then
        echo -e "${RED}Interface $WG_INTERFACE is not in an active state!${NC}"
        echo "$interface_info"
        return 1
    fi

    # Detailed interface information
    echo -e "${GREEN}Interface Details:${NC}"
    echo "$interface_info"

    # Get IP addresses with more robust method
    ipv4_address=$(ip -4 addr show dev "$WG_INTERFACE" | grep -oP '(?<=inet\s)\d+\.\d+\.\d+\.\d+/\d+' | cut -d'/' -f1 | head -n1)
    ipv6_address=$(ip -6 addr show dev "$WG_INTERFACE" scope global | grep -oP '(?<=inet6\s)[0-9a-f:]+/\d+' | cut -d'/' -f1 | head -n1)

    echo -e "${GREEN}IPv4 Address:${NC} ${ipv4_address:-Not found}"
    echo -e "${GREEN}IPv6 Address:${NC} ${ipv6_address:-Not found}"

    # Connectivity tests
    if [[ -n "$ipv4_address" ]]; then
        echo -e "${YELLOW}Testing IPv4 connectivity from $ipv4_address...${NC}"

        # Ping test
        if timeout 5 ping -c 3 -I "$ipv4_address" 8.8.8.8 >/dev/null 2>&1; then
            echo -e "${GREEN}IPv4 connectivity to 8.8.8.8: Success${NC}"
        else
            echo -e "${RED}IPv4 connectivity to 8.8.8.8: Failed${NC}"
        fi

        # DNS resolution test
        if timeout 5 ping -c 3 -I "$ipv4_address" google.com >/dev/null 2>&1; then
            echo -e "${GREEN}IPv4 DNS resolution: Success${NC}"
        else
            echo -e "${RED}IPv4 DNS resolution: Failed${NC}"
        fi

        # HTTP(S) connectivity test
        if command -v curl &>/dev/null; then
            if timeout 5 curl -s --interface "$ipv4_address" -o /dev/null -w "%{http_code}" https://www.google.com | grep -q "200"; then
                echo -e "${GREEN}IPv4 HTTPS connectivity: Success${NC}"
            else
                echo -e "${RED}IPv4 HTTPS connectivity: Failed${NC}"
            fi
        fi
    else
        echo -e "${RED}No IPv4 address configured on $WG_INTERFACE${NC}"
    fi

    # Similar tests for IPv6 if available
    if [[ -n "$ipv6_address" ]]; then
        echo -e "${YELLOW}Testing IPv6 connectivity from $ipv6_address...${NC}"

        if timeout 5 ping6 -c 3 -I "$ipv6_address" 2001:4860:4860::8888 >/dev/null 2>&1; then
            echo -e "${GREEN}IPv6 connectivity to Google DNS: Success${NC}"
        else
            echo -e "${RED}IPv6 connectivity to Google DNS: Failed${NC}"
        fi

        if timeout 5 ping6 -c 3 -I "$ipv6_address" google.com >/dev/null 2>&1; then
            echo -e "${GREEN}IPv6 DNS resolution: Success${NC}"
        else
            echo -e "${RED}IPv6 DNS resolution: Failed${NC}"
        fi

        if command -v curl &>/dev/null; then
            if timeout 5 curl -s --interface "$ipv6_address" -o /dev/null -w "%{http_code}" https://www.google.com | grep -q "200"; then
                echo -e "${GREEN}IPv6 HTTPS connectivity: Success${NC}"
            else
                echo -e "${RED}IPv6 HTTPS connectivity: Failed${NC}"
            fi
        fi
    else
        echo -e "${YELLOW}No IPv6 address configured on $WG_INTERFACE${NC}"
    fi

    echo -e "${GREEN}Connectivity tests completed.${NC}"
}

main() {
    # Check for root privileges
    if [ "$(id -u)" -ne 0 ]; then
        echo -e "${RED}This script must be run as root${NC}" >&2
        exit 1
    fi

    # Parse command-line arguments
    case "$1" in
    install)
        install_dependencies
        configure_ip_forwarding
        create_nym_chain
        setup_nat_rules
        configure_dns_and_icmp
        apply_spamhaus_blocklist
        apply_port_allowlist
        save_rules
        echo -e "${GREEN}Nym exit policy installed successfully.${NC}"
        ;;
    status)
        show_status
        ;;
    test)
        test_connectivity
        ;;
    clear)
        clear_rules
        echo -e "${GREEN}Nym exit policy rules cleared.${NC}"
        ;;
    remove-duplicates)
        remove_duplicate_rules "$2"
        ;;
    help | --help | -h)
        echo "Usage: $0 [command]"
        echo ""
        echo "Commands:"
        echo "  install           Install and configure Nym exit policy"
        echo "  status            Show current Nym exit policy status"
        echo "  test              Test connectivity through Wireguard interface"
        echo "  clear             Remove all Nym exit policy rules"
        echo "  remove-duplicates <interface>  Remove duplicate iptables rules for an interface"
        echo "  help              Show this help message"
        ;;
    *)
        echo -e "${RED}Invalid command. Use '$0 help' for usage information.${NC}" >&2
        exit 1
        ;;
    esac
}

main "$@"