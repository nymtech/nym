// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

pub(crate) fn config_template() -> &'static str {
    // While using normal toml marshalling would have been way simpler with less overhead,
    // I think it's useful to have comments attached to the saved config file to explain behaviour of
    // particular fields.
    // Note: any changes to the template must be reflected in the appropriate structs in mod.rs.
    r#"
# This is a TOML config file.
# For more information, see https://github.com/toml-lang/toml

##### main base client config options #####

[client]
# Version of the client for which this configuration was created.
version = '{{ client.version }}'

# Human readable ID of this particular client.
id = '{{ client.id }}'

# URL to the validator server for obtaining network topology.
validator_rest_urls = [
    {{#each client.validator_rest_urls }}
        '{{this}}',
    {{/each}}
]

# Address of the validator contract managing the network.
mixnet_contract_address = '{{ client.mixnet_contract_address }}'

# Special mode of the system such that all messages are sent as soon as they are received
# and no cover traffic is generated. If set all message delays are set to 0 and overwriting
# 'Debug' values will have no effect.
vpn_mode = {{ client.vpn_mode }}

# Path to file containing private identity key.
private_identity_key_file = '{{ client.private_identity_key_file }}'

# Path to file containing public identity key.
public_identity_key_file = '{{ client.public_identity_key_file }}'

# Path to file containing private encryption key.
private_encryption_key_file = '{{ client.private_encryption_key_file }}'

# Path to file containing public encryption key.
public_encryption_key_file = '{{ client.public_encryption_key_file }}'

# Full path to file containing reply encryption keys of all reply-SURBs we have ever
# sent but not received back.
reply_encryption_key_store_path = '{{ client.reply_encryption_key_store_path }}'

##### additional client config options #####

# ID of the gateway from which the client should be fetching messages.
gateway_id = '{{ client.gateway_id }}'

# Address of the gateway listener to which all client requests should be sent.
gateway_listener = '{{ client.gateway_listener }}'

# A gateway specific, optional, base58 stringified shared key used for
# communication with particular gateway.
gateway_shared_key_file = '{{ client.gateway_shared_key_file }}'

# Path to file containing key used for encrypting and decrypting the content of an
# acknowledgement so that nobody besides the client knows which packet it refers to.
ack_key_file = '{{ client.ack_key_file }}'
    
##### advanced configuration options #####

# Absolute path to the home Nym Clients directory.
nym_root_directory = '{{ client.nym_root_directory }}'


##### socket config options #####

[socks5]

# The mix address of the provider to which all requests are going to be sent.
provider_mix_address = '{{ socks5.provider_mix_address }}'

# The port on which the client will be listening for incoming requests
listening_port = {{ socks5.listening_port }}


##### logging configuration options #####

[logging]

# TODO


##### debug configuration options #####
# The following options should not be modified unless you know EXACTLY what you are doing
# as if set incorrectly, they may impact your anonymity.

[debug]

average_packet_delay = '{{ debug.average_packet_delay }}'
average_ack_delay = '{{ debug.average_ack_delay }}'
loop_cover_traffic_average_delay = '{{ debug.loop_cover_traffic_average_delay }}'
message_sending_average_delay = '{{ debug.message_sending_average_delay }}'

"#
}
