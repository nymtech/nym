#!/bin/bash

network_device=$(ip route show default | awk '/default/ {print $5}')
tunnel_interface="nymtun0"
wg_tunnel_interface="nymwg"

if ! dpkg -s iptables-persistent >/dev/null 2>&1; then
    sudo apt-get update
    sudo apt-get install -y iptables-persistent
else
    echo "iptables-persistent is already installed."
fi

fetch_ipv6_address() {
    local interface=$1
    ipv6_global_address=$(ip -6 addr show "$interface" scope global | grep inet6 | awk '{print $2}' | head -n 1)

    if [[ -z "$ipv6_global_address" ]]; then
        echo "no globally routable IPv6 address found on $interface. Please configure IPv6 or check your network settings."
        exit 1
    else
        echo "using IPv6 address: $ipv6_global_address"
    fi
}

fetch_and_display_ipv6() {
    ipv6_address=$(ip -6 addr show "$network_device" scope global | grep inet6 | awk '{print $2}')
    if [[ -z "$ipv6_address" ]]; then
        echo "no global IPv6 address found on $network_device."
    elsen
        echo "IPv6 address on $network_device: $ipv6_address"
    fi
}

adjust_ip_forwarding() {
    echo "adjusting IP forwarding settings..."
    sudo sysctl -w net.ipv6.conf.all.forwarding=1
    sudo sysctl -w net.ipv4.ip_forward=1
}

apply_iptables_rules() {
    local interface=$1
    echo "applying IPtables rules for $interface..."
    sleep 2

    # remove duplicates for IPv4
    sudo iptables -D FORWARD -i "$interface" -o "$network_device" -j ACCEPT 2>/dev/null || true
    sudo iptables -D FORWARD -i "$network_device" -o "$interface" -m state --state RELATED,ESTABLISHED -j ACCEPT 2>/dev/null || true
    sudo iptables -t nat -D POSTROUTING -o "$network_device" -j MASQUERADE 2>/dev/null || true

    # remove duplicates for IPv6
    sudo ip6tables -D FORWARD -i "$interface" -o "$network_device" -j ACCEPT 2>/dev/null || true
    sudo ip6tables -D FORWARD -i "$network_device" -o "$interface" -m state --state RELATED,ESTABLISHED -j ACCEPT 2>/dev/null || true
    sudo ip6tables -t nat -D POSTROUTING -o "$network_device" -j MASQUERADE 2>/dev/null || true

    # add new rules for IPv4
    sudo iptables -t nat -A POSTROUTING -o "$network_device" -j MASQUERADE
    sudo iptables -A FORWARD -i "$interface" -o "$network_device" -j ACCEPT
    sudo iptables -A FORWARD -i "$network_device" -o "$interface" -m state --state RELATED,ESTABLISHED -j ACCEPT

    # add new rules for IPv6
    sudo ip6tables -t nat -A POSTROUTING -o "$network_device" -j MASQUERADE
    sudo ip6tables -A FORWARD -i "$interface" -o "$network_device" -j ACCEPT
    sudo ip6tables -A FORWARD -i "$network_device" -o "$interface" -m state --state RELATED,ESTABLISHED -j ACCEPT

    sudo iptables-save | sudo tee /etc/iptables/rules.v4
    sudo ip6tables-save | sudo tee /etc/iptables/rules.v6
}

apply_iptables_rules_wg() {
    local interface=$wg_tunnel_interface
    echo "applying IPtables rules for WireGuard ($interface)..."
    sleep 2

    # remove duplicates for IPv4
    sudo iptables -D FORWARD -i "$interface" -o "$network_device" -j ACCEPT 2>/dev/null || true
    sudo iptables -D FORWARD -i "$network_device" -o "$interface" -m state --state RELATED,ESTABLISHED -j ACCEPT 2>/dev/null || true
    sudo iptables -t nat -D POSTROUTING -o "$network_device" -j MASQUERADE 2>/dev/null || true

    # remove duplicates for IPv6
    sudo ip6tables -D FORWARD -i "$interface" -o "$network_device" -j ACCEPT 2>/dev/null || true
    sudo ip6tables -D FORWARD -i "$network_device" -o "$interface" -m state --state RELATED,ESTABLISHED -j ACCEPT 2>/dev/null || true
    sudo ip6tables -t nat -D POSTROUTING -o "$network_device" -j MASQUERADE 2>/dev/null || true

    # add new rules for IPv4
    sudo iptables -t nat -A POSTROUTING -o "$network_device" -j MASQUERADE
    sudo iptables -A FORWARD -i "$interface" -o "$network_device" -j ACCEPT
    sudo iptables -A FORWARD -i "$network_device" -o "$interface" -m state --state RELATED,ESTABLISHED -j ACCEPT

    # add new rules for IPv6
    sudo ip6tables -t nat -A POSTROUTING -o "$network_device" -j MASQUERADE
    sudo ip6tables -A FORWARD -i "$interface" -o "$network_device" -j ACCEPT
    sudo ip6tables -A FORWARD -i "$network_device" -o "$interface" -m state --state RELATED,ESTABLISHED -j ACCEPT

    sudo iptables-save | sudo tee /etc/iptables/rules.v4
    sudo ip6tables-save | sudo tee /etc/iptables/rules.v6
}


