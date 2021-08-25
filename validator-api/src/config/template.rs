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

# Mnemonic (currently of the network monitor) used for rewarding
mnemonic = '{{ base.mnemonic }}'

##### network monitor config options #####

[network_monitor]

# Specifies whether network monitoring service is enabled in this process.
enabled = {{ network_monitor.enabled }}

# Specifies list of all validators on the network issuing coconut credentials.
# A special care must be taken to ensure they are in correct order.
# The list must also contain THIS validator that is running the test
all_validator_apis = [
    {{#each network_monitor.all_validator_apis }}
        '{{this}}',
    {{/each}}
]

# Specifies whether a detailed report should be printed after each run
print_detailed_report = {{ network_monitor.print_detailed_report }}

# Location of .json file containing IPv4 'good' network topology
good_v4_topology_file = '{{ network_monitor.good_v4_topology_file }}'

# Location of .json file containing IPv6 'good' network topology
good_v6_topology_file = '{{ network_monitor.good_v6_topology_file }}'

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
    
[node_status_api]

# Path to the database file containing uptime statuses for all mixnodes and gateways.
database_path = '{{ node_status_api.database_path }}'

##### rewarding config options #####

[rewarding]

# Specifies whether rewarding service is enabled in this process.
enabled = {{ rewarding.enabled }}

# Mnemonic (currently of the network monitor) used for rewarding
mnemonic = '{{ rewarding.mnemonic }}'

# Datetime of the first rewarding epoch of the current length used for referencing
# starting time of any subsequent epoch.
first_rewarding_epoch = '{{ rewarding.first_rewarding_epoch }}'

# Current length of the epoch. If modified `first_rewarding_epoch` should also get changed.
epoch_length = '{{ rewarding.epoch_length }}'

"#
}
