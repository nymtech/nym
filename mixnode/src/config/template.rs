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

##### advanced configuration options #####

# Absolute path to the home Nym Clients directory.
nym_root_directory = '{{ mixnode.nym_root_directory }}'


##### logging configuration options #####

[logging]

# TODO


##### debug configuration options #####
# The following options should not be modified unless you know EXACTLY what you are doing
# as if set incorrectly, they may impact your anonymity.

[debug]

# Directory server to which the server will be reporting their presence data.
presence_directory_server = '{{ debug.presence_directory_server}}'

# Delay between each subsequent presence data being sent.
# The provided value is interpreted as milliseconds.
presence_sending_delay = {{ debug.presence_sending_delay }}

# Directory server to which the server will be reporting their metrics data.
metrics_directory_server = '{{ debug.metrics_directory_server }}'

# Delay between each subsequent metrics data being sent.
# The provided value is interpreted as milliseconds.
metrics_sending_delay = {{ debug.metrics_sending_delay }}

# Initial value of an exponential backoff to reconnect to dropped TCP connection when
# forwarding sphinx packets.
# The provided value is interpreted as milliseconds.
packet_forwarding_initial_backoff = {{ debug.packet_forwarding_initial_backoff }}

# Maximum value of an exponential backoff to reconnect to dropped TCP connection when
# forwarding sphinx packets.
# The provided value is interpreted as milliseconds.
packet_forwarding_maximum_backoff = {{ debug.packet_forwarding_maximum_backoff }}    
"#
}
