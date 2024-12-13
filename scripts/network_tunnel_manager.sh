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

fetch_ipv6_address_nym_tun() {
    ipv6_global_address=$(ip -6 addr show $tunnel_interface scope global | grep inet6 | awk '{print $2}' | head -n 1)

    if [[ -z "$ipv6_global_address" ]]; then
        echo "no globally routable IPv6 address found on $tunnel_interface. please configure IPv6 or check your network settings."
        exit 1
    else
        echo "using IPv6 address: $ipv6_global_address"
    fi
}

fetch_and_display_ipv6() {
    ipv6_address=$(ip -6 addr show ${network_device} scope global | grep inet6 | awk '{print $2}')
    if [[ -z "$ipv6_address" ]]; then
        echo "no global IPv6 address found on ${network_device}."
    else
        echo "IPv6 address on ${network_device}: $ipv6_address"
    fi
}

adjust_ip_forwarding() {
    ipv6_forwarding_setting="net.ipv6.conf.all.forwarding=1"
    ipv4_forwarding_setting="net.ipv4.ip_forward=1"
    echo "$ipv6_forwarding_setting" | sudo tee -a /etc/sysctl.conf
    echo "$ipv4_forwarding_setting" | sudo tee -a /etc/sysctl.conf
    sysctl -p /etc/sysctl.conf
}

apply_iptables_rules_wg() {
    echo "applying IPtables rules..."
    echo "network device: ${network_device}"
    echo "tunnel_interface: ${wg_tunnel_interface}"
    sleep 2
    sudo iptables -A FORWARD -i "$wg_tunnel_interface" -o "$network_device" -j ACCEPT
    sudo iptables -A FORWARD -i "$network_device" -o "$wg_tunnel_interface" -m state --state RELATED,ESTABLISHED -j ACCEPT
    sudo ip6tables -A FORWARD -i "$network_device" -o "$wg_tunnel_interface" -m state --state RELATED,ESTABLISHED -j ACCEPT
    sudo ip6tables -A FORWARD -i "$wg_tunnel_interface" -o "$network_device" -j ACCEPT
    sudo iptables-save | sudo tee /etc/iptables/rules.v4
    sudo ip6tables-save | sudo tee /etc/iptables/rules.v6
}

remove_iptables_rules_wg() {
    echo "removing IPtables rules..."
    echo "network device: ${network_device}"
    echo "tunnel_interface: ${wg_tunnel_interface}"
    sleep 2

    # IPv4 rules removal wg
    sudo iptables -D FORWARD -i "$wg_tunnel_interface" -o "$network_device" -j ACCEPT
    sudo iptables -D FORWARD -i "$network_device" -o "$wg_tunnel_interface" -m state --state RELATED,ESTABLISHED -j ACCEPT

    # IPv6 rules removal wg
    sudo ip6tables -t nat -D POSTROUTING -o "$network_device" -j MASQUERADE
    sudo ip6tables -D FORWARD -i "$network_device" -o "$wg_tunnel_interface" -m state --state RELATED,ESTABLISHED -j ACCEPT
    sudo ip6tables -D FORWARD -i "$wg_tunnel_interface" -o "$network_device" -j ACCEPT

    sudo iptables-save | sudo tee /etc/iptables/rules.v4
    sudo ip6tables-save | sudo tee /etc/iptables/rules.v6
}

apply_iptables_rules() {
    echo "applying IPtables rules..."
    echo "network device: ${network_device}"
    echo "tunnel_interface: ${tunnel_interface}"
    sleep 2
    sudo iptables -t nat -A POSTROUTING -o "$network_device" -j MASQUERADE
    sudo iptables -A FORWARD -i "$tunnel_interface" -o "$network_device" -j ACCEPT
    sudo iptables -A FORWARD -i "$network_device" -o "$tunnel_interface" -m state --state RELATED,ESTABLISHED -j ACCEPT
    sudo ip6tables -t nat -A POSTROUTING -o "$network_device" -j MASQUERADE
    sudo ip6tables -A FORWARD -i "$network_device" -o "$tunnel_interface" -m state --state RELATED,ESTABLISHED -j ACCEPT
    sudo ip6tables -A FORWARD -i "$tunnel_interface" -o "$network_device" -j ACCEPT
    adjust_ip_forwarding
    sudo iptables-save | sudo tee /etc/iptables/rules.v4
    sudo ip6tables-save | sudo tee /etc/iptables/rules.v6
}

