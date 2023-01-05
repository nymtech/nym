// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub(crate) fn config_template() -> &'static str {
    // While using normal toml marshalling would have been way simpler with less overhead,
    // I think it's useful to have comments attached to the saved config file to explain behaviour of
    // particular fields.
    // Note: any changes to the template must be reflected in the appropriate structs in verloc.
    r#"
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

# Path to file containing private identity key.
private_identity_key_file = '{{ gateway.private_identity_key_file }}'

# Path to file containing public identity key.
public_identity_key_file = '{{ gateway.public_identity_key_file }}'

# Path to file containing private sphinx key.
private_sphinx_key_file = '{{ gateway.private_sphinx_key_file }}'

# Path to file containing public sphinx key.
public_sphinx_key_file = '{{ gateway.public_sphinx_key_file }}'

##### additional gateway config options #####

# Optional address announced to the directory server for the clients to connect to.
# It is useful, say, in NAT scenarios or wanting to more easily update actual IP address
# later on by using name resolvable with a DNS query, such as `nymtech.net`.
announce_address = '{{ gateway.announce_address }}'

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

cosmos_mnemonic = "{{ gateway.cosmos_mnemonic }}"

# Nym wallet address on the blockchain that should control this gateway
wallet_address = '{{ gateway.wallet_address }}'

##### advanced configuration options #####

# nym_home_directory specifies absolute path to the home nym gateway directory.
# It is expected to use default value and hence .toml file should not redefine this field.
nym_root_directory = '{{ gateway.nym_root_directory }}'

# Path to sqlite database containing all persistent data: messages for offline clients,
# derived shared keys and available client bandwidths.
persistent_storage = '{{ gateway.persistent_storage }}'

##### logging configuration options #####

[logging]

# TODO

"#
}
