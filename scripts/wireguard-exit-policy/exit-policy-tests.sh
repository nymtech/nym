#!/bin/bash
# Nym Exit Policy Verification Unit Tests

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

NYM_CHAIN="NYM-EXIT"
WG_INTERFACE="nymwg"

# Function to check port range rules - with sampling for large ranges
check_port_range_rules() {
    local port_range="$1"
    local protocol="${2:-tcp}"
    local chain="${3:-$NYM_CHAIN}"
    local start_port=$(echo "$port_range" | cut -d'-' -f1)
    local end_port=$(echo "$port_range" | cut -d'-' -f2)
    local range_size=$((end_port - start_port + 1))
    local failures=0
    local sample_size=5 
    local step=1

    # For large ranges, test only a sample of ports
    if [ $range_size -gt $sample_size ]; then
        step=$((range_size / sample_size))
        # Ensure we test the start and end ports
        local sample_ports=($start_port $end_port)

        for ((i=1; i<sample_size-1; i++)); do
            sample_ports+=($((start_port + i*step)))
        done

        for port in "${sample_ports[@]}"; do
            if [ $port -le $end_port ]; then  # Ensure we don't go out of range
                if ! iptables -t filter -C "$chain" -p "$protocol" --dport "$port" -j ACCEPT 2>/dev/null; then
                    echo -e "${RED}✗ Rule missing: $chain $protocol port $port${NC}"
                    ((failures++))
                fi
            fi
        done
    else
        for ((port=start_port; port<=end_port; port++)); do
            if ! iptables -t filter -C "$chain" -p "$protocol" --dport "$port" -j ACCEPT 2>/dev/null; then
                echo -e "${RED}✗ Rule missing: $chain $protocol port $port${NC}"
                ((failures++))
            fi
        done
    fi

    if [ $failures -eq 0 ]; then
        echo -e "${GREEN}✓ Rule exists: $chain $protocol port range $port_range${NC}"
        return 0
    else
        echo -e "${RED}✗ Some rules missing in port range $port_range${NC}"
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
    )

    local total_failures=0

    for range in "${port_ranges[@]}"; do
        IFS=':' read -r port_range protocol service <<< "$range"

        echo -e "${YELLOW}Testing $service $protocol port range $port_range${NC}"

        check_port_range_rules "$port_range" "$protocol"
        failures=$?

        total_failures=$((total_failures + failures))
    done

    return $total_failures
}

# Test critical services
test_critical_services() {
    echo -e "${YELLOW}Testing Critical Service Rules...${NC}"

    local tcp_services=(
        22      # SSH
        53      # DNS
        80      # HTTP
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
        if ! iptables -t filter -C "$NYM_CHAIN" -p tcp --dport "$port" -j ACCEPT 2>/dev/null; then
            echo -e "${RED}✗ Rule missing: NYM-EXIT tcp port $port${NC}"
            ((failures++))
        else
            echo -e "${GREEN}✓ Rule exists: NYM-EXIT tcp port $port${NC}"
        fi
    done

    # Test UDP services
    for port in "${udp_services[@]}"; do
        if ! iptables -t filter -C "$NYM_CHAIN" -p udp --dport "$port" -j ACCEPT 2>/dev/null; then
            echo -e "${RED}✗ Rule missing: NYM-EXIT udp port $port${NC}"
            ((failures++))
        else
            echo -e "${GREEN}✓ Rule exists: NYM-EXIT udp port $port${NC}"
        fi
    done

    return $failures
}

# Verify default reject rule exists - with improved detection
test_default_reject_rule() {
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