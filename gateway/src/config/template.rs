// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub(crate) fn config_template() -> &'static str {
    // While using normal toml marshalling would have been way simpler with less overhead,
    // I think it's useful to have comments attached to the saved config file to explain behaviour of
    // particular fields.
    // Note: any changes to the template must be reflected in the appropriate structs in mod.rs.
    r#"
# This is a TOML config file.
# For more information, see https://github.com/toml-lang/toml

##### main base mixnode config options #####

[gateway]
# Version of the gateway for which this configuration was created.
version = '{{ gateway.version }}'

# Human readable ID of this particular gateway.
id = '{{ gateway.id }}'

# Path to file containing private identity key.
private_identity_key_file = '{{ gateway.private_identity_key_file }}'

# Path to file containing public identity key.
public_identity_key_file = '{{ gateway.public_identity_key_file }}'

# Path to file containing private sphinx key.
private_sphinx_key_file = '{{ gateway.private_sphinx_key_file }}'

# Path to file containing public sphinx key.
public_sphinx_key_file = '{{ gateway.public_sphinx_key_file }}'

# Validator server to which the node will be reporting their presence data.
validator_rest_urls = [
    {{#each gateway.validator_rest_urls }}
        '{{this}}',
    {{/each}}
]

# Address of the validator contract managing the network.
mixnet_contract_address = '{{ gateway.mixnet_contract_address }}'

# nym_home_directory specifies absolute path to the home nym gateway directory.
# It is expected to use default value and hence .toml file should not redefine this field.
nym_root_directory = '{{ gateway.nym_root_directory }}'
    

##### Mixnet endpoint config options #####

[mixnet_endpoint]
# Socket address to which this gateway will bind to
# and will be listening for sphinx packets coming from the mixnet.
listening_address = '{{ mixnet_endpoint.listening_address }}'

# Optional address announced to the directory server for the clients to connect to.
# It is useful, say, in NAT scenarios or wanting to more easily update actual IP address
# later on by using name resolvable with a DNS query, such as `nymtech.net:8080`.
# Additionally a custom port can be provided, so both `nymtech.net:8080` and `nymtech.net`
# are valid announce addresses, while the later will default to whatever port is used for
# `listening_address`.
announce_address = '{{ mixnet_endpoint.announce_address }}'


#### Clients endpoint config options #####

[clients_endpoint]
# Socket address to which this gateway will bind to
# and will be listening for sphinx packets coming from the mixnet.
listening_address = '{{ clients_endpoint.listening_address }}'

# Optional address announced to the directory server for the clients to connect to.
# It is useful, say, in NAT scenarios or wanting to more easily update actual IP address
# later on by using name resolvable with a DNS query, such as `nymtech.net:8080`.
# Additionally a custom port can be provided, so both `nymtech.net:8080` and `nymtech.net`
# are valid announce addresses, while the later will default to whatever port is used for
# `listening_address`.
announce_address = '{{ clients_endpoint.announce_address }}'

# Path to the directory with clients inboxes containing messages stored for them.
inboxes_directory = '{{ clients_endpoint.inboxes_directory }}'

# Full path to a file containing mapping of client addresses to their access tokens.
ledger_path = '{{ clients_endpoint.ledger_path }}'


##### logging configuration options #####

[logging]

# TODO

"#
}
