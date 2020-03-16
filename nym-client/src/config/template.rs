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

# The parameter of Poisson distribution determining how long, on average,
# sent packet is going to be delayed at any given mix node.
# So for a packet going through three mix nodes, on average, it will take three times this value
# until the packet reaches its destination.
# The provided value is interpreted as milliseconds.
average_packet_delay = {{ debug.average_packet_delay }}

# The parameter of Poisson distribution determining how long, on average,
# it is going to take for another loop cover traffic message to be sent.
# The provided value is interpreted as milliseconds.
loop_cover_traffic_average_delay = {{ debug.loop_cover_traffic_average_delay }}

# The uniform delay every which clients are querying the providers for received packets.
# The provided value is interpreted as milliseconds.
fetch_message_delay = {{ debug.fetch_message_delay }}

# The parameter of Poisson distribution determining how long, on average,
# it is going to take another 'real traffic stream' message to be sent.
# If no real packets are available and cover traffic is enabled,
# a loop cover message is sent instead in order to preserve the rate.
# The provided value is interpreted as milliseconds.
message_sending_average_delay = {{ debug.message_sending_average_delay }}

# Whether loop cover messages should be sent to respect message_sending_rate.
# In the case of it being disabled and not having enough real traffic
# waiting to be sent the actual sending rate is going be lower than the desired value
# thus decreasing the anonymity.
rate_compliant_cover_messages_disabled = {{ debug.rate_compliant_cover_messages_disabled }}

# The uniform delay every which clients are querying the directory server
# to try to obtain a compatible network topology to send sphinx packets through.
# The provided value is interpreted as milliseconds.
topology_refresh_rate = {{ debug.topology_refresh_rate }}

# During topology refresh, test packets are sent through every single possible network
# path. This timeout determines waiting period until it is decided that the packet
# did not reach its destination.
# The provided value is interpreted as milliseconds.
topology_resolution_timeout = {{ debug.topology_resolution_timeout }}
  
# How many packets should be sent through each path during the healthcheck
number_of_healthcheck_test_packets = {{ debug.number_of_healthcheck_test_packets }}

# In the current healthcheck implementation, threshold indicating percentage of packets 
# node received during healthcheck. Node's score must be above that value to be 
# considered healthy.
node_score_threshold = {{ debug.node_score_threshold }}

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
