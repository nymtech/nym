// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

// While using normal toml marshalling would have been way simpler with less overhead,
// I think it's useful to have comments attached to the saved config file to explain behaviour of
// particular fields.
// Note: any changes to the template must be reflected in the appropriate structs.
pub(crate) const CONFIG_TEMPLATE: &str = r#"
# This is a TOML config file.
# For more information, see https://github.com/toml-lang/toml

# do note that some of the configuration options are not explicitly exposed in this template,
# but can still be manually adjusted. in particular delays, rates, etc.
# look at nym-node/src/config/mod.rs file for more details.

##### main base nym-node config options #####

# Human-readable ID of this particular node.
id = '{{ id }}'

# Current mode of this nym-node.
# Expect this field to be changed in the future to allow running the node in multiple modes (i.e. mixnode + gateway)
mode = '{{ mode }}'

[host]
# Ip address(es) of this host, such as 1.1.1.1 that external clients will use for connections.
# If no values are provided, when this node gets included in the network, 
# its ip addresses will be populated by whatever value is resolved by associated nym-api.
public_ips = [
{{#each host.public_ips }}'{{this}}',{{/each}}
]

# (temporary) Optional hostname of this node, for example nymtech.net.
hostname = '{{ host.hostname }}'

# Optional ISO 3166 alpha-2 two-letter country code of the node's **physical** location
location = '{{ host.location }}'

[mixnet]
# Address this node will bind to for listening for mixnet packets
# default: `0.0.0.0:1789`
bind_address = '{{ mixnet.bind_address }}'

# Addresses to nym APIs from which the node gets the view of the network.
nym_api_urls = [
{{#each mixnet.nym_api_urls }}'{{this}}',{{/each}}
]

# Addresses to nyxd which the node uses to interact with the nyx chain.
nyxd_urls = [
    {{#each mixnet.nyxd_urls }}'{{this}}',{{/each}}
]

# Storage paths to persistent nym-node data, such as its long term keys.
[storage_paths]

# Path to a file containing basic node description: human-readable name, website, details, etc.
description = '{{ storage_paths.description }}' 

[storage_paths.keys]
# Path to file containing ed25519 identity private key.
private_ed25519_identity_key_file = '{{ storage_paths.keys.private_ed25519_identity_key_file }}'

# Path to file containing ed25519 identity public key.
public_ed25519_identity_key_file = '{{ storage_paths.keys.public_ed25519_identity_key_file }}'

# Path to file containing x25519 sphinx private key.
private_x25519_sphinx_key_file = '{{ storage_paths.keys.private_x25519_sphinx_key_file }}'

# Path to file containing x25519 sphinx public key.
public_x25519_sphinx_key_file = '{{ storage_paths.keys.public_x25519_sphinx_key_file }}'

# Path to file containing x25519 noise private key.
private_x25519_noise_key_file = '{{ storage_paths.keys.private_x25519_noise_key_file }}'

# Path to file containing x25519 noise public key.
public_x25519_noise_key_file = '{{ storage_paths.keys.public_x25519_noise_key_file }}'


##### http-API nym-node config options #####

[http]
# Socket address this node will use for binding its http API.
# default: `0.0.0.0:8080`
bind_address = '{{ http.bind_address }}'

# Path to assets directory of custom landing page of this node
landing_page_assets_path = '{{ http.landing_page_assets_path }}'

# An optional bearer token for accessing certain http endpoints.
# Currently only used for obtaining mixnode's stats.
access_token = '{{ http.access_token }}'

# Specify whether basic system information should be exposed.
# default: true
expose_system_info = {{ http.expose_system_info }}

# Specify whether basic system hardware information should be exposed.
# This option is superseded by `expose_system_info`
# default: true
expose_system_hardware = {{ http.expose_system_hardware }}

# Specify whether detailed system crypto hardware information should be exposed.
# This option is superseded by `expose_system_hardware`
# default: true
expose_crypto_hardware = {{ http.expose_crypto_hardware }}

##### wireguard-API nym-node config options #####

[wireguard]
# Specifies whether the wireguard service is enabled on this node.
enabled = {{ wireguard.enabled }}

# Socket address this node will use for binding its wireguard interface.
# default: `0.0.0.0:51822`
bind_address = '{{ wireguard.bind_address }}'

# Private IP address of the wireguard gateway.
# default: `10.1.0.1`
private_ip = '{{ wireguard.private_ip }}'

# Port announced to external clients wishing to connect to the wireguard interface.
# Useful in the instances where the node is behind a proxy.
announced_port = {{ wireguard.announced_port }}

# The prefix denoting the maximum number of the clients that can be connected via Wireguard.
# The maximum value for IPv4 is 32 and for IPv6 is 128
private_network_prefix = {{ wireguard.private_network_prefix }}

[wireguard.storage_paths]
# Path to file containing wireguard x25519 diffie hellman private key.
private_diffie_hellman_key_file = '{{ wireguard.storage_paths.private_diffie_hellman_key_file }}'

# Path to file containing wireguard x25519 diffie hellman public key.
public_diffie_hellman_key_file = '{{ wireguard.storage_paths.public_diffie_hellman_key_file }}'


##### mixnode mode nym-node config options #####

[mixnode]

[mixnode.verloc]
# Socket address this node will use for binding its verloc API.
# default: `0.0.0.0:1790`
bind_address = '{{ mixnode.verloc.bind_address }}'

[mixnode.storage_paths]
# currently empty

##### entry-gateway mode nym-node config options #####

[entry_gateway]
# Indicates whether this gateway is accepting only coconut credentials for accessing the mixnet
# or if it also accepts non-paying clients
enforce_zk_nyms = {{ entry_gateway.enforce_zk_nyms }}

# Indicates whether this gateway is using offline mode for zk-nyms verification
offline_zk_nyms = {{ entry_gateway.offline_zk_nyms }}

# Socket address this node will use for binding its client websocket API.
# default: `0.0.0.0:9000`
bind_address = '{{ entry_gateway.bind_address }}'

# Custom announced port for listening for websocket client traffic.
# If unspecified, the value from the `bind_address` will be used instead
# (default: 0 - unspecified)
announce_ws_port = {{#if entry_gateway.announce_ws_port }} {{ entry_gateway.announce_ws_port }} {{else}} 0 {{/if}}

# If applicable, announced port for listening for secure websocket client traffic.
# (default: 0 - disabled)
announce_wss_port = {{#if entry_gateway.announce_wss_port }} {{ entry_gateway.announce_wss_port }} {{else}} 0 {{/if}}

[entry_gateway.storage_paths]
# Path to sqlite database containing all persistent data: messages for offline clients,
# derived shared keys and available client bandwidths.
clients_storage = '{{ entry_gateway.storage_paths.clients_storage }}'

# Path to file containing cosmos account mnemonic used for zk-nym redemption.
cosmos_mnemonic = '{{ entry_gateway.storage_paths.cosmos_mnemonic }}'

##### exit-gateway mode nym-node config options #####

[exit_gateway]

# specifies whether this exit node should run in 'open-proxy' mode
# and thus would attempt to resolve **ANY** request it receives.
open_proxy = {{ exit_gateway.open_proxy }}

# Specifies the custom url for an upstream source of the exit policy used by this node.
upstream_exit_policy_url = '{{ exit_gateway.upstream_exit_policy_url }}'

[exit_gateway.network_requester]
# currently empty (there are some debug options one might want to configure)

[exit_gateway.ip_packet_router]
# currently empty (there are some debug options one might want to configure)

[exit_gateway.storage_paths]

[exit_gateway.storage_paths.network_requester]
# Path to file containing network requester ed25519 identity private key.
private_ed25519_identity_key_file = '{{ exit_gateway.storage_paths.network_requester.private_ed25519_identity_key_file }}'

# Path to file containing network requester ed25519 identity public key.
public_ed25519_identity_key_file = '{{ exit_gateway.storage_paths.network_requester.public_ed25519_identity_key_file }}'

# Path to file containing network requester x25519 diffie hellman private key.
private_x25519_diffie_hellman_key_file = '{{ exit_gateway.storage_paths.network_requester.private_x25519_diffie_hellman_key_file }}'

# Path to file containing network requester x25519 diffie hellman public key.
public_x25519_diffie_hellman_key_file = '{{ exit_gateway.storage_paths.network_requester.public_x25519_diffie_hellman_key_file }}'

# Path to file containing key used for encrypting and decrypting the content of an
# acknowledgement so that nobody besides the client knows which packet it refers to.
ack_key_file = '{{ exit_gateway.storage_paths.network_requester.ack_key_file }}'

# Path to the persistent store for received reply surbs, unused encryption keys and used sender tags.
reply_surb_database = '{{ exit_gateway.storage_paths.network_requester.reply_surb_database }}'

# Normally this is a path to the file containing information about gateways used by this client,
# i.e. details such as their public keys, owner addresses or the network information.
# but in this case it just has the basic information of "we're using custom gateway".
# Due to how clients are started up, this file has to exist.
gateway_registrations = '{{ exit_gateway.storage_paths.network_requester.gateway_registrations }}'

[exit_gateway.storage_paths.ip_packet_router]
# Path to file containing ip packet router ed25519 identity private key.
private_ed25519_identity_key_file = '{{ exit_gateway.storage_paths.ip_packet_router.private_ed25519_identity_key_file }}'

# Path to file containing ip packet router ed25519 identity public key.
public_ed25519_identity_key_file = '{{ exit_gateway.storage_paths.ip_packet_router.public_ed25519_identity_key_file }}'

# Path to file containing ip packet router x25519 diffie hellman private key.
private_x25519_diffie_hellman_key_file = '{{ exit_gateway.storage_paths.ip_packet_router.private_x25519_diffie_hellman_key_file }}'

# Path to file containing ip packet router x25519 diffie hellman public key.
public_x25519_diffie_hellman_key_file = '{{ exit_gateway.storage_paths.ip_packet_router.public_x25519_diffie_hellman_key_file }}'

# Path to file containing key used for encrypting and decrypting the content of an
# acknowledgement so that nobody besides the client knows which packet it refers to.
ack_key_file = '{{ exit_gateway.storage_paths.ip_packet_router.ack_key_file }}'

# Path to the persistent store for received reply surbs, unused encryption keys and used sender tags.
reply_surb_database = '{{ exit_gateway.storage_paths.ip_packet_router.reply_surb_database }}'

# Normally this is a path to the file containing information about gateways used by this client,
# i.e. details such as their public keys, owner addresses or the network information.
# but in this case it just has the basic information of "we're using custom gateway".
# Due to how clients are started up, this file has to exist.
gateway_registrations = '{{ exit_gateway.storage_paths.ip_packet_router.gateway_registrations }}'

##### logging configuration options #####

[logging]

# TODO

"#;
