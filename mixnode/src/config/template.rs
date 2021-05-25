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

[mixnode]
# Version of the mixnode for which this configuration was created.
version = '{{ mixnode.version }}'
    
# Human readable ID of this particular mixnode.
id = '{{ mixnode.id }}'
   
# Layer of this particular mixnode determining its position in the network.
layer = {{ mixnode.layer }}

# Socket address to which this mixnode will bind to and will be listening for packets.
listening_address = '{{ mixnode.listening_address }}'

# Path to file containing private identity key.
private_identity_key_file = '{{ mixnode.private_identity_key_file }}'

# Path to file containing public identity key.
public_identity_key_file = '{{ mixnode.public_identity_key_file }}'

# Path to file containing private identity key.
private_sphinx_key_file = '{{ mixnode.private_sphinx_key_file }}'

# Path to file containing public sphinx key.
public_sphinx_key_file = '{{ mixnode.public_sphinx_key_file }}'

##### additional mixnode config options #####

# Optional address announced to the directory server for the clients to connect to.
# It is useful, say, in NAT scenarios or wanting to more easily update actual IP address
# later on by using name resolvable with a DNS query, such as `nymtech.net:8080`.
# Additionally a custom port can be provided, so both `nymtech.net:8080` and `nymtech.net`
# are valid announce addresses, while the later will default to whatever port is used for
# `listening_address`.
announce_address = '{{ mixnode.announce_address }}'

# Validator server to which the node will be reporting their presence data.
validator_rest_urls = [
    {{#each mixnode.validator_rest_urls }}
        '{{this}}',
    {{/each}}
]

# Metrics server to which the node will be reporting their metrics data.
metrics_server_url = '{{ mixnode.metrics_server_url }}'

# Address of the validator contract managing the network.
mixnet_contract_address = '{{ mixnode.mixnet_contract_address }}'

##### advanced configuration options #####

# Absolute path to the home Nym Clients directory.
nym_root_directory = '{{ mixnode.nym_root_directory }}'


##### logging configuration options #####

[logging]

# TODO

"#
}
