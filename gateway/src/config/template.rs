// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// While using normal toml marshalling would have been way simpler with less overhead,
// I think it's useful to have comments attached to the saved config file to explain behaviour of
// particular fields.
// Note: any changes to the template must be reflected in the appropriate structs.
pub(crate) const CONFIG_TEMPLATE: &str = r#"
# This is a TOML config file.
# For more information, see https://github.com/toml-lang/toml

##### main base mixnode config options #####

[gateway]
# Version of the gateway for which this configuration was created.
version = '{{ gateway.version }}'

# Human readable ID of this particular gateway.
id = '{{ gateway.id }}'

# Indicates whether this gateway is accepting only coconut credentials for accessing the
# the mixnet, or if it also accepts non-paying clients
only_coconut_credentials = {{ gateway.only_coconut_credentials }}

# Socket address to which this gateway will bind to and will be listening for packets.
listening_address = '{{ gateway.listening_address }}'

# Port used for listening for all mixnet traffic.
# (default: 1789)
mix_port = {{ gateway.mix_port }}

# Port used for listening for all client websocket traffic.
# (default: 9000)
clients_port = {{ gateway.clients_port }}

# Wheather gateway collects and sends anonymized statistics
enabled_statistics = {{ gateway.enabled_statistics }}

# Domain address of the statistics service
statistics_service_url = '{{ gateway.statistics_service_url }}'

# Addresses to APIs running on validator from which the node gets the view of the network.
nym_api_urls = [
    {{#each gateway.nym_api_urls }}
        '{{this}}',
    {{/each}}
]

# Addresses to validators which the node uses to check for double spending of nym tokens.
nyxd_urls = [
    {{#each gateway.nyxd_urls }}
        '{{this}}',
    {{/each}}
]

cosmos_mnemonic = '{{ gateway.cosmos_mnemonic }}'

[network_requester]
# Specifies whether network requester service is enabled in this process.
enabled = {{ network_requester.enabled }}

[storage_paths] 

# Path to file containing private identity key.
keys.private_identity_key_file = '{{ storage_paths.keys.private_identity_key_file }}'

# Path to file containing public identity key.
keys.public_identity_key_file = '{{ storage_paths.keys.public_identity_key_file }}'

# Path to file containing private identity key.
keys.private_sphinx_key_file = '{{ storage_paths.keys.private_sphinx_key_file }}'

# Path to file containing public sphinx key.
keys.public_sphinx_key_file = '{{ storage_paths.keys.public_sphinx_key_file }}'

# Path to sqlite database containing all persistent data: messages for offline clients,
# derived shared keys and available client bandwidths.
clients_storage = '{{ storage_paths.clients_storage }}'

# Path to the configuration of the locally running network requester.
network_requester_config = '{{ storage_paths.network_requester_config }}'

##### logging configuration options #####

[logging]

# TODO

"#;
