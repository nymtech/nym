#!/bin/bash
# Nym Exit Policy Verification Unit Tests

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m'

NYM_CHAIN="NYM-EXIT"
WG_INTERFACE="nymwg"

check_port_range_rules() {
    local port_range="$1"
    local protocol="${2:-tcp}"
    local chain="${3:-$NYM_CHAIN}"

    # Extract start and end ports
    local start_port=$(echo "$port_range" | cut -d'-' -f1)
    local end_port=$(echo "$port_range" | cut -d'-' -f2)

    if iptables -t filter -C "$chain" -p "$protocol" --dport "$start_port:$end_port" -j ACCEPT 2>/dev/null; then
        echo -e "${GREEN}✓ Rule exists: $chain $protocol port range $start_port:$end_port${NC}"
        return 0
    else
        echo -e "${RED}✗ Rule missing: $chain $protocol port range $start_port:$end_port${NC}"

        echo -e "${YELLOW}Dumping all rules in $chain:${NC}"
        iptables -L "$chain" -n | grep "$protocol"

        return 1
    fi
}

# Test port range rules
test_port_range_rules() {
    echo -e "${YELLOW}Testing Port Range Rules...${NC}"

    # Select the essential port ranges for testing
    local port_ranges=(
        "20-21:tcp:FTP"
        "80-81:tcp:HTTP"
        "2082-2083:tcp:CPanel"
        "5222-5223:tcp:XMPP"
        "27000-27050:tcp:Steam (sampling)"
        "989-990:tcp:FTP over TLS"
        "5000-5005:tcp:RTP/VoIP"
        "8087-8088:tcp:Simplify Media"
        "8232-8233:tcp:Zcash"
        "8332-8333:tcp:Bitcoin"
	    "18080-18081:tcp:Monero"
    )

    local total_failures=0

    for range in "${port_ranges[@]}"; do
        IFS=':' read -r port_range protocol service <<< "$range"

        # Extract start and end ports
        local start_port=$(echo "$port_range" | cut -d'-' -f1)
        local end_port=$(echo "$port_range" | cut -d'-' -f2)

        echo -e "${YELLOW}Testing $service $protocol port range $port_range${NC}"

        if iptables -t filter -C "$NYM_CHAIN" -p "$protocol" --dport "$start_port:$end_port" -j ACCEPT 2>/dev/null; then
            echo -e "${GREEN}✓ Rule exists: $NYM_CHAIN $protocol port range $start_port:$end_port${NC}"
        else
            echo -e "${RED}✗ Rule missing: $NYM_CHAIN $protocol port range $start_port:$end_port${NC}"
            ((total_failures++))

            echo -e "${YELLOW}Existing rules for protocol $protocol:${NC}"
            iptables -L "$NYM_CHAIN" -n | grep "$protocol"
        fi
    done

    if [ $total_failures -eq 0 ]; then
        return 0
    else
        return 1
    fi
}

test_critical_services() {
    echo -e "${YELLOW}Testing Critical Service Rules...${NC}"

    local tcp_services=(
        22      # SSH
        53      # DNS
        443     # HTTPS
        853     # DNS over TLS
        1194    # OpenVPN
    )

    local udp_services=(
        53      # DNS
        123     # NTP
        1194    # OpenVPN
    )

    local failures=0

    # Test TCP services
    for port in "${tcp_services[@]}"; do
        local rule_found=false

        # First check for exact match
        if iptables -t filter -C "$NYM_CHAIN" -p tcp --dport "$port" -j ACCEPT 2>/dev/null; then
            echo -e "${GREEN}✓ Rule exists: NYM-EXIT tcp port $port${NC}"
            rule_found=true
        else
            # If not found as exact port, search for it in port ranges
            # This checks if the port is covered by any range rule
            if iptables-save | grep -E "^-A $NYM_CHAIN.*tcp.*dpts:" | grep -qP "dpts:(\d+:)?$port(:|\d+)" || \
               iptables-save | grep -E "^-A $NYM_CHAIN.*tcp.*dpts:" | grep -qP "dpts:$port:"; then
                echo -e "${GREEN}✓ Rule exists: NYM-EXIT tcp port $port (covered by a range rule)${NC}"
                rule_found=true
            else
                echo -e "${RED}✗ Rule missing: NYM-EXIT tcp port $port${NC}"
                ((failures++))
            fi
        fi
    done

    # Test UDP services - similar approach
    for port in "${udp_services[@]}"; do
        local rule_found=false

        if iptables -t filter -C "$NYM_CHAIN" -p udp --dport "$port" -j ACCEPT 2>/dev/null; then
            echo -e "${GREEN}✓ Rule exists: NYM-EXIT udp port $port${NC}"
            rule_found=true
        else
            # If not found as exact port, search for it in port ranges
            if iptables-save | grep -E "^-A $NYM_CHAIN.*udp.*dpts:" | grep -qP "dpts:(\d+:)?$port(:|\d+)" || \
               iptables-save | grep -E "^-A $NYM_CHAIN.*udp.*dpts:" | grep -qP "dpts:$port:"; then
                echo -e "${GREEN}✓ Rule exists: NYM-EXIT udp port $port (covered by a range rule)${NC}"
                rule_found=true
            else
                echo -e "${RED}✗ Rule missing: NYM-EXIT udp port $port${NC}"
                ((failures++))
            fi
        fi
    done

    echo -e "${YELLOW}Relevant existing rules for HTTP (port 80):${NC}"
    iptables-save | grep -E "$NYM_CHAIN.*tcp" | grep -E "(dpt|dpts):.*80"

    return $failures
}