remove_iptables_rules() {
    echo "removing IPtables rules..."
    echo "network device: ${network_device}"
    echo "tunnel_interface: ${tunnel_interface}"
    sleep 2

    # IPv4 rules removal
    sudo iptables -t nat -D POSTROUTING -o "$network_device" -j MASQUERADE
    sudo iptables -D FORWARD -i "$tunnel_interface" -o "$network_device" -j ACCEPT
    sudo iptables -D FORWARD -i "$network_device" -o "$tunnel_interface" -m state --state RELATED,ESTABLISHED -j ACCEPT

    # IPv6 rules removal
    sudo ip6tables -t nat -D POSTROUTING -o "$network_device" -j MASQUERADE
    sudo ip6tables -D FORWARD -i "$network_device" -o "$tunnel_interface" -m state --state RELATED,ESTABLISHED -j ACCEPT
    sudo ip6tables -D FORWARD -i "$tunnel_interface" -o "$network_device" -j ACCEPT

    sudo iptables-save | sudo tee /etc/iptables/rules.v4
    sudo ip6tables-save | sudo tee /etc/iptables/rules.v6
}

check_ipv6_ipv4_forwarding() {
    result_ipv4=$(cat /proc/sys/net/ipv4/ip_forward)
    result_ipv6=$(cat /proc/sys/net/ipv6/conf/all/forwarding)
    echo "IPv4 forwarding is $([ "$result_ipv4" == "1" ] && echo "enabled" || echo "not enabled")."
    echo "IPv6 forwarding is $([ "$result_ipv6" == "1" ] && echo "enabled" || echo "not enabled")."
}

check_nymtun_iptables() {
    echo "network Device: $network_device"
    echo "---------------------------------------"
    echo
    echo "inspecting IPv4 firewall rules..."
    iptables -L FORWARD -v -n | awk -v dev="$network_device" '/^Chain FORWARD/ || /nymtun0/ && dev || dev && /nymtun0/ || /ufw-reject-forward/'

    echo "---------------------------------------"
    echo
    echo "inspecting IPv6 firewall rules..."
    ip6tables -L FORWARD -v -n | awk -v dev="$network_device" '/^Chain FORWARD/ || /nymtun0/ && dev || dev && /nymtun0/ || /ufw6-reject-forward/'
}

joke_through_the_mixnet() {
    echo "checking Internet and mixnet connectivity (IPv4) via nymtun0..."
    ipv4_address=$(ip addr show nymtun0 | grep 'inet ' | awk '{print $2}' | cut -d'/' -f1)

    if [ -z "$ipv4_address" ]; then
        echo
        echo "no IPv4 address found on nymtun0."
        echo "please ensure IPv4 is configured correctly on your device."
        echo "unfortunately, there's no joke for you :( and you might not be able to route IPv4 traffic through your gateway to the internet."
    else
        joke=$(curl -s -H "Accept: application/json" --interface "$ipv4_address" https://icanhazdadjoke.com/ | jq -c .joke)

        if [ -z "$joke" ] || [ "$joke" = "null" ]; then
            echo "failed to fetch a joke. there might be an issue with the Internet connectivity or the joke service."
        else
            echo "joke fetched successfully:"
            echo "$joke"
        fi
    fi

    echo "checking Internet and mixnet connectivity (IPv6) via nymtun0..."
    ipv6_address=$(ip addr show nymtun0 | grep 'inet6 ' | awk '{print $2}' | cut -d'/' -f1 | grep -v '^fe80:')

    if [ -z "$ipv6_address" ]; then
        echo
        echo "no globally routable IPv6 address found on nymtun0."
        echo "please ensure IPv6 is enabled on your VPS or configure your security groups/firewall settings appropriately."
        echo "unfortunately there's no joke fo you :( and you can't route ipv6 traffic through your gateway to the internet"
    else
        joke=$(curl -s -H "Accept: application/json" --interface "$ipv6_address" https://icanhazdadjoke.com/ | jq -c .joke)

        if [ -z "$joke" ] || [ "$joke" = "null" ]; then
            echo "failed to fetch a joke. there might be an issue with the Internet connectivity or the joke service."
        else
            echo "joke fetched successfully:"
            echo "$joke"
        fi
    fi
}

check_ip6_ipv4_routing() {
    echo "---------------------------------------"
    echo "examining IPv4 routing table..."
    ip route
    echo "---------------------------------------"
    echo
    echo "examining IPv6 routing table..."
    ip -6 route
    echo
}

perform_ipv4_ipv6_pings() {
    echo "---------------------------------------"
    echo "checking IPv4 connectivity (example: google.com)..."
    ping -c 4 google.com
    echo "---------------------------------------"
    echo
    echo "checking IPv6 connectivity (example: google.com)..."
    ping6 -c 4 google.com
    echo
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
    sudo iptables-save > /etc/iptables/rules.v4

    echo "dns and icmp configuration completed."
}

joke_through_wg_tunnel() {
    echo "checking nymwg tunnel status..."

    tunnel_status=$(ip link show nymwg | grep -o "state [A-Z]*")

    if [[ $tunnel_status == "state UNKNOWN" ]]; then
        echo "nymwg tunnel is up."
    else
        echo "nymwg tunnel is down."
        echo "please check your nymwg tunnel configuration."
        return
    fi

    echo "checking internet and mixnet connectivity (ipv4) via nymwg..."
    ipv4_address=$(ip addr show nymwg | grep 'inet ' | awk '{print $2}' | cut -d'/' -f1)

    if [ -z "$ipv4_address" ]; then
        echo
        echo "no ipv4 address found on nymwg."
        echo "please ensure ipv4 is configured correctly on your device."
        echo "unfortunately, there's no joke for you :( and you might not be able to route ipv4 traffic through your gateway to the internet."
    else
        joke=$(curl -s -H "accept: application/json" --interface "$ipv4_address" https://icanhazdadjoke.com/ | jq -c .joke)

        if [ -z "$joke" ] || [ "$joke" = "null" ]; then
            echo "failed to fetch a joke. there might be an issue with the internet connectivity or the joke service."
        else
            echo "joke fetched successfully:"
            echo "$joke"
        fi
    fi

    echo "checking internet and mixnet connectivity (ipv6) via nymwg..."
    ipv6_address=$(ip addr show nymwg | grep 'inet6 ' | awk '{print $2}' | cut -d'/' -f1 | grep -v '^fe80:')

    if [ -z "$ipv6_address" ]; then
        echo
        echo "no globally routable ipv6 address found on nymwg."
        echo "please ensure ipv6 is enabled on your vps or configure your security groups/firewall settings appropriately."
        echo "unfortunately, there's no joke for you :( and you can't route ipv6 traffic through your gateway to the internet."
    else
        joke=$(curl -s -H "accept: application/json" --interface "$ipv6_address" https://icanhazdadjoke.com/ | jq -c .joke)

        if [ -z "$joke" ] || [ "$joke" = "null" ]; then
            echo "failed to fetch a joke. there might be an issue with the internet connectivity or the joke service."
        else
            echo "joke fetched successfully:"
            echo "$joke"
        fi
    fi
}

case "$1" in
fetch_ipv6_address_nym_tun)
    fetch_ipv6_address_nym_tun
    ;;
fetch_and_display_ipv6)
    fetch_and_display_ipv6
    ;;
check_nymtun_iptables)
    check_nymtun_iptables
    ;;
apply_iptables_rules)
    apply_iptables_rules
    ;;
