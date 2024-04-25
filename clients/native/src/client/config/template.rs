// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// While using normal toml marshalling would have been way simpler with less overhead,
// I think it's useful to have comments attached to the saved config file to explain behaviour of
// particular fields.
// Note: any changes to the template must be reflected in the appropriate structs.
pub(crate) const CONFIG_TEMPLATE: &str = r#"
# This is a TOML config file.
# For more information, see https://github.com/toml-lang/toml

##### main base client config options #####

[client]
# Version of the client for which this configuration was created.
version = '{{ client.version }}'

# Human readable ID of this particular client.
id = '{{ client.id }}'

# Indicates whether this client is running in a disabled credentials mode, thus attempting
# to claim bandwidth without presenting bandwidth credentials.
disabled_credentials_mode = {{ client.disabled_credentials_mode }}

# Addresses to nyxd validators via which the client can communicate with the chain.
nyxd_urls = [
    {{#each client.nyxd_urls }}
        '{{this}}',
    {{/each}}
]

# Addresses to APIs running on validator from which the client gets the view of the network.
nym_api_urls = [
    {{#each client.nym_api_urls }}
        '{{this}}',
    {{/each}}
]

[storage_paths] 

# Path to file containing private identity key.
keys.private_identity_key_file = '{{ storage_paths.keys.private_identity_key_file }}'

# Path to file containing public identity key.
keys.public_identity_key_file = '{{ storage_paths.keys.public_identity_key_file }}'

# Path to file containing private encryption key.
keys.private_encryption_key_file = '{{ storage_paths.keys.private_encryption_key_file }}'

# Path to file containing public encryption key.
keys.public_encryption_key_file = '{{ storage_paths.keys.public_encryption_key_file }}'

# Path to file containing key used for encrypting and decrypting the content of an
# acknowledgement so that nobody besides the client knows which packet it refers to.
keys.ack_key_file = '{{ storage_paths.keys.ack_key_file }}'

# Path to the database containing bandwidth credentials
credentials_database = '{{ storage_paths.credentials_database }}'

# Path to the persistent store for received reply surbs, unused encryption keys and used sender tags.
reply_surb_database = '{{ storage_paths.reply_surb_database }}'

# Path to the file containing information about gateways used by this client,
# i.e. details such as their public keys, owner addresses or the network information.
gateway_registrations = '{{ storage_paths.gateway_registrations }}'

##### socket config options #####

[socket]

# allowed values are 'WebSocket' or 'None'
socket_type = '{{ socket.socket_type }}'

# if applicable (for the case of 'WebSocket'), the port on which the client
# will be listening for incoming requests
listening_port = {{ socket.listening_port }}

# if applicable (for the case of 'WebSocket'), the ip address on which the client
# will be listening for incoming requests
host = '{{ socket.host }}'

##### logging configuration options #####

[logging]

# TODO


##### debug configuration options #####
# The following options should not be modified unless you know EXACTLY what you are doing
# as if set incorrectly, they may impact your anonymity.

[debug]

[debug.traffic]
average_packet_delay = '{{ debug.traffic.average_packet_delay }}'
message_sending_average_delay = '{{ debug.traffic.message_sending_average_delay }}'

[debug.acknowledgements]
average_ack_delay = '{{ debug.acknowledgements.average_ack_delay }}'

[debug.cover_traffic]
loop_cover_traffic_average_delay = '{{ debug.cover_traffic.loop_cover_traffic_average_delay }}'


"#;
