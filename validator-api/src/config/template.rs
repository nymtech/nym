// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub(crate) fn config_template() -> &'static str {
    r#"
# This is a TOML config file.
# For more information, see https://github.com/toml-lang/toml

##### main base validator-api config options #####

[base]

# Validator server to which the API will be getting information about the network.
validator_rest_urls = [
    {{#each base.validator_rest_urls }}
        '{{this}}',
    {{/each}}
]

# Address of the validator contract managing the network.
mixnet_contract_address = '{{ base.mixnet_contract_address }}'

##### network monitor config options #####

[network_monitor]

# Specifies whether network monitoring service is enabled in this process.
enabled = {{ network_monitor.enabled }}

# Specifies whether a detailed report should be printed after each run
print_detailed_report = {{ network_monitor.print_detailed_report }}

# Location of .json file containing IPv4 'good' network topology
good_v4_topology_file = '{{ network_monitor.good_v4_topology_file }}'

# Location of .json file containing IPv6 'good' network topology
good_v6_topology_file = '{{ network_monitor.good_v6_topology_file }}'

# Address of the node status api to submit results to. Most likely it's a local address
node_status_api_url = '{{ network_monitor.node_status_api_url }}'

# Specifies maximum rate (in packets per second) of test packets being sent to gateway
gateway_sending_rate = {{ network_monitor.gateway_sending_rate }}

# Maximum number of gateway clients the network monitor will try to talk to concurrently.
max_concurrent_gateway_clients = {{ network_monitor.max_concurrent_gateway_clients }}

# Maximum allowed time for receiving gateway response.
gateway_response_timeout = '{{ network_monitor.gateway_response_timeout }}'

# Maximum allowed time for the gateway connection to get established.
gateway_connection_timeout = '{{ network_monitor.gateway_connection_timeout }}'


"#
}
