
#!/bin/bash

# detect active network interface
INTERFACE=$(ip -o link show | awk -F': ' '{print $2}' | grep -v lo | head -n 1)

echo "Detected active network interface: $INTERFACE"
read -p "Is this correct? (y/n): " CONFIRM
if [[ "$CONFIRM" != "y" ]]; then
    echo "Exiting. Please manually specify the correct network interface."
    exit 1
fi

# prompt for network settings
read -p "Enter IPv4 address for VM (leave blank if not used): " IPv4_VM
read -p "Enter IPv6 address for VM (leave blank if not used): " IPv6_VM
read -p "Enter IPv4 gateway (host server, leave blank if not used): " IPv4_GATEWAY_HOST_SERVER
read -p "Enter IPv6 gateway (host server, leave blank if not used): " IPv6_GATEWAY_HOST_SERVER

# resize partition
echo "Expanding partition and resizing filesystem..."
growpart /dev/vda 1
resize2fs /dev/vda1
df -h

# ask before continuing
read -p "Continue with network configuration? (y/n): " CONTINUE
if [[ "$CONTINUE" != "y" ]]; then
    echo "Exiting."
    exit 1
fi

# generate Netplan config
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

# append IPv4 address if provided
if [[ -n "$IPv4_VM" ]]; then
  echo "        - $IPv4_VM/24" >> $NETPLAN_CONFIG
fi

# append IPv6 address if provided
if [[ -n "$IPv6_VM" ]]; then
  echo "        - $IPv6_VM/64" >> $NETPLAN_CONFIG
fi

echo "      routes:" >> $NETPLAN_CONFIG

# append IPv4 route if provided
if [[ -n "$IPv4_GATEWAY_HOST_SERVER" ]]; then
  echo "        - to: default" >> $NETPLAN_CONFIG
  echo "          via: $IPv4_GATEWAY_HOST_SERVER" >> $NETPLAN_CONFIG
fi

# append IPv6 route if provided
if [[ -n "$IPv6_GATEWAY_HOST_SERVER" ]]; then
  echo "        - to: default" >> $NETPLAN_CONFIG
  echo "          via: $IPv6_GATEWAY_HOST_SERVER" >> $NETPLAN_CONFIG
fi

cat <<EOF >> $NETPLAN_CONFIG
      nameservers:
        addresses:
          - 1.1.1.1  # Cloudflare IPv4 DNS
          - 8.8.8.8  # Google IPv4 DNS
          - 2606:4700:4700::1111  # Cloudflare IPv6 DNS
          - 2001:4860:4860::8888  # Google IPv6 DNS
EOF

# secure Netplan config
chmod 600 $NETPLAN_CONFIG

# generate Netplan configuration
netplan generate

# ask before applying Netplan
read -p "Apply Netplan changes? (y/n): " CONTINUE
if [[ "$CONTINUE" != "y" ]]; then
    echo "Exiting."
    exit 1
fi

# apply Netplan and verify settings
netplan --debug apply

# show IP configurations
ip -4 a
ip -6 a
ip -4 r
ip -6 r

# test network connectivity
echo "Testing IPv4 connectivity for 10 seconds..."
timeout 10 ping -4 google.com

echo "Testing IPv6 connectivity for 10 seconds..."
timeout 10 ping -6 google.com

# ask before updating system
read -p "Proceed with system update and upgrade? (y/n): " CONTINUE
if [[ "$CONTINUE" != "y" ]]; then
    echo "Skipping updates."
else
    apt update && apt upgrade -y
fi

# generate SSH host keys without password
echo "Generating SSH host keys..."
ssh-keygen -t rsa -f /etc/ssh/ssh_host_rsa_key -N ""
ssh-keygen -t dsa -f /etc/ssh/ssh_host_dsa_key -N ""
ssh-keygen -t ecdsa -f /etc/ssh/ssh_host_ecdsa_key -N ""
ssh-keygen -t ed25519 -f /etc/ssh/ssh_host_ed25519_key -N ""

# restart SSH service
systemctl restart ssh.service

# ensure ~/.ssh directory exists
mkdir -p ~/.ssh

# Open authorized_keys file for user input
echo "# Add your admin SSH keys here, save and exit!" > ~/.ssh/authorized_keys
nano ~/.ssh/authorized_keys

echo "Setup complete! Try to ping and ssh from the outside before killing this console"
