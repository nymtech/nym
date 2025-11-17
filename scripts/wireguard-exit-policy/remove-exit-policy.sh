#!/bin/bash
# Remove exit policy rules (use this if accidentally applied to entry gateway)

WG_INTERFACE="nymwg"
NETWORK_DEVICE=$(ip route show default | awk '/default/ {print $5}')
NYM_CHAIN="NYM-EXIT"

echo "Removing exit policy rules..."

# Remove NYM-EXIT hooks from FORWARD chain
iptables -D FORWARD -i "$WG_INTERFACE" -o "$NETWORK_DEVICE" -j "$NYM_CHAIN" 2>/dev/null || true
iptables -D FORWARD -o "$WG_INTERFACE" -j "$NYM_CHAIN" 2>/dev/null || true
ip6tables -D FORWARD -i "$WG_INTERFACE" -o "$NETWORK_DEVICE" -j "$NYM_CHAIN" 2>/dev/null || true
ip6tables -D FORWARD -o "$WG_INTERFACE" -j "$NYM_CHAIN" 2>/dev/null || true

# Flush and delete NYM-EXIT chains
iptables -F "$NYM_CHAIN" 2>/dev/null || true
iptables -X "$NYM_CHAIN" 2>/dev/null || true
ip6tables -F "$NYM_CHAIN" 2>/dev/null || true
ip6tables -X "$NYM_CHAIN" 2>/dev/null || true

echo "✓ Exit policy rules removed"
echo ""
echo "=== Current FORWARD chain (first 10 rules) ==="
iptables -L FORWARD -n --line-numbers | head -10

