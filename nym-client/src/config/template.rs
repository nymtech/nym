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
id = "{{ client.id }}"

# URL to the directory server.
directory_server = "{{ client.directory_server }}"

# Path to file containing private identity key.
private_identity_key_file = "{{ client.private_identity_key_file }}"

# Path to file containing public identity key.
public_identity_key_file = "{{ client.public_identity_key_file }}"

##### additional client config options #####

# ID of the provider from which the client should be fetching messages.
provider_id = "{{ client.provider_id }}"

##### advanced configuration options #####

# Absolute path to the home Nym Clients directory.
nym_root_directory = "{{ client.nym_root_directory }}"


##### socket config options #####

[socket]

# allowed values are 'TCP', 'WebSocket' or 'None'
socket_type = "{{ socket.socket_type }}"

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

# The parameter of Poisson distribution determining how long, on average,
# sent packet is going to be delayed at any given mix node.
# So for a packet going through three mix nodes, on average, it will take three times this value
# until the packet reaches its destination.
# The provided value is interpreted as seconds.
average_packet_delay = {{ debug.average_packet_delay }}

# The parameter of Poisson distribution determining how long, on average,
# it is going to take for another loop cover traffic message to be sent.
# If set to a negative value, the loop cover traffic stream will be disabled.
# The provided value is interpreted as seconds.
loop_cover_traffic_average_delay = {{ debug.loop_cover_traffic_average_delay }}

# The uniform delay every which clients are querying the providers for received packets.
# If set to a negative value, client will never try to fetch their messages.
# The provided value is interpreted as seconds.
fetch_message_delay = {{ debug.fetch_message_delay }}

# The parameter of Poisson distribution determining how long, on average,
# it is going to take another 'real traffic stream' message to be sent.
# If no real packets are available and cover traffic is enabled,
# a loop cover message is sent instead in order to preserve the rate.
# If set to a negative value, client will never try to send real traffic data.
# The provided value is interpreted as seconds.
message_sending_average_delay = {{ debug.message_sending_average_delay }}

# Whether loop cover messages should be sent to respect message_sending_rate.
# In the case of it being disabled and not having enough real traffic
# waiting to be sent the actual sending rate is going be lower than the desired value
# thus decreasing the anonymity.
rate_compliant_cover_messages_disabled = {{ debug.rate_compliant_cover_messages_disabled }}

# The uniform delay every which clients are querying the directory server
# to try to obtain a compatible network topology to send sphinx packets through.
# If set to a negative value, client will never try to refresh its topology,
# meaning it will always try to use whatever it obtained on startup.
# The provided value is interpreted as seconds.
topology_refresh_rate = {{ debug.topology_refresh_rate }}

    "#
}
