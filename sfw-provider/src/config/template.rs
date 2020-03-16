pub(crate) fn config_template() -> &'static str {
    // While using normal toml marshalling would have been way simpler with less overhead,
    // I think it's useful to have comments attached to the saved config file to explain behaviour of
    // particular fields.
    // Note: any changes to the template must be reflected in the appropriate structs in mod.rs.
    r#"
# This is a TOML config file.
# For more information, see https://github.com/toml-lang/toml

##### main base mixnode config options #####

[provider]
# Human readable ID of this particular service provider.
id = '{{ provider.id }}'

# Completely optional value specifying geographical location of this particular node.
# Currently it's used entirely for debug purposes, as there are no mechanisms implemented
# to verify correctness of the information provided. However, feel free to fill in
# this field with as much accuracy as you wish to share.
location = '{{ provider.location }}'

# Path to file containing private sphinx key.
private_sphinx_key_file = '{{ provider.private_sphinx_key_file }}'

# Path to file containing public sphinx key.
public_sphinx_key_file = '{{ provider.public_sphinx_key_file }}'

# nym_home_directory specifies absolute path to the home nym service providers directory.
# It is expected to use default value and hence .toml file should not redefine this field.
nym_root_directory = '{{ provider.nym_root_directory }}'
    

##### Mixnet endpoint config options #####

[mixnet_endpoint]
# Socket address to which this service provider will bind to
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
# Socket address to which this service provider will bind to
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

# [TODO: implement its storage] Full path to a file containing mapping of
# client addresses to their access tokens.
ledger_path = '{{ clients_endpoint.ledger_path }}'


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
presence_sending_delay = {{ debug.presence_sending_delay }}

# Length of filenames for new client messages.
stored_messages_filename_length = {{ debug.stored_messages_filename_length }}

# number of messages client gets on each request
# if there are no real messages, dummy ones are create to always return  
# `message_retrieval_limit` total messages
message_retrieval_limit = {{ debug.message_retrieval_limit }}

"#
}