# Test that the exit policy chain is correctly wired up to FORWARD chain
test_forward_chain_hook() {
    echo -e "${YELLOW}Testing FORWARD Chain Hook...${NC}"

    local failures=0
    NETWORK_DEVICE=$(ip route show default | awk '/default/ {print $5}')

    if [[ -z "$NETWORK_DEVICE" ]]; then
        echo -e "${RED}✗ Could not determine network device${NC}"
        return 1
    fi

    # (incoming from nymwg, outgoing to network device)
    if iptables -C FORWARD -i "$WG_INTERFACE" -o "$NETWORK_DEVICE" -j "$NYM_CHAIN" 2>/dev/null; then
        echo -e "${GREEN}✓ IPv4 FORWARD hook exists with correct direction: -i $WG_INTERFACE -o $NETWORK_DEVICE${NC}"
    else
        echo -e "${RED}✗ IPv4 FORWARD hook missing or has wrong direction${NC}"
        echo -e "${YELLOW}  Expected: -i $WG_INTERFACE -o $NETWORK_DEVICE -j $NYM_CHAIN${NC}"
        
        # Check if wrong direction exists
        if iptables -C FORWARD -o "$WG_INTERFACE" -j "$NYM_CHAIN" 2>/dev/null; then
            echo -e "${RED}✗  WRONG DIRECTION FOUND: -o $WG_INTERFACE (this matches return traffic, not client egress!)${NC}"
        fi
        
        # Show what actually exists - specifically look for jump rules to NYM-EXIT
        echo -e "${YELLOW}  Current FORWARD rules that jump to $NYM_CHAIN:${NC}"
        iptables -L FORWARD -n -v --line-numbers | grep -E "$NYM_CHAIN" || echo "    (none found - exit policy chain is not hooked to FORWARD!)"
        echo -e "${YELLOW}  All FORWARD rules with $WG_INTERFACE (for reference):${NC}"
        iptables -L FORWARD -n -v --line-numbers | grep -E "$WG_INTERFACE" | head -5 || echo "    (none found)"
        ((failures++))
    fi

    # Check IPv6
    if ip6tables -C FORWARD -i "$WG_INTERFACE" -o "$NETWORK_DEVICE" -j "$NYM_CHAIN" 2>/dev/null; then
        echo -e "${GREEN}✓ IPv6 FORWARD hook exists with correct direction: -i $WG_INTERFACE -o $NETWORK_DEVICE${NC}"
    else
        echo -e "${RED}✗ IPv6 FORWARD hook missing or has wrong direction${NC}"
        echo -e "${YELLOW}  Expected: -i $WG_INTERFACE -o $NETWORK_DEVICE -j $NYM_CHAIN${NC}"
        
        # Check if wrong direction exists
        if ip6tables -C FORWARD -o "$WG_INTERFACE" -j "$NYM_CHAIN" 2>/dev/null; then
            echo -e "${RED}✗ WRONG DIRECTION FOUND: -o $WG_INTERFACE (this matches return traffic, not client egress!)${NC}"
        fi
        
        # Show what actually exists - specifically look for jump rules to NYM-EXIT
        echo -e "${YELLOW}  Current IPv6 FORWARD rules that jump to $NYM_CHAIN:${NC}"
        ip6tables -L FORWARD -n -v --line-numbers | grep -E "$NYM_CHAIN" || echo "    (none found - exit policy chain is not hooked to FORWARD!)"
        echo -e "${YELLOW}  All IPv6 FORWARD rules with $WG_INTERFACE (for reference):${NC}"
        ip6tables -L FORWARD -n -v --line-numbers | grep -E "$WG_INTERFACE" | head -5 || echo "    (none found)"
        ((failures++))
    fi

    # Check rule position (should be before UFW rules)
    local rule_num=$(iptables -L FORWARD -n --line-numbers | grep -E "$NYM_CHAIN.*$WG_INTERFACE.*$NETWORK_DEVICE" | awk '{print $1}' | head -1)
    if [[ -n "$rule_num" ]]; then
        if [[ $rule_num -le 5 ]]; then
            echo -e "${GREEN}✓ Rule is early in FORWARD chain (position #$rule_num) - good for UFW compatibility${NC}"
        else
            echo -e "${YELLOW}⚠ Rule is later in FORWARD chain (position #$rule_num) - may conflict with UFW${NC}"
        fi
    fi

    return $failures
}

