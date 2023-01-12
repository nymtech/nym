// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub(crate) fn config_template() -> &'static str {
    r#"
# This is a TOML config file.
# For more information, see https://github.com/toml-lang/toml

##### main base nym-api config options #####

[base]

# ID specifies the human readable ID of this particular nym-api.
id = '{{ base.id }}'

# Validator server to which the API will be getting information about the network.
local_validator = '{{ base.local_validator }}'

# Address announced to the directory server for the clients to connect to.
# It is useful, say, in NAT scenarios or wanting to more easily update actual IP address
# later on by using name resolvable with a DNS query, such as `nymtech.net`.
announce_address = '{{ base.announce_address }}'

# Address of the validator contract managing the network.
mixnet_contract_address = '{{ base.mixnet_contract_address }}'

# Address of the vesting contract holding locked tokens
vesting_contract_address = '{{ base.vesting_contract_address }}'

# Mnemonic used for rewarding and validator interaction
mnemonic = '{{ base.mnemonic }}'

##### network monitor config options #####

[network_monitor]
# Specifies whether network monitoring service is enabled in this process.
enabled = {{ network_monitor.enabled }}

# Indicates whether this validator api is running in a disabled credentials mode, thus attempting
# to claim bandwidth without presenting bandwidth credentials.
disabled_credentials_mode = {{ network_monitor.disabled_credentials_mode }}

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

# Specifies the minimum percentage of monitor test run data present in order to
# distribute rewards for given interval.
# Note, only values in range 0-100 are valid
minimum_interval_monitor_threshold = {{ rewarding.minimum_interval_monitor_threshold }}

[coconut_signer]

# Specifies whether coconut signing protocol is enabled in this process.
enabled = {{ coconut_signer.enabled }}

# Path to the coconut verification key
verification_key_path = '{{ coconut_signer.verification_key_path }}'

# Path to the coconut verification key
secret_key_path = '{{ coconut_signer.secret_key_path }}'

# Path to the dkg dealer decryption key
decryption_key_path = '{{ coconut_signer.decryption_key_path }}'

# Path to the dkg dealer public key with proof
public_key_with_proof_path = '{{ coconut_signer.public_key_with_proof_path }}'

"#
}
