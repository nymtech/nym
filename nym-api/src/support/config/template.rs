// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

pub(crate) const CONFIG_TEMPLATE: &str = r#"
# This is a TOML config file.
# For more information, see https://github.com/toml-lang/toml

##### main base nym-api config options #####

[base]

# ID specifies the human readable ID of this particular nym-api.
id = '{{ base.id }}'

# Validator server to which the API will be getting information about the network.
local_validator = '{{ base.local_validator }}'

# Socket address this api will use for binding its http API.
# default: `0.0.0.0:8080`
bind_address = '{{ base.bind_address }}'

# Mnemonic used for rewarding and validator interaction
mnemonic = '{{ base.mnemonic }}'

[base.storage_paths]
# Path to file containing private identity key of the nym-api.
private_identity_key_file = '{{ base.storage_paths.private_identity_key_file }}'

# Path to file containing public identity key of the nym-api.
public_identity_key_file = '{{ base.storage_paths.public_identity_key_file }}'

##### network monitor config options #####

[network_monitor]
# Specifies whether network monitoring service is enabled in this process.
enabled = {{ network_monitor.enabled }}

[network_monitor.storage_paths]

# Path to the database containing bandwidth credentials of this client.
credentials_database_path = '{{ network_monitor.storage_paths.credentials_database_path }}'

[network_monitor.debug]

# Indicates whether this validator api is running in a disabled credentials mode, thus attempting
# to claim bandwidth without presenting bandwidth credentials.
disabled_credentials_mode = {{ network_monitor.debug.disabled_credentials_mode }}

# Specifies the interval at which the network monitor sends the test packets.
run_interval = '{{ network_monitor.debug.run_interval }}'

# Desired number of test routes to be constructed (and working) during a monitor test run.
test_routes = {{ network_monitor.debug.test_routes }}

# The minimum number of test routes that need to be constructed (and working) in order for
# a monitor test run to be valid.
minimum_test_routes = {{ network_monitor.debug.minimum_test_routes }}

# Number of test packets sent via each pseudorandom route to verify whether they work correctly,
# before using them for testing the rest of the network.
route_test_packets = {{ network_monitor.debug.route_test_packets }}

# Number of test packets sent to each node during regular monitor test run.
per_node_test_packets = {{ network_monitor.debug.per_node_test_packets }}
    

##### node status api config options #####

[node_status_api.storage_paths]

# Path to the database file containing uptime statuses for all mixnodes and gateways.
database_path = '{{ node_status_api.storage_paths.database_path }}'

[node_status_api.debug]

caching_interval = '{{ node_status_api.debug.caching_interval }}'


##### topology cacher config options #####

[topology_cacher.debug]

caching_interval = '{{ topology_cacher.debug.caching_interval }}'


##### circulating supply cacher config options #####

[circulating_supply_cacher]

# Specifies whether circulating supply caching service is enabled in this process.
enabled = {{ circulating_supply_cacher.enabled }}

[circulating_supply_cacher.debug]

caching_interval = '{{ circulating_supply_cacher.debug.caching_interval }}'


##### rewarding config options #####

[rewarding]

# Specifies whether rewarding service is enabled in this process.
enabled = {{ rewarding.enabled }}

[rewarding.debug]

# Specifies the minimum percentage of monitor test run data present in order to
# distribute rewards for given interval.
# Note, only values in range 0-100 are valid
minimum_interval_monitor_threshold = {{ rewarding.debug.minimum_interval_monitor_threshold }}

[coconut_signer]

# Specifies whether coconut signing protocol is enabled in this process.
enabled = {{ coconut_signer.enabled }}

# address of this nym-api as announced to other instances for the purposes of performing the DKG.
announce_address = '{{ coconut_signer.announce_address }}'

[coconut_signer.storage_paths]

# Path to a JSON file where state is persisted between different stages of DKG.
dkg_persistent_state_path = '{{ coconut_signer.storage_paths.dkg_persistent_state_path }}'

# Path to the coconut key.
coconut_key_path = '{{ coconut_signer.storage_paths.coconut_key_path }}'

# Path to the dkg dealer decryption key
decryption_key_path = '{{ coconut_signer.storage_paths.decryption_key_path }}'

# Path to the dkg dealer public key with proof
public_key_with_proof_path = '{{ coconut_signer.storage_paths.public_key_with_proof_path }}'

"#;
