#!/bin/bash

validate_exit_policy() {
    echo "=== Nym Exit Policy Blocking Validation ==="

    # Check iptables rules
    echo "Checking iptables NYM-EXIT chain:"
    sudo iptables -L NYM-EXIT -v -n

    # Test IP ranges and individual IPs
    test_ips=(
        "5.188.10.0/24"    # Blocked network range
        "31.132.36.50"     # Specific blocked IP
        "37.9.42.100"      # Another blocked IP
    )

    for target in "${test_ips[@]}"; do
        echo -e "\n\e[33mTesting blocking for $target\e[0m"

        # Multiple connection test methods
        methods=(
            "ping -c 4 -W 2"
            "curl -m 5 http://$target"
            "nc -z -w 5 $target 80"
            "telnet $target 80"
        )

        for method in "${methods[@]}"; do
            echo -n "Testing with $method: "
            if sudo timeout 5 $method >/dev/null 2>&1; then
                echo -e "\e[31mFAILED: Connection succeeded (Blocking ineffective)\e[0m"
            else
                echo -e "\e[32mBLOCKED\e[0m"
            fi
        done
    done
}

# Run the test
validate_exit_policy