check_tunnel_iptables() {
    local interface=$1
    echo "inspecting IPtables rules for $interface..."
    echo "---------------------------------------"
    echo "IPv4 rules:"
    iptables -L FORWARD -v -n | awk -v dev="$interface" '/^Chain FORWARD/ || $0 ~ dev || $0 ~ "ufw-reject-forward"'
    echo "---------------------------------------"
    echo "IPv6 rules:"
    ip6tables -L FORWARD -v -n | awk -v dev="$interface" '/^Chain FORWARD/ || $0 ~ dev || $0 ~ "ufw6-reject-forward"'
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
    echo "performing IPv4 ping to google.com..."
    ping -c 4 google.com
    echo "---------------------------------------"
    echo "performing IPv6 ping to google.com..."
    ping6 -c 4 google.com
}

joke_through_tunnel() {
    local interface=$1
    echo "checking tunnel connectivity and fetching a joke for $interface..."
    ipv4_address=$(ip addr show "$interface" | grep 'inet ' | awk '{print $2}' | cut -d'/' -f1)
    ipv6_address=$(ip addr show "$interface" | grep 'inet6 ' | awk '{print $2}' | grep -v '^fe80:' | cut -d'/' -f1)

    if [[ -z "$ipv4_address" && -z "$ipv6_address" ]]; then
        echo "no IP address found on $interface. Unable to fetch a joke."
        return
    fi

    if [[ -n "$ipv4_address" ]]; then
        joke=$(curl -s -H "Accept: application/json" --interface "$ipv4_address" https://icanhazdadjoke.com/ | jq -r .joke)
        [[ -n "$joke" && "$joke" != "null" ]] && echo "IPv4 joke: $joke" || echo "Failed to fetch a joke via IPv4."
    fi

    if [[ -n "$ipv6_address" ]]; then
        joke=$(curl -s -H "Accept: application/json" --interface "$ipv6_address" https://icanhazdadjoke.com/ | jq -r .joke)
        [[ -n "$joke" && "$joke" != "null" ]] && echo "IPv6 joke: $joke" || echo "Failed to fetch a joke via IPv6."
    fi
}

case "$1" in
fetch_ipv6_address_nym_tun)
    fetch_ipv6_address "$tunnel_interface"
    ;;
fetch_and_display_ipv6)
    fetch_and_display_ipv6
    ;;
apply_iptables_rules)
    apply_iptables_rules "$tunnel_interface"
    ;;
apply_iptables_rules_wg)
    apply_iptables_rules "$wg_tunnel_interface"
    ;;
check_nymtun_iptables)
    check_tunnel_iptables "$tunnel_interface"
    ;;
check_nym_wg_tun)
    check_tunnel_iptables "$wg_tunnel_interface"
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
joke_through_the_mixnet)
    joke_through_tunnel "$tunnel_interface"
    ;;
joke_through_wg_tunnel)
    joke_through_tunnel "$wg_tunnel_interface"
    ;;
*)
    echo "Usage: $0 [command]"
    echo "Commands:"
    echo "  fetch_ipv6_address_nym_tun      - Fetch IPv6 for nymtun0."
    echo "  fetch_and_display_ipv6          - Show IPv6 on default device."
    echo "  apply_iptables_rules            - Apply IPtables rules for nymtun0."
    echo "  apply_iptables_rules_wg         - Apply IPtables rules for nymwg."
    echo "  remove_iptables_rules           - Remove IPtables rules for nymtun0."
    echo "  remove_iptables_rules_wg        - Remove IPtables rules for nymwg."
    echo "  check_nymtun_iptables           - Check IPtables for nymtun0."
    echo "  check_nym_wg_tun                - Check IPtables for nymwg."
    echo "  check_ipv6_ipv4_forwarding      - Check IPv4 and IPv6 forwarding."
    echo "  check_ip_routing                - Display IP routing tables."
    echo "  perform_pings                   - Test IPv4 and IPv6 connectivity."
    echo "  joke_through_the_mixnet         - Fetch a joke via nymtun0."
    echo "  joke_through_wg_tunnel          - Fetch a joke via nymwg."
    exit 1
    ;;
esac

echo "operation $1 completed successfully."
