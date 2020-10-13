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

##### main base mixnode config options #####

[mixnode]
# Version of the mixnode for which this configuration was created.
version = '{{ mixnode.version }}'
    
# Human readable ID of this particular mixnode.
id = '{{ mixnode.id }}'

# Completely optional value specifying geographical location of this particular node.
# Currently it's used entirely for debug purposes, as there are no mechanisms implemented
# to verify correctness of the information provided. However, feel free to fill in
# this field with as much accuracy as you wish to share.
location = '{{ mixnode.location }}'
    
# Layer of this particular mixnode determining its position in the network.
layer = {{ mixnode.layer }}

# Socket address to which this mixnode will bind to and will be listening for packets.
listening_address = '{{ mixnode.listening_address }}'

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

# Directory server to which the server will be reporting their presence data.
presence_directory_server = '{{ mixnode.presence_directory_server }}'

# Directory server to which the server will be reporting their metrics data.
metrics_directory_server = '{{ mixnode.metrics_directory_server }}'

##### advanced configuration options #####

# Absolute path to the home Nym Clients directory.
nym_root_directory = '{{ mixnode.nym_root_directory }}'


##### logging configuration options #####

[logging]

# TODO

"#
}
