// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub(crate) fn config_template() -> &'static str {
    r#"
# This is a TOML config file.
# For more information, see https://github.com/toml-lang/toml

##### main base validator-api config options #####

[base]

# Validator server to which the API will be getting information about the network.
local_validator = '{{ base.local_validator }}'

# Address of the validator contract managing the network.
mixnet_contract_address = '{{ base.mixnet_contract_address }}'

##### network monitor config options #####

[network_monitor]
# Specifies whether network monitoring service is enabled in this process.
enabled = {{ network_monitor.enabled }}

# Indicates whether this validator api is running in a testnet mode, thus attempting
# to claim bandwidth without presenting bandwidth credentials.
testnet_mode = {{ network_monitor.testnet_mode }}

# Specifies list of all validators on the network issuing coconut credentials.
# A special care must be taken to ensure they are in correct order.
# The list must also contain THIS validator that is running the test
all_validator_apis = [
    {{#each network_monitor.all_validator_apis }}
        '{{this}}',
    {{/each}}
]

# Specifies the interval at which the network monitor sends the test packets.
run_interval = '{{ network_monitor.run_interval }}'

# Specifies interval at which we should be sending ping packets to all active gateways
# in order to keep the websocket connections alive.
gateway_ping_interval = '{{ network_monitor.gateway_ping_interval }}'

# Specifies maximum rate (in packets per second) of test packets being sent to gateway
gateway_sending_rate = {{ network_monitor.gateway_sending_rate }}

# Maximum number of gateway clients the network monitor will try to talk to concurrently.
max_concurrent_gateway_clients = {{ network_monitor.max_concurrent_gateway_clients }}

# Maximum allowed time for receiving gateway response.
gateway_response_timeout = '{{ network_monitor.gateway_response_timeout }}'

# Maximum allowed time for the gateway connection to get established.
gateway_connection_timeout = '{{ network_monitor.gateway_connection_timeout }}'

# Specifies the duration the monitor is going to wait after sending all measurement
# packets before declaring nodes unreachable.
packet_delivery_timeout = '{{ network_monitor.packet_delivery_timeout }}'

credentials_database_path = '{{ network_monitor.credentials_database_path }}'

# Ethereum private key.
eth_private_key = '{{ network_monitor.eth_private_key }}'

# Addess to an Ethereum full node.
eth_endpoint = '{{ network_monitor.eth_endpoint }}'

# Desired number of test routes to be constructed (and working) during a monitor test run.
test_routes = {{ network_monitor.test_routes }}

# The minimum number of test routes that need to be constructed (and working) in order for
# a monitor test run to be valid.
minimum_test_routes = {{ network_monitor.minimum_test_routes }}

# Number of test packets sent via each pseudorandom route to verify whether they work correctly,
# before using them for testing the rest of the network.
route_test_packets = {{ network_monitor.route_test_packets }}

# Number of test packets sent to each node during regular monitor test run.
per_node_test_packets = {{ network_monitor.per_node_test_packets }}
    
[node_status_api]

# Path to the database file containing uptime statuses for all mixnodes and gateways.
database_path = '{{ node_status_api.database_path }}'

##### rewarding config options #####

[rewarding]

# Specifies whether rewarding service is enabled in this process.
enabled = {{ rewarding.enabled }}

# Mnemonic (currently of the network monitor) used for rewarding
mnemonic = '{{ rewarding.mnemonic }}'

# Specifies the minimum percentage of monitor test run data present in order to
# distribute rewards for given interval.
# Note, only values in range 0-100 are valid
minimum_interval_monitor_threshold = {{ rewarding.minimum_interval_monitor_threshold }}

"#
}
