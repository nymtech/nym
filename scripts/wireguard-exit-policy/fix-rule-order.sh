#!/bin/bash
# Fix the FORWARD chain order so NYM-EXIT comes before ACCEPT

WG_INTERFACE="nymwg"
NETWORK_DEVICE=$(ip route show default | awk '/default/ {print $5}')
NYM_CHAIN="NYM-EXIT"

echo "Fixing FORWARD chain order..."

# Remove ALL ACCEPT rules for nymwg → network_device (there might be duplicates)
while iptables -D FORWARD -i "$WG_INTERFACE" -o "$NETWORK_DEVICE" -j ACCEPT 2>/dev/null; do
    echo "Removed ACCEPT rule"
done

# Remove NYM-EXIT hook if it exists (we'll re-add it)
iptables -D FORWARD -i "$WG_INTERFACE" -o "$NETWORK_DEVICE" -j "$NYM_CHAIN" 2>/dev/null || true

# Insert NYM-EXIT at position 1
iptables -I FORWARD 1 -i "$WG_INTERFACE" -o "$NETWORK_DEVICE" -j "$NYM_CHAIN"
echo "Added NYM-EXIT hook at position 1"

# Insert ACCEPT right after NYM-EXIT (position 2)
iptables -I FORWARD 2 -i "$WG_INTERFACE" -o "$NETWORK_DEVICE" -j ACCEPT
echo "Added ACCEPT rule at position 2 (after NYM-EXIT)"

# Same for IPv6
while ip6tables -D FORWARD -i "$WG_INTERFACE" -o "$NETWORK_DEVICE" -j ACCEPT 2>/dev/null; do
    echo "Removed IPv6 ACCEPT rule"
done

ip6tables -D FORWARD -i "$WG_INTERFACE" -o "$NETWORK_DEVICE" -j "$NYM_CHAIN" 2>/dev/null || true
ip6tables -I FORWARD 1 -i "$WG_INTERFACE" -o "$NETWORK_DEVICE" -j "$NYM_CHAIN"
ip6tables -I FORWARD 2 -i "$WG_INTERFACE" -o "$NETWORK_DEVICE" -j ACCEPT
echo "Fixed IPv6 rules"

echo ""
echo "=== New FORWARD chain order ==="
iptables -L FORWARD -n --line-numbers | head -10

echo ""
echo "=== Verify NYM-EXIT is before ACCEPT ==="
iptables -L FORWARD -n --line-numbers | grep -E "$WG_INTERFACE|$NYM_CHAIN" | head -5

