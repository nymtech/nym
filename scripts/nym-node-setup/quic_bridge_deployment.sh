    #!/bin/bash
# Nym QUIC Bridge Deployment Helper Script
# This script provides network configuration and troubleshooting tools for Nym QUIC bridges

network_device=$(ip route show default | awk '/default/ {print $5}')
wg_tunnel_interface="nymwg"

BRIDGE_CONFIG_DIR="/opt/nym-bridge"
BRIDGE_KEYS_DIR="$BRIDGE_CONFIG_DIR/keys"
BRIDGE_CONFIG="$BRIDGE_CONFIG_DIR/bridges.toml"
CLIENT_PARAMS="$BRIDGE_CONFIG_DIR/client_bridge_params.json"
BRIDGE_BINARY="/usr/local/bin/nym-bridge"


if ! dpkg -s iptables-persistent >/dev/null 2>&1; then
    echo "Installing iptables-persistent..."
    sudo apt-get update
    sudo apt-get install -y iptables-persistent
else
    echo "iptables-persistent is already installed."
fi

fetch_and_display_ipv6() {
    ipv6_address=$(ip -6 addr show "$network_device" scope global | grep inet6 | awk '{print $2}')
    if [[ -z "$ipv6_address" ]]; then
        echo "No global IPv6 address found on $network_device."
    else
        echo "IPv6 address on $network_device: $ipv6_address"
    fi
}

fetch_wg_ipv6_address() {
    ipv6_global_address=$(ip -6 addr show "$wg_tunnel_interface" scope global | grep inet6 | awk '{print $2}' | head -n 1)

    if [[ -z "$ipv6_global_address" ]]; then
        echo "No globally routable IPv6 address found on $wg_tunnel_interface. Please configure IPv6 or check your network settings."
        exit 1
    else
        echo "Using IPv6 address: $ipv6_global_address"
    fi
}

adjust_ip_forwarding() {
    ipv6_forwarding_setting="net.ipv6.conf.all.forwarding=1"
    ipv4_forwarding_setting="net.ipv4.ip_forward=1"

    # Remove duplicate entries for these settings from the file
    sudo sed -i "/^net.ipv6.conf.all.forwarding=/d" /etc/sysctl.conf
    sudo sed -i "/^net.ipv4.ip_forward=/d" /etc/sysctl.conf

    echo "$ipv6_forwarding_setting" | sudo tee -a /etc/sysctl.conf
    echo "$ipv4_forwarding_setting" | sudo tee -a /etc/sysctl.conf

    sudo sysctl -p /etc/sysctl.conf

    echo "IP forwarding enabled for IPv4 and IPv6."
}

apply_bridge_iptables_rules() {
    echo "Applying iptables rules for QUIC bridge ($wg_tunnel_interface)..."
    sleep 1

    # INPUT rules - allow incoming connections TO the bridge from WireGuard clients
    # CRITICAL: This allows mobile clients to reach the bandwidth controller at 10.1.0.1:51830
    sudo iptables -I INPUT -i "$wg_tunnel_interface" -j ACCEPT
    sudo ip6tables -I INPUT -i "$wg_tunnel_interface" -j ACCEPT

    # NAT rules - for outbound traffic masquerading
    sudo iptables -t nat -A POSTROUTING -o "$network_device" -j MASQUERADE
    sudo ip6tables -t nat -A POSTROUTING -o "$network_device" -j MASQUERADE

    # FORWARD rules - allow traffic through the bridge
    sudo iptables -A FORWARD -i "$wg_tunnel_interface" -o "$network_device" -j ACCEPT
    sudo iptables -A FORWARD -i "$network_device" -o "$wg_tunnel_interface" -m state --state RELATED,ESTABLISHED -j ACCEPT
    sudo ip6tables -A FORWARD -i "$wg_tunnel_interface" -o "$network_device" -j ACCEPT
    sudo ip6tables -A FORWARD -i "$network_device" -o "$wg_tunnel_interface" -m state --state RELATED,ESTABLISHED -j ACCEPT

    # Save rules
    sudo iptables-save | sudo tee /etc/iptables/rules.v4
    sudo ip6tables-save | sudo tee /etc/iptables/rules.v6
    
    echo "Iptables rules applied successfully for QUIC bridge (including INPUT rules for bandwidth controller)."
}

