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

[mixnode]
# Version of the mixnode for which this configuration was created.
version = '{{ mixnode.version }}'
    
# Human readable ID of this particular mixnode.
id = '{{ mixnode.id }}'

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
# later on by using name resolvable with a DNS query, such as `nymtech.net`.
announce_address = '{{ mixnode.announce_address }}'

# Port used for listening for all mixnet traffic.
# (default: 1789)
mix_port = {{ mixnode.mix_port }}

# Port used for listening for verloc traffic.
# (default: 1790)
verloc_port = {{ mixnode.verloc_port }}

# Port used for listening for http requests.
# (default: 8000)
http_api_port = {{ mixnode.http_api_port }}

# Addresses to APIs running on validator from which the node gets the view of the network.
validator_api_urls = [
    {{#each mixnode.validator_api_urls }}
        '{{this}}',
    {{/each}}
]

# Nym wallet address on the blockchain that should control this mixnode
wallet_address = '{{ mixnode.wallet_address }}'

##### advanced configuration options #####

# Absolute path to the home Nym Clients directory.
nym_root_directory = '{{ mixnode.nym_root_directory }}'


##### logging configuration options #####

[logging]

# TODO

"#
}
