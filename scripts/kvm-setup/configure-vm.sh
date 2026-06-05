#!/bin/bash

usage() {
    local code="${1:-0}"
    cat <<EOF
Usage: $0 [OPTIONS]

Options:
  -i, --interface   Network interface (optional; auto-detected if omitted)
  -4, --ipv4        IPv4 address for the VM (optional)
  -6, --ipv6        IPv6 address for the VM (optional)
  -g, --gateway4    IPv4 gateway of the host server (optional)
  -G, --gateway6    IPv6 gateway of the host server (optional)
  -y, --yes         Skip all confirmation prompts (auto-confirm)
  -h, --help        Show this help message

Example:
  $0 --ipv4 192.168.1.100 --gateway4 192.168.1.1 --ipv6 2001:db8::1 --gateway6 2001:db8::fffe
  $0 --ipv4 192.168.1.100 --gateway4 192.168.1.1 --yes
EOF
    exit "$code"
}

# --- parse flags ---
INTERFACE=""
IPv4_VM=""
IPv6_VM=""
IPv4_GATEWAY_HOST_SERVER=""
IPv6_GATEWAY_HOST_SERVER=""
AUTO_YES=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        -i|--interface)
            [[ -n "${2:-}" && "${2:0:1}" != "-" ]] || { echo "Error: --interface requires a value."; exit 1; }
            INTERFACE="$2"; shift 2 ;;
        -4|--ipv4)
            [[ -n "${2:-}" && "${2:0:1}" != "-" ]] || { echo "Error: --ipv4 requires a value."; exit 1; }
            IPv4_VM="$2"; shift 2 ;;
        -6|--ipv6)
            [[ -n "${2:-}" && "${2:0:1}" != "-" ]] || { echo "Error: --ipv6 requires a value."; exit 1; }
            IPv6_VM="$2"; shift 2 ;;
        -g|--gateway4)
            [[ -n "${2:-}" && "${2:0:1}" != "-" ]] || { echo "Error: --gateway4 requires a value."; exit 1; }
            IPv4_GATEWAY_HOST_SERVER="$2"; shift 2 ;;
        -G|--gateway6)
            [[ -n "${2:-}" && "${2:0:1}" != "-" ]] || { echo "Error: --gateway6 requires a value."; exit 1; }
            IPv6_GATEWAY_HOST_SERVER="$2"; shift 2 ;;
        -y|--yes)       AUTO_YES=true;                  shift ;;
        -h|--help)      usage ;;
        *)
            echo "Error: Unknown option: $1"
            usage 1
            ;;
    esac
done

# at least one of IPv4 or IPv6 must be provided
if [[ -z "$IPv4_VM" && -z "$IPv6_VM" ]]; then
    echo "Error: At least one of --ipv4 or --ipv6 must be provided."
    echo "Run '$0 --help' for usage."
    exit 1
fi

confirm() {
    local prompt="$1"
    if $AUTO_YES; then
        echo "${prompt} [Y/n] (auto-confirmed)"
        return 0
    fi
    read -p "${prompt} [Y/n]: " REPLY
    [[ -z "$REPLY" || "$REPLY" == "y" || "$REPLY" == "Y" ]]
}

# --- detect or validate interface ---
if [[ -z "$INTERFACE" ]]; then
    INTERFACE=$(ip -o link show | awk -F': ' '{print $2}' | grep -v lo | head -n 1)
    echo "Detected active network interface: $INTERFACE"
    if ! confirm "Is this correct?"; then
        echo "Exiting. Re-run with --interface <name> to specify one manually."
        exit 1
    fi
else
    echo "Using network interface: $INTERFACE"
fi

# --- resize partition ---
echo "Expanding partition and resizing filesystem..."
growpart /dev/vda 1
resize2fs /dev/vda1
df -h

if ! confirm "Continue with network configuration?"; then
    echo "Exiting."
    exit 1
fi

# --- generate Netplan config ---
NETPLAN_CONFIG="/etc/netplan/01-network-config.yaml"
echo "Creating Netplan configuration at $NETPLAN_CONFIG..."

cat <<EOF > $NETPLAN_CONFIG
network:
  version: 2
  renderer: networkd
  ethernets:
    $INTERFACE:
      dhcp4: false
      dhcp6: false
      addresses:
EOF

[[ -n "$IPv4_VM" ]] && echo "        - $IPv4_VM/24" >> $NETPLAN_CONFIG
[[ -n "$IPv6_VM" ]] && echo "        - $IPv6_VM/64" >> $NETPLAN_CONFIG

if [[ -n "$IPv4_GATEWAY_HOST_SERVER" || -n "$IPv6_GATEWAY_HOST_SERVER" ]]; then
    echo "      routes:" >> $NETPLAN_CONFIG
    if [[ -n "$IPv4_GATEWAY_HOST_SERVER" ]]; then
        echo "        - to: default"                    >> $NETPLAN_CONFIG
        echo "          via: $IPv4_GATEWAY_HOST_SERVER" >> $NETPLAN_CONFIG
    fi
    if [[ -n "$IPv6_GATEWAY_HOST_SERVER" ]]; then
        echo "        - to: default"                    >> $NETPLAN_CONFIG
        echo "          via: $IPv6_GATEWAY_HOST_SERVER" >> $NETPLAN_CONFIG
    fi
fi

cat <<EOF >> $NETPLAN_CONFIG
      nameservers:
        addresses:
          - 1.1.1.1  # Cloudflare IPv4 DNS
          - 8.8.8.8  # Google IPv4 DNS
          - 2606:4700:4700::1111  # Cloudflare IPv6 DNS
          - 2001:4860:4860::8888  # Google IPv6 DNS
EOF

chmod 600 $NETPLAN_CONFIG
netplan generate

if ! confirm "Apply Netplan changes?"; then
    echo "Exiting."
    exit 1
fi

netplan --debug apply

ip -4 a
ip -6 a
ip -4 r
ip -6 r

echo "Testing IPv4 connectivity for 10 seconds..."
timeout 10 ping -4 google.com
echo "Testing IPv6 connectivity for 10 seconds..."
timeout 10 ping -6 google.com

if confirm "Proceed with system update and upgrade?"; then
    apt update && apt upgrade -y
else
    echo "Skipping updates."
fi

# --- SSH setup ---
echo "Generating SSH host keys..."
ssh-keygen -t rsa     -f /etc/ssh/ssh_host_rsa_key     -N ""
ssh-keygen -t dsa     -f /etc/ssh/ssh_host_dsa_key     -N ""
ssh-keygen -t ecdsa   -f /etc/ssh/ssh_host_ecdsa_key   -N ""
ssh-keygen -t ed25519 -f /etc/ssh/ssh_host_ed25519_key -N ""

systemctl restart ssh.service

mkdir -p ~/.ssh
echo "# Add your admin SSH keys here, save and exit!" > ~/.ssh/authorized_keys
nano ~/.ssh/authorized_keys

echo "Setup complete! Try to ping and ssh from the outside before killing this console"