# Verify default reject rule exists
test_default_reject_rule() {
    echo -e "${YELLOW}This test takes some time, do not quit the process${NC}"
    echo
    echo -e "${YELLOW}Testing Default Reject Rule...${NC}"

    # Try different patterns to detect the reject rule
    if iptables -L "$NYM_CHAIN" | grep -q "REJECT.*all.*anywhere.*anywhere.*reject-with"; then
        echo -e "${GREEN}✓ Default REJECT rule exists${NC}"
        return 0
    elif iptables -L "$NYM_CHAIN" | grep -q "REJECT.*all  --  .*everywhere.*everywhere"; then
        echo -e "${GREEN}✓ Default REJECT rule exists${NC}"
        return 0
    elif iptables -L "$NYM_CHAIN" | grep -q "REJECT.*all.*0.0.0.0/0.*0.0.0.0/0"; then
        echo -e "${GREEN}✓ Default REJECT rule exists${NC}"
        return 0
    elif iptables -n -L "$NYM_CHAIN" | grep -qE "REJECT.*all.*0.0.0.0/0.*0.0.0.0/0"; then
        echo -e "${GREEN}✓ Default REJECT rule exists${NC}"
        return 0
    elif iptables -L "$NYM_CHAIN" | tail -1 | grep -q "REJECT"; then
        echo -e "${GREEN}✓ Default REJECT rule exists at the end of chain${NC}"
        return 0
    else
        echo -e "${RED}✗ Default REJECT rule missing${NC}"
        # Display the last 3 rules in the chain for debugging
        echo -e "${YELLOW}Last 3 rules in the chain:${NC}"
        iptables -L "$NYM_CHAIN" | tail -3
        return 1
    fi
}

run_all_tests() {
    local total_failures=0
    local total_tests=0
    local skip_default_reject=false

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --skip-default-reject)
                skip_default_reject=true
                shift
                ;;
            *)
                echo -e "${RED}Unknown argument: $1${NC}"
                exit 1
                ;;
        esac
    done

    local test_functions=(
        "test_forward_chain_hook"
        "test_port_range_rules"
        "test_critical_services"
    )

    if [ "$skip_default_reject" = false ]; then
        test_functions+=("test_default_reject_rule")
    fi

    echo -e "${YELLOW}Running Nym Exit Policy Verification Tests...${NC}"

    for test_func in "${test_functions[@]}"; do
        ((total_tests++))
        $test_func
        if [ $? -ne 0 ]; then
            ((total_failures++))
            echo -e "${RED}Test $test_func FAILED${NC}"
        else
            echo -e "${GREEN}Test $test_func PASSED${NC}"
        fi
    done

    echo -e "\n${YELLOW}Test Summary:${NC}"
    echo -e "Total Tests:     $total_tests"
    echo -e "Failures:        $total_failures"

    if [ $total_failures -eq 0 ]; then
        echo -e "${GREEN}All Tests Passed Successfully!${NC}"
        exit 0
    else
        echo -e "${RED}Some Tests Failed. Please review the iptables configuration.${NC}"
        exit 1
    fi
}

if [[ $EUID -ne 0 ]]; then
   echo -e "${RED}This script must be run as root${NC}"
   exit 1
fi

# Run the tests
run_all_tests "$@"