remove_iptables_rules)
    remove_iptables_rules
    ;;
check_ipv6_ipv4_forwarding)
    check_ipv6_ipv4_forwarding
    ;;
check_ip6_ipv4_routing)
    check_ip6_ipv4_routing
    ;;
perform_ipv4_ipv6_pings)
    perform_ipv4_ipv6_pings
    ;;
joke_through_the_mixnet)
    joke_through_the_mixnet
    ;;
apply_iptables_rules_wg)
    apply_iptables_rules_wg
    ;;
joke_through_wg_tunnel)
    joke_through_wg_tunnel
    ;;
configure_dns_and_icmp_wg)
    configure_dns_and_icmp_wg
    ;;
*)
    echo "usage: $0 [command]"
    echo "commands:"
    echo "  fetch_ipv6_address_nym_tun    - Fetches the IPv6 address assigned to the '$tunnel_interface'."
    echo "  fetch_and_display_ipv6        - Displays the IPv6 address on the default network device."
    echo "  apply_iptables_rules          - Applies necessary IPv4 and IPv6 iptables rules."
    echo "  apply_iptables_rules_wg       - Applies iptable rules for IPv4 and IPv6 for Wireguard."
    echo "  remove_iptables_rules         - Removes applied IPv4 and IPv6 iptables rules."
    echo "  remove_iptables_rules_wg      - Removes applied IPv4 and IPv6 iptables rules for Wireguard."
    echo "  check_ipv6_ipv4_forwarding    - Checks if IPv4 and IPv6 forwarding are enabled."
    echo "  check_nymtun_iptables         - Check nymtun0 device."
    echo "  perform_ipv4_ipv6_pings       - Perform IPv4 and IPv6 pings to google."
    echo "  check_ip6_ipv4_routing        - Check IPv6 and IPv4 routing."
    echo "  joke_through_the_mixnet       - Run a joke through the mixnet via IPv4 and IPv6."
    echo "  joke_through_wg_tunnel        - Run a wg test, and get a joke through the wg tunnel."
    echo "  configure_dns_and_icmp_wg     - Allows icmp ping tests for probes alongside configuring dns"
    echo "please provide one of the above commands."
    exit 1
    ;;
esac

echo "operation $1 completed successfully."
