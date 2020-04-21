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
# Human readable ID of this particular client.
id = '{{ client.id }}'

# URL to the directory server.
directory_server = '{{ client.directory_server }}'

# Path to file containing private identity key.
private_identity_key_file = '{{ client.private_identity_key_file }}'

# Path to file containing public identity key.
public_identity_key_file = '{{ client.public_identity_key_file }}'

##### additional client config options #####

# ID of the provider from which the client should be fetching messages.
provider_id = '{{ client.provider_id }}'

# A provider specific, optional, base58 stringified authentication token used for 
# communication with particular provider.
provider_authtoken = '{{ client.provider_authtoken }}'
    
##### advanced configuration options #####

# Absolute path to the home Nym Clients directory.
nym_root_directory = '{{ client.nym_root_directory }}'


##### socket config options #####

[socket]

# allowed values are 'TCP', 'WebSocket' or 'None'
socket_type = '{{ socket.socket_type }}'

# if applicable (for the case of 'TCP' or 'WebSocket'), the port on which the client
# will be listening for incoming requests
listening_port = {{ socket.listening_port }}


##### logging configuration options #####

[logging]

# TODO


##### debug configuration options #####
# The following options should not be modified unless you know EXACTLY what you are doing
# as if set incorrectly, they may impact your anonymity.

[debug]

average_packet_delay = {{ debug.average_packet_delay }}
loop_cover_traffic_average_delay = {{ debug.loop_cover_traffic_average_delay }}
fetch_message_delay = {{ debug.fetch_message_delay }}
message_sending_average_delay = {{ debug.message_sending_average_delay }}

"#
}
