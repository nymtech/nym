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
        else
        echo "IPv6 address on $network_device: $ipv6_address"
    fi
}

remove_duplicate_rules() {
    local interface=$1
    local script_name=$(basename "$0")

    if [[ -z "$interface" ]]; then
        echo "error: no interface specified. please enter the interface (nymwg or nymtun0):"
        read -r interface
    fi

    if [[ "$interface" != "nymwg" && "$interface" != "nymtun0" ]]; then
        echo "error: invalid interface '$interface'. allowed values are 'nymwg' or 'nymtun0'." >&2
        exit 1
    fi

    echo "removing duplicate rules for $interface..."

    iptables-save | grep "$interface" | while read -r line; do
        sudo iptables -D ${line#-A } || echo "Failed to delete rule: $line"
    done

    ip6tables-save | grep "$interface" | while read -r line; do
        sudo ip6tables -D ${line#-A } || echo "Failed to delete rule: $line"
    done

    echo "duplicates removed for $interface."
    echo "!!-important-!!  you need to now reapply the iptables rules for $interface."
    if [ "$interface" == "nymwg" ]; then
        echo "run: ./$script_name apply_iptables_rules_wg"
    else
        echo "run: ./$script_name apply_iptables_rules"
    fi
}

adjust_ip_forwarding() {
    ipv6_forwarding_setting="net.ipv6.conf.all.forwarding=1"
    ipv4_forwarding_setting="net.ipv4.ip_forward=1"
    echo "$ipv6_forwarding_setting" | sudo tee -a /etc/sysctl.conf
    echo "$ipv4_forwarding_setting" | sudo tee -a /etc/sysctl.conf
    sysctl -p /etc/sysctl.conf
}

apply_iptables_rules() {
    local interface=$1
    echo "applying IPtables rules for $interface..."
    sleep 2

    sudo iptables -t nat -A POSTROUTING -o "$network_device" -j MASQUERADE
    sudo iptables -A FORWARD -i "$interface" -o "$network_device" -j ACCEPT
    sudo iptables -A FORWARD -i "$network_device" -o "$interface" -m state --state RELATED,ESTABLISHED -j ACCEPT

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

    ipv4_address=$(ip addr show "$interface" | awk '/inet / {print $2}' | cut -d'/' -f1)
    ipv6_address=$(ip addr show "$interface" | awk '/inet6 / && $2 !~ /^fe80/ {print $2}' | cut -d'/' -f1)

    if [[ -z "$ipv4_address" && -z "$ipv6_address" ]]; then
        echo "No IP address found on $interface. Unable to fetch a joke."
        echo "Please verify your tunnel configuration and ensure the interface is up."
        return 1
    fi

    if [[ -n "$ipv4_address" ]]; then
        echo "detected IPv4 address: $ipv4_address"
        echo "testing IPv4 connectivity..."
        echo "if this test succeeds, it confirms your machine can reach the outside world via IPv4."
        echo "however, probes and external clients may experience different connectivity to your probe."

        if ping -c 1 -I "$ipv4_address" google.com >/dev/null 2>&1; then
            echo "IPv4 connectivity is working. Fetching a joke..."
            joke=$(curl -s -H "Accept: application/json" --interface "$ipv4_address" https://icanhazdadjoke.com/ | jq -r .joke)
            [[ -n "$joke" && "$joke" != "null" ]] && echo "IPv4 joke: $joke" || echo "Failed to fetch a joke via IPv4."
        else
            echo "IPv4 connectivity is not working for $interface. verify your routing and NAT settings."
        fi
    fi

    if [[ -n "$ipv6_address" ]]; then
        echo "detected IPv6 address: $ipv6_address"
        echo "testing IPv6 connectivity..."
        echo "if this test succeeds, it confirms your machine can reach the outside world via IPv6."
        echo "however, probes and external clients may experience different connectivity to your nym-node."

        if ping6 -c 1 -I "$ipv6_address" google.com >/dev/null 2>&1; then
            echo "IPv6 connectivity is working. Fetching a joke..."
            joke=$(curl -s -H "Accept: application/json" --interface "$ipv6_address" https://icanhazdadjoke.com/ | jq -r .joke)
            [[ -n "$joke" && "$joke" != "null" ]] && echo "IPv6 joke: $joke" || echo "Failed to fetch a joke via IPv6."
        else
            echo "IPv6 connectivity is not working for $interface. verify your routing and NAT settings."
        fi
    fi

    echo "joke fetching process completed for $interface."
    sleep 1
    echo
    echo "### Connectivity Testing Recommendations ###"
    echo "- use the following command to test WebSocket connectivity from an external client:"
    echo "  wscat -c wss://<your-ip-address>:9001"
    echo
    echo "- test UDP connectivity on port 51822 (commonly used for nym wireguard):"
    echo "  From another machine, use tools like nc or socat to send UDP packets:"
    echo "  echo 'test message' | nc -u <your-ip-address> 51822"
    echo 
    echo "if connectivity issues persist, ensure port forwarding and firewall rules are correctly configured"
}

configure_dns_and_icmp_wg() {
    echo "allowing icmp (ping)..."
    sudo iptables -A INPUT -p icmp --icmp-type echo-request -j ACCEPT
    sudo iptables -A OUTPUT -p icmp --icmp-type echo-reply -j ACCEPT

    echo "allowing dns over udp (port 53)..."
    sudo iptables -A INPUT -p udp --dport 53 -j ACCEPT

    echo "allowing dns over tcp (port 53)..."
    sudo iptables -A INPUT -p tcp --dport 53 -j ACCEPT

    echo "saving iptables rules..."
    sudo iptables-save >/etc/iptables/rules.v4

    echo "dns and icmp configuration completed."
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
configure_dns_and_icmp_wg)
    configure_dns_and_icmp_wg
    ;;
adjust_ip_forwarding)
    adjust_ip_forwarding
    ;;
remove_duplicate_rules)
    remove_duplicate_rules "$2"
    ;;
*)
    echo "Usage: $0 [command]"
    echo "Commands:"
    echo "  fetch_ipv6_address_nym_tun      - Fetch IPv6 for nymtun0."
    echo "  fetch_and_display_ipv6          - Show IPv6 on default device."
    echo "  apply_iptables_rules            - Apply IPtables rules for nymtun0."
    echo "  apply_iptables_rules_wg         - Apply IPtables rules for nymwg."
    echo "  check_nymtun_iptables           - Check IPtables for nymtun0."
    echo "  check_nym_wg_tun                - Check IPtables for nymwg."
    echo "  check_ipv6_ipv4_forwarding      - Check IPv4 and IPv6 forwarding."
    echo "  check_ip_routing                - Display IP routing tables."
    echo "  perform_pings                   - Test IPv4 and IPv6 connectivity."
    echo "  joke_through_the_mixnet         - Fetch a joke via nymtun0."
    echo "  joke_through_wg_tunnel          - Fetch a joke via nymwg."
    echo "  configure_dns_and_icmp_wg       - Allows icmp ping tests for probes alongside configuring dns"
    echo "  adjust_ip_forwarding            - Enable IPV6 and IPV4 forwarding"
    echo "  remove_duplicate_rules <interface> - Remove duplicate iptables rules. Valid interfaces: nymwg, nymtun0"
    exit 1
    ;;
esac

echo "operation $1 completed successfully."