check_bridge_iptables() {
    echo "Inspecting iptables rules for QUIC bridge ($wg_tunnel_interface)..."
    echo "---------------------------------------"
    echo "IPv4 INPUT rules (for bandwidth controller):"
    iptables -L INPUT -v -n | grep -E "$wg_tunnel_interface|Chain INPUT" | head -20
    echo "---------------------------------------"
    echo "IPv4 FORWARD rules:"
    iptables -L FORWARD -v -n | awk -v dev="$wg_tunnel_interface" '/^Chain FORWARD/ || $0 ~ dev || $0 ~ "ufw-reject-forward"'
    echo "---------------------------------------"
    echo "IPv6 INPUT rules (for bandwidth controller):"
    ip6tables -L INPUT -v -n | grep -E "$wg_tunnel_interface|Chain INPUT" | head -20
    echo "---------------------------------------"
    echo "IPv6 FORWARD rules:"
    ip6tables -L FORWARD -v -n | awk -v dev="$wg_tunnel_interface" '/^Chain FORWARD/ || $0 ~ dev || $0 ~ "ufw6-reject-forward"'
}

remove_duplicate_bridge_rules() {
    local script_name=$(basename "$0")

    echo "Removing duplicate iptables rules for $wg_tunnel_interface..."

    iptables-save | grep "$wg_tunnel_interface" | while read -r line; do
        sudo iptables -D ${line#-A } 2>/dev/null || echo "Failed to delete rule: $line"
    done

    ip6tables-save | grep "$wg_tunnel_interface" | while read -r line; do
        sudo ip6tables -D ${line#-A } 2>/dev/null || echo "Failed to delete rule: $line"
    done

    echo "Duplicates removed for $wg_tunnel_interface."
    echo "!!IMPORTANT!! You need to now reapply the iptables rules."
    echo "Run: ./$script_name apply_bridge_iptables_rules"
}

configure_dns_and_icmp() {
    echo "Allowing ICMP (ping)..."
    sudo iptables -A INPUT -p icmp --icmp-type echo-request -j ACCEPT
    sudo iptables -A OUTPUT -p icmp --icmp-type echo-reply -j ACCEPT
    sudo ip6tables -A INPUT -p ipv6-icmp -j ACCEPT
    sudo ip6tables -A OUTPUT -p ipv6-icmp -j ACCEPT

    echo "Allowing DNS over UDP (port 53)..."
    sudo iptables -A INPUT -p udp --dport 53 -j ACCEPT
    sudo ip6tables -A INPUT -p udp --dport 53 -j ACCEPT

    echo "Allowing DNS over TCP (port 53)..."
    sudo iptables -A INPUT -p tcp --dport 53 -j ACCEPT
    sudo ip6tables -A INPUT -p tcp --dport 53 -j ACCEPT

    echo "Saving iptables rules..."
    sudo iptables-save | sudo tee /etc/iptables/rules.v4
    sudo ip6tables-save | sudo tee /etc/iptables/rules.v6

    echo "DNS and ICMP configuration completed."
}

check_ipv6_ipv4_forwarding() {
    result_ipv4=$(cat /proc/sys/net/ipv4/ip_forward)
    result_ipv6=$(cat /proc/sys/net/ipv6/conf/all/forwarding)
    echo "IPv4 forwarding is $([ "$result_ipv4" == "1" ] && echo "enabled" || echo "not enabled")."
    echo "IPv6 forwarding is $([ "$result_ipv6" == "1" ] && echo "enabled" || echo "not enabled")."
}

check_ip_routing() {
    echo "IPv4 routing table:"
    ip route
    echo "---------------------------------------"
    echo "IPv6 routing table:"
    ip -6 route
}

perform_pings() {
    echo "Performing IPv4 ping to google.com..."
    ping -c 4 google.com
    echo "---------------------------------------"
    echo "Performing IPv6 ping to google.com..."
    ping6 -c 4 google.com
}

test_bridge_connectivity() {
    local green="\033[0;32m"
    local reset="\033[0m"
    local red="\033[0;31m"
    local yellow="\033[0;33m"

    sleep 1
    echo 
    echo -e "${yellow}Testing QUIC bridge connectivity on $wg_tunnel_interface...${reset}"
    echo -e "${yellow}If these tests succeed, it confirms the bridge can reach the outside world via IPv4 and IPv6.${reset}"
    echo -e "${yellow}However, external clients may experience different connectivity to your bridge.${reset}"

    ipv4_address=$(ip addr show "$wg_tunnel_interface" | awk '/inet / {print $2}' | cut -d'/' -f1)
    ipv6_address=$(ip addr show "$wg_tunnel_interface" | awk '/inet6 / && $2 !~ /^fe80/ {print $2}' | cut -d'/' -f1)

    if [[ -z "$ipv4_address" && -z "$ipv6_address" ]]; then
        echo -e "${red}No IP address found on $wg_tunnel_interface. Unable to test connectivity.${reset}"
        echo -e "${red}Please verify your bridge configuration and ensure the interface is up.${reset}"
        return 1
    fi
    
    if [[ -n "$ipv4_address" ]]; then
        echo 
        echo -e "------------------------------------"
        echo -e "Detected IPv4 address: $ipv4_address"
        echo -e "Testing IPv4 connectivity..."
        echo 

        if ping -c 1 -I "$ipv4_address" google.com >/dev/null 2>&1; then
            echo -e "${green}IPv4 connectivity is working. Fetching test data...${reset}"
            joke=$(curl -s -H "Accept: application/json" --interface "$ipv4_address" https://icanhazdadjoke.com/ | jq -r .joke)
            [[ -n "$joke" && "$joke" != "null" ]] && echo -e "${green}IPv4 test joke: $joke${reset}" || echo -e "${red}Failed to fetch test data via IPv4.${reset}"
        else
            echo -e "${red}IPv4 connectivity is not working for $wg_tunnel_interface. Verify your routing and NAT settings.${reset}"
        fi
    else
        echo -e "${yellow}No IPv4 address found on $wg_tunnel_interface. Skipping IPv4 test.${reset}"
    fi

    if [[ -n "$ipv6_address" ]]; then
        echo 
        echo -e "------------------------------------"
        echo -e "Detected IPv6 address: $ipv6_address"
        echo -e "Testing IPv6 connectivity..."
        echo 

        if ping6 -c 1 -I "$ipv6_address" google.com >/dev/null 2>&1; then
            echo -e "${green}IPv6 connectivity is working. Fetching test data...${reset}"
            joke=$(curl -s -H "Accept: application/json" --interface "$ipv6_address" https://icanhazdadjoke.com/ | jq -r .joke)
            [[ -n "$joke" && "$joke" != "null" ]] && echo -e "${green}IPv6 test joke: $joke${reset}" || echo -e "${red}Failed to fetch test data via IPv6.${reset}"
        else
            echo -e "${red}IPv6 connectivity is not working for $wg_tunnel_interface. Verify your routing and NAT settings.${reset}"
        fi
    else
        echo -e "${yellow}No IPv6 address found on $wg_tunnel_interface. Skipping IPv6 test.${reset}"
    fi

    echo -e "${green}Connectivity testing completed for $wg_tunnel_interface.${reset}"
    echo -e "------------------------------------"

    sleep 2
    echo
    echo 
    echo -e "${yellow}### Bridge Connectivity Testing Recommendations ###${reset}"
    echo -e "${yellow}- Test UDP connectivity on port 51822 (used for Nym QUIC/WireGuard)${reset}"
    echo -e "${yellow}  From another machine: echo 'test message' | nc -u <your-ip-address> 51822${reset}"
    echo -e "${yellow}- Test bandwidth controller access on port 51830:${reset}"
    echo -e "${yellow}  From inside the WireGuard tunnel: curl http://10.1.0.1:51830${reset}"
    echo -e "${yellow}- If connectivity issues persist, check port forwarding and firewall rules${reset}"
    echo 
}

check_bridge_service_status() {
    echo "Checking nym-bridge service status..."
    systemctl status nym-bridge.service --no-pager
    echo "---------------------------------------"
    echo "Checking nym-node service status..."
    systemctl status nym-node.service --no-pager
}

show_bridge_logs() {
    local lines=${1:-50}
    echo "Showing last $lines lines of nym-bridge logs..."
    journalctl -u nym-bridge.service -n "$lines" --no-pager
}

check_bridge_installation() {
    local green="\033[0;32m"
    local reset="\033[0m"
    local red="\033[0;31m"
    local yellow="\033[0;33m"

    echo -e "${yellow}=== Nym QUIC Bridge Installation Status ===${reset}"
    echo ""

    # Check binary
    if [[ -f "$BRIDGE_BINARY" ]]; then
        echo -e "${green}✓ Bridge binary found: $BRIDGE_BINARY${reset}"
        bridge_version=$($BRIDGE_BINARY --version 2>/dev/null | head -1 || echo "Unable to determine version")
        echo "  Version: $bridge_version"
    else
        echo -e "${red}✗ Bridge binary not found at $BRIDGE_BINARY${reset}"
    fi
    echo ""

    # Check configuration directory
    if [[ -d "$BRIDGE_CONFIG_DIR" ]]; then
        echo -e "${green}✓ Configuration directory exists: $BRIDGE_CONFIG_DIR${reset}"
    else
        echo -e "${red}✗ Configuration directory not found: $BRIDGE_CONFIG_DIR${reset}"
    fi
    echo ""

    # Check keys directory
    if [[ -d "$BRIDGE_KEYS_DIR" ]]; then
        echo -e "${green}✓ Keys directory exists: $BRIDGE_KEYS_DIR${reset}"
        key_count=$(ls -1 "$BRIDGE_KEYS_DIR"/*.pem 2>/dev/null | wc -l)
        echo "  Keys found: $key_count"
    else
        echo -e "${red}✗ Keys directory not found: $BRIDGE_KEYS_DIR${reset}"
    fi
    echo ""

    # Check configuration files
    if [[ -f "$BRIDGE_CONFIG" ]]; then
        echo -e "${green}✓ Bridge config found: $BRIDGE_CONFIG${reset}"
    else
        echo -e "${red}✗ Bridge config not found: $BRIDGE_CONFIG${reset}"
    fi

    if [[ -f "$CLIENT_PARAMS" ]]; then
        echo -e "${green}✓ Client params found: $CLIENT_PARAMS${reset}"
    else
        echo -e "${red}✗ Client params not found: $CLIENT_PARAMS${reset}"
    fi
    echo ""

    # Check services
    echo -e "${yellow}Service Status:${reset}"
    if systemctl is-active --quiet nym-bridge.service; then
        echo -e "${green}✓ nym-bridge service is running${reset}"
    else
        echo -e "${red}✗ nym-bridge service is not running${reset}"
    fi

    if systemctl is-active --quiet nym-node.service; then
        echo -e "${green}✓ nym-node service is running${reset}"
    else
        echo -e "${red}✗ nym-node service is not running${reset}"
    fi
    echo ""
}

show_bridge_config() {
    echo "=== Bridge Configuration ==="
    echo ""
    
    if [[ -f "$BRIDGE_CONFIG" ]]; then
        echo "Bridge config ($BRIDGE_CONFIG):"
        echo "---------------------------------------"
        cat "$BRIDGE_CONFIG"
        echo ""
    else
        echo "Bridge config not found at $BRIDGE_CONFIG"
    fi

    if [[ -f "$CLIENT_PARAMS" ]]; then
        echo "Client parameters ($CLIENT_PARAMS):"
        echo "---------------------------------------"
        cat "$CLIENT_PARAMS"
        echo ""
    else
        echo "Client parameters not found at $CLIENT_PARAMS"
    fi
}

show_bridge_keys() {
    local green="\033[0;32m"
    local reset="\033[0m"
    local red="\033[0;31m"
    local yellow="\033[0;33m"

    echo -e "${yellow}=== Bridge Keys Information ===${reset}"
    echo ""

    if [[ ! -d "$BRIDGE_KEYS_DIR" ]]; then
        echo -e "${red}Keys directory not found: $BRIDGE_KEYS_DIR${reset}"
        return 1
    fi

    echo "Keys directory: $BRIDGE_KEYS_DIR"
    echo "---------------------------------------"
    
    # List all key files
    if ls -1 "$BRIDGE_KEYS_DIR"/*.pem >/dev/null 2>&1; then
        for key_file in "$BRIDGE_KEYS_DIR"/*.pem; do
            key_name=$(basename "$key_file")
            echo -e "${green}Key file: $key_name${reset}"
            
            # If it's a public key, show the content
            if [[ "$key_name" == *"_bridge_identity.pem" ]]; then
                echo "  Type: ED25519 Bridge Identity (Private)"
                echo "  Path: $key_file"
                
                # Extract and show public key
                if command -v openssl >/dev/null 2>&1; then
                    echo -e "${yellow}  Public key (base64):${reset}"
                    openssl pkey -in "$key_file" -pubout 2>/dev/null | grep -v "\---" | base64 -d | tail -c 32 | base64
                fi
            fi
            echo ""
        done
    else
        echo -e "${red}No key files found in $BRIDGE_KEYS_DIR${reset}"
    fi
}

show_bridge_info() {
    local green="\033[0;32m"
    local reset="\033[0m"
    local yellow="\033[0;33m"

    echo -e "${yellow}=== Nym QUIC Bridge Information ===${reset}"
    echo ""

    # Network interfaces
    echo -e "${yellow}Network Configuration:${reset}"
    echo "Primary network device: $network_device"
    echo "WireGuard interface: $wg_tunnel_interface"
    
    # Show IP addresses
    echo ""
    echo "IPv4 addresses:"
    ip -4 addr show | grep inet | awk '{print "  " $2 " on " $NF}'
    
    echo ""
    echo "IPv6 addresses:"
    ip -6 addr show scope global | grep inet6 | awk '{print "  " $2 " on " $NF}'
    
    echo ""
    echo -e "${yellow}Bridge Paths:${reset}"
    echo "Configuration: $BRIDGE_CONFIG_DIR"
    echo "Keys: $BRIDGE_KEYS_DIR"
    echo "Binary: $BRIDGE_BINARY"
    
    echo ""
    echo -e "${yellow}Important Commands:${reset}"
    echo "  Check bridge status:    systemctl status nym-bridge"
    echo "  Check nym-node status:  systemctl status nym-node"
    echo "  View bridge logs:       journalctl -u nym-bridge -f"
    echo "  View nym-node logs:     journalctl -u nym-node -f"
    echo ""
}

verify_bridge_prerequisites() {
    local green="\033[0;32m"
    local reset="\033[0m"
    local red="\033[0;31m"
    local yellow="\033[0;33m"

    echo -e "${yellow}=== Verifying Bridge Prerequisites ===${reset}"
    echo ""

    local all_good=true

    # Check IP forwarding
    ipv4_forward=$(cat /proc/sys/net/ipv4/ip_forward)
    ipv6_forward=$(cat /proc/sys/net/ipv6/conf/all/forwarding)

    if [[ "$ipv4_forward" == "1" ]]; then
        echo -e "${green}✓ IPv4 forwarding enabled${reset}"
    else
        echo -e "${red}✗ IPv4 forwarding disabled${reset}"
        echo "  Fix: Run 'nym-bridge-helper adjust_ip_forwarding'"
        all_good=false
    fi

    if [[ "$ipv6_forward" == "1" ]]; then
        echo -e "${green}✓ IPv6 forwarding enabled${reset}"
    else
        echo -e "${red}✗ IPv6 forwarding disabled${reset}"
        echo "  Fix: Run 'nym-bridge-helper adjust_ip_forwarding'"
        all_good=false
    fi

    # Check iptables-persistent
    if dpkg -s iptables-persistent >/dev/null 2>&1; then
        echo -e "${green}✓ iptables-persistent installed${reset}"
    else
        echo -e "${red}✗ iptables-persistent not installed${reset}"
        echo "  Fix: This script will auto-install on first run"
        all_good=false
    fi

    # Check required packages
    for pkg in openssl jq curl wg; do
        if command -v "$pkg" >/dev/null 2>&1; then
            echo -e "${green}✓ $pkg installed${reset}"
        else
            echo -e "${red}✗ $pkg not installed${reset}"
            if [[ "$pkg" == "wg" ]]; then
                echo "  Install: sudo apt install wireguard-tools"
            fi
            all_good=false
        fi
    done

    echo ""
    if [[ "$all_good" == true ]]; then
        echo -e "${green}All prerequisites satisfied!${reset}"
    else
        echo -e "${yellow}Some prerequisites need attention. See above for fixes.${reset}"
    fi
    echo ""
}

generate_bridge_keys() {
    local green="\033[0;32m"
    local reset="\033[0m"
    local red="\033[0;31m"
    local yellow="\033[0;33m"

    echo -e "${yellow}=== Generating Bridge Keys ===${reset}"
    echo ""

    # Create directories
    sudo mkdir -p "$BRIDGE_CONFIG_DIR"
    sudo mkdir -p "$BRIDGE_KEYS_DIR"
    sudo chmod 700 "$BRIDGE_KEYS_DIR"

    # Generate ED25519 private key
    local key_file="$BRIDGE_KEYS_DIR/ed25519_bridge_identity.pem"
    
    if [[ -f "$key_file" ]]; then
        echo -e "${yellow}Warning: Key file already exists at $key_file${reset}"
        read -p "Overwrite existing key? (yes/no): " confirm
        if [[ "$confirm" != "yes" ]]; then
            echo "Aborted. Keeping existing key."
            return 1
        fi
    fi

    echo "Generating ED25519 key..."
    sudo openssl genpkey -algorithm ED25519 -out "$key_file"
    sudo chmod 600 "$key_file"
    
    echo -e "${green}✓ Bridge key generated at $key_file${reset}"
    
    # Extract and display public key
    echo ""
    echo "Extracting public key..."
    pubkey=$(sudo openssl pkey -in "$key_file" -pubout 2>/dev/null | grep -v "\---" | base64 -d | tail -c 32 | base64)
    echo -e "${green}Public key (base64): $pubkey${reset}"
    
    echo ""
    echo -e "${yellow}Next steps:${reset}"
    echo "1. Run 'nym-bridge-helper create_client_params' to generate client parameters"
    echo "2. Run 'nym-bridge-helper create_bridge_config' to create bridge configuration"
}

create_client_params() {
    local green="\033[0;32m"
    local reset="\033[0m"
    local red="\033[0;31m"
    local yellow="\033[0;33m"

    echo -e "${yellow}=== Creating Client Bridge Parameters ===${reset}"
    echo ""

    # Check if key exists
    local key_file="$BRIDGE_KEYS_DIR/ed25519_bridge_identity.pem"
    if [[ ! -f "$key_file" ]]; then
        echo -e "${red}Error: Bridge key not found at $key_file${reset}"
        echo "Run 'nym-bridge-helper generate_bridge_keys' first"
        return 1
    fi

    # Get forward address
    read -p "Enter forward address (e.g., <IPv4>:51822, can be found by running 'curl -6 https://ifconfig.co/ip'): " forward_addr
    if [[ -z "$forward_addr" ]]; then
        echo -e "${red}Error: Forward address is required${reset}"
        return 1
    fi

    # Extract public key
    echo "Extracting public key..."
    pubkey=$(sudo openssl pkey -in "$key_file" -pubout 2>/dev/null | grep -v "\---" | base64 -d | tail -c 32 | base64)

    # Create client params JSON
    echo "Creating client parameters file..."
    sudo tee "$CLIENT_PARAMS" > /dev/null <<EOF
{
  "ed25519_bridge_identity": "$pubkey",
  "forward_address": "$forward_addr",
  "endpoint": {
    "Quic": {
      "host": "$forward_addr"
    }
  }
}
EOF

    sudo chmod 644 "$CLIENT_PARAMS"
    
    echo -e "${green}✓ Client parameters created at $CLIENT_PARAMS${reset}"
    echo ""
    echo "Content:"
    cat "$CLIENT_PARAMS"
}

create_bridge_config() {
    local green="\033[0;32m"
    local reset="\033[0m"
    local red="\033[0;31m"
    local yellow="\033[0;33m"

    echo -e "${yellow}=== Creating Bridge Configuration ===${reset}"
    echo ""

    # Get configuration parameters
    read -p "Enter listening address (press enter for default: 0.0.0.0): " listen_addr
    listen_addr=${listen_addr:-0.0.0.0}

    read -p "Enter listening port (press enter for default: 51822): " listen_port
    listen_port=${listen_port:-51822}

    read -p "Enter tunnel device name (press enter for default: nymwg): " tunnel_dev
    tunnel_dev=${tunnel_dev:-nymwg}

    read -p "Enter tunnel IPv4 address (press enter for default: 10.1.0.1/24): " tunnel_ipv4
    tunnel_ipv4=${tunnel_ipv4:-10.1.0.1/24}

    read -p "Enter tunnel IPv6 address (optional, can be found by running 'curl -6 https://ifconfig.co/ip', press enter to skip): " tunnel_ipv6

    read -p "Enter WireGuard private key (or press enter to generate): " wg_privkey
    if [[ -z "$wg_privkey" ]]; then
        echo "Generating WireGuard private key..."
        wg_privkey=$(wg genkey)
        wg_pubkey=$(echo "$wg_privkey" | wg pubkey)
        echo -e "${green}Generated WireGuard public key: $wg_pubkey${reset}"
    fi

    # Create bridges.toml
    echo "Creating bridge configuration..."
    sudo tee "$BRIDGE_CONFIG" > /dev/null <<EOF
# Nym QUIC Bridge Configuration

[[bridges]]
# Listening address and port for the bridge
listening_address = "$listen_addr:$listen_port"

# WireGuard tunnel configuration
tunnel_device_name = "$tunnel_dev"
tunnel_device_address = "$tunnel_ipv4"
EOF

    if [[ -n "$tunnel_ipv6" ]]; then
        echo "tunnel_device_ipv6_address = \"$tunnel_ipv6\"" | sudo tee -a "$BRIDGE_CONFIG" > /dev/null
    fi

    sudo tee -a "$BRIDGE_CONFIG" > /dev/null <<EOF

# Bridge identity key
bridge_identity_private_key_file = "$BRIDGE_KEYS_DIR/ed25519_bridge_identity.pem"

# WireGuard private key
wireguard_private_key = "$wg_privkey"

# Additional settings
bandwidth_controller_port = 51830
EOF

    sudo chmod 644 "$BRIDGE_CONFIG"
    
    echo -e "${green}✓ Bridge configuration created at $BRIDGE_CONFIG${reset}"
    echo ""
    echo "Configuration preview:"
    cat "$BRIDGE_CONFIG"
}

create_bridge_service() {
    local green="\033[0;32m"
    local reset="\033[0m"
    local red="\033[0;31m"
    local yellow="\033[0;33m"

    echo -e "${yellow}=== Creating nym-bridge systemd Service ===${reset}"
    echo ""

    # Check if bridge binary exists
    if [[ ! -f "$BRIDGE_BINARY" ]]; then
        echo -e "${red}Error: Bridge binary not found at $BRIDGE_BINARY${reset}"
        echo "Please install the nym-bridge binary first"
        return 1
    fi

    # Check if config exists
    if [[ ! -f "$BRIDGE_CONFIG" ]]; then
        echo -e "${red}Error: Bridge config not found at $BRIDGE_CONFIG${reset}"
        echo "Run 'nym-bridge-helper create_bridge_config' first"
        return 1
    fi

    # Create systemd service file
    local service_file="/etc/systemd/system/nym-bridge.service"
    
    echo "Creating systemd service file..."
    sudo tee "$service_file" > /dev/null <<EOF
[Unit]
Description=Nym QUIC Bridge
After=network.target nym-node.service
Wants=nym-node.service

[Service]
Type=simple
User=root
ExecStart=$BRIDGE_BINARY --config $BRIDGE_CONFIG
Restart=on-failure
RestartSec=10
StandardOutput=journal
StandardError=journal
SyslogIdentifier=nym-bridge

# Security settings
NoNewPrivileges=false
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$BRIDGE_CONFIG_DIR

# Network capabilities
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_BIND_SERVICE
CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_BIND_SERVICE

[Install]
WantedBy=multi-user.target
EOF

    echo -e "${green}✓ Systemd service file created at $service_file${reset}"
    
    # Reload systemd
    echo "Reloading systemd daemon..."
    sudo systemctl daemon-reload
    
    echo ""
    echo -e "${green}Service created successfully!${reset}"
    echo ""
    echo "To enable and start the service:"
    echo "  sudo systemctl enable nym-bridge"
    echo "  sudo systemctl start nym-bridge"
    echo ""
    echo "To check status:"
    echo "  sudo systemctl status nym-bridge"
}

install_bridge_binary() {
    local green="\033[0;32m"
    local reset="\033[0m"
    local red="\033[0;31m"
    local yellow="\033[0;33m"

    echo -e "${yellow}=== Installing nym-bridge Binary ===${reset}"
    echo ""

    read -p "Enter bridge binary URL for your system from here https://builds.ci.nymte.ch/QUIC/: " binary_url
    if [[ -z "$binary_url" ]]; then
        echo -e "${red}Error: Binary URL is required${reset}"
        return 1
    fi

    echo "Downloading nym-bridge binary..."
    if sudo curl -L "$binary_url" -o "$BRIDGE_BINARY"; then
        sudo chmod 755 "$BRIDGE_BINARY"
        echo -e "${green}✓ Bridge binary installed at $BRIDGE_BINARY${reset}"
        
        # Show version
        echo ""
        echo "Binary version:"
        $BRIDGE_BINARY --version || echo "Unable to determine version"
    else
        echo -e "${red}✗ Failed to download bridge binary${reset}"
        return 1
    fi
}

full_bridge_setup() {
    local green="\033[0;32m"
    local reset="\033[0m"
    local yellow="\033[0;33m"

    echo -e "${yellow}========================================${reset}"
    echo -e "${yellow}   Nym QUIC Bridge - Full Setup${reset}"
    echo -e "${yellow}========================================${reset}"
    echo ""

    echo "This will guide you through complete bridge setup"
    echo ""

    # Step 1: Prerequisites
    echo "Step 1/7: Checking prerequisites..."
    verify_bridge_prerequisites
    read -p "Press Enter to continue..."

    # Step 2: Install binary
    echo ""
    echo "Step 2/7: Installing bridge binary..."
    install_bridge_binary
    read -p "Press Enter to continue..."

    # Step 3: Generate keys
    echo ""
    echo "Step 3/7: Generating bridge keys..."
    generate_bridge_keys
    read -p "Press Enter to continue..."

    # Step 4: Create client params
    echo ""
    echo "Step 4/7: Creating client parameters..."
    create_client_params
    read -p "Press Enter to continue..."

    # Step 5: Create bridge config
    echo ""
    echo "Step 5/7: Creating bridge configuration..."
    create_bridge_config
    read -p "Press Enter to continue..."

    # Step 6: Create service
    echo ""
    echo "Step 6/7: Creating systemd service..."
    create_bridge_service
    read -p "Press Enter to continue..."

    # Step 7: Network setup
    echo ""
    echo "Step 7/7: Configuring network..."
    adjust_ip_forwarding
    apply_bridge_iptables_rules
    configure_dns_and_icmp

    echo ""
    echo -e "${green}========================================${reset}"
    echo -e "${green}   Bridge Setup Complete!${reset}"
    echo -e "${green}========================================${reset}"
    echo ""
    echo "To start the bridge:"
    echo "  sudo systemctl enable nym-bridge"
    echo "  sudo systemctl start nym-bridge"
    echo ""
    echo "To check status:"
    echo "  nym-bridge-helper check_bridge_service_status"
}

case "$1" in
fetch_and_display_ipv6)
    fetch_and_display_ipv6
    ;;
fetch_wg_ipv6_address)
    fetch_wg_ipv6_address
    ;;
apply_bridge_iptables_rules)
    apply_bridge_iptables_rules
    ;;
check_bridge_iptables)
    check_bridge_iptables
    ;;
remove_duplicate_bridge_rules)
    remove_duplicate_bridge_rules
    ;;
configure_dns_and_icmp)
    configure_dns_and_icmp
    ;;
adjust_ip_forwarding)
    adjust_ip_forwarding
    ;;
check_ipv6_ipv4_forwarding)
    check_ipv6_ipv4_forwarding
    ;;
check_ip_routing)
    check_ip_routing
    ;;
perform_pings)
    perform_pings
    ;;
test_bridge_connectivity)
    test_bridge_connectivity
    ;;
check_bridge_service_status)
    check_bridge_service_status
    ;;
show_bridge_logs)
    show_bridge_logs "$2"
    ;;
check_bridge_installation)
    check_bridge_installation
    ;;
show_bridge_config)
    show_bridge_config
    ;;
show_bridge_keys)
    show_bridge_keys
    ;;
show_bridge_info)
    show_bridge_info
    ;;
verify_bridge_prerequisites)
    verify_bridge_prerequisites
    ;;
generate_bridge_keys)
    generate_bridge_keys
    ;;
create_client_params)
    create_client_params
    ;;
create_bridge_config)
    create_bridge_config
    ;;
create_bridge_service)
    create_bridge_service
    ;;
install_bridge_binary)
    install_bridge_binary
    ;;
full_bridge_setup)
    full_bridge_setup
    ;;
*)
    echo "Usage: $0 [command] [options]"
    echo ""
    echo "Nym QUIC Bridge Deployment Helper Script"
    echo ""
    echo "Bridge Installation & Configuration:"
    echo "  check_bridge_installation       - Check bridge installation status"
    echo "  show_bridge_config              - Display bridge configuration files"
    echo "  show_bridge_keys                - Display bridge key information"
    echo "  show_bridge_info                - Show comprehensive bridge information"
    echo "  verify_bridge_prerequisites     - Verify all prerequisites are met"
    echo ""
    echo "Bridge Setup Commands:"
    echo "  install_bridge_binary           - Download and install nym-bridge binary"
    echo "  generate_bridge_keys            - Generate ED25519 bridge identity keys"
    echo "  create_client_params            - Create client_bridge_params.json"
    echo "  create_bridge_config            - Create bridges.toml configuration"
    echo "  create_bridge_service           - Create systemd service file"
    echo "  full_bridge_setup               - Interactive full bridge setup wizard"
    echo ""
    echo "Network Configuration Commands:"
    echo "  adjust_ip_forwarding            - Enable IPv4 and IPv6 forwarding"
    echo "  apply_bridge_iptables_rules     - Apply iptables rules for QUIC bridge (nymwg)"
    echo "  configure_dns_and_icmp          - Allow ICMP ping tests and configure DNS"
    echo "  remove_duplicate_bridge_rules   - Remove duplicate iptables rules for nymwg"
    echo ""
    echo "Network Inspection Commands:"
    echo "  fetch_and_display_ipv6          - Show IPv6 on default network device"
    echo "  fetch_wg_ipv6_address           - Fetch IPv6 for nymwg interface"
    echo "  check_bridge_iptables           - Check iptables rules for nymwg"
    echo "  check_ipv6_ipv4_forwarding      - Check IPv4 and IPv6 forwarding status"
    echo "  check_ip_routing                - Display IP routing tables"
    echo ""
    echo "Testing Commands:"
    echo "  perform_pings                   - Test IPv4 and IPv6 connectivity"
    echo "  test_bridge_connectivity        - Comprehensive bridge connectivity test"
    echo ""
    echo "Service Management Commands:"
    echo "  check_bridge_service_status     - Check nym-bridge and nym-node service status"
    echo "  show_bridge_logs [lines]        - Show recent nym-bridge logs (default: 50 lines)"
    echo ""
    echo "Quick Start:"
    echo "  1. Run 'verify_bridge_prerequisites' to check prerequisites"
    echo "  2. Run 'check_bridge_installation' to verify installation"
    echo "  3. Run 'test_bridge_connectivity' to test connectivity"
    echo ""
    exit 1
    ;;
esac

echo "Operation '$1' completed successfully."

