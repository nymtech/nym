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

# Current modes of this nym-node.

[modes]
# Specifies whether this node can operate in a mixnode mode.
mixnode = {{ modes.mixnode }}

# Specifies whether this node can operate in an entry mode.
entry = {{ modes.entry }}

# Specifies whether this node can operate in an exit mode.
exit = {{ modes.exit }}

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
# default: `[::]:1789`
bind_address = '{{ mixnet.bind_address }}'

# If applicable, custom port announced in the self-described API that other clients and nodes
# will use.
# Useful when the node is behind a proxy.
# (default: 0 - disabled)
announce_port ={{#if mixnet.announce_port }} {{ mixnet.announce_port }} {{else}} 0 {{/if}}

# Addresses to nym APIs from which the node gets the view of the network.
nym_api_urls = [
{{#each mixnet.nym_api_urls }}'{{this}}',{{/each}}
]

# Addresses to nyxd which the node uses to interact with the nyx chain.
nyxd_urls = [
    {{#each mixnet.nyxd_urls }}'{{this}}',{{/each}}
]

[mixnet.replay_protection]

[mixnet.replay_protection.storage_paths]
# Path to the directory storing currently used bloomfilter(s).
current_bloomfilters_directory = '{{ mixnet.replay_protection.storage_paths.current_bloomfilters_directory }}'

# Storage paths to persistent nym-node data, such as its long term keys.
[storage_paths]

# Path to a file containing basic node description: human-readable name, website, details, etc.
description = '{{ storage_paths.description }}' 

[storage_paths.keys]
# Path to file containing ed25519 identity private key.
private_ed25519_identity_key_file = '{{ storage_paths.keys.private_ed25519_identity_key_file }}'

# Path to file containing ed25519 identity public key.
public_ed25519_identity_key_file = '{{ storage_paths.keys.public_ed25519_identity_key_file }}'

# Path to file containing the primary x25519 sphinx private key.
primary_x25519_sphinx_key_file = '{{ storage_paths.keys.primary_x25519_sphinx_key_file }}'

# Path to file containing the secondary x25519 sphinx private key.
secondary_x25519_sphinx_key_file = '{{ storage_paths.keys.secondary_x25519_sphinx_key_file }}'

# Path to file containing x25519 noise private key.
private_x25519_noise_key_file = '{{ storage_paths.keys.private_x25519_noise_key_file }}'

# Path to file containing x25519 noise public key.
public_x25519_noise_key_file = '{{ storage_paths.keys.public_x25519_noise_key_file }}'


##### http-API nym-node config options #####

[http]
# Socket address this node will use for binding its http API.
# default: `[::]:8080`
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
# default: `[::]:51822`
bind_address = '{{ wireguard.bind_address }}'

# Private IP address of the wireguard gateway.
# default: `10.1.0.1`
private_ipv4 = '{{ wireguard.private_ipv4 }}'

# Private IP address of the wireguard gateway.
# default: `fc01::1`
private_ipv6 = '{{ wireguard.private_ipv6 }}'

# Port announced to external clients wishing to connect to the wireguard interface.
# Useful in the instances where the node is behind a proxy.
announced_port = {{ wireguard.announced_port }}

# The prefix denoting the maximum number of the clients that can be connected via Wireguard using IPv4.
# The maximum value for IPv4 is 32
private_network_prefix_v4 = {{ wireguard.private_network_prefix_v4 }}

# The prefix denoting the maximum number of the clients that can be connected via Wireguard using IPv6.
# The maximum value for IPv6 is 128
private_network_prefix_v6 = {{ wireguard.private_network_prefix_v6 }}

[wireguard.storage_paths]
# Path to file containing wireguard x25519 diffie hellman private key.
private_diffie_hellman_key_file = '{{ wireguard.storage_paths.private_diffie_hellman_key_file }}'

# Path to file containing wireguard x25519 diffie hellman public key.
public_diffie_hellman_key_file = '{{ wireguard.storage_paths.public_diffie_hellman_key_file }}'


##### verloc config options #####

[verloc]
# Socket address this node will use for binding its verloc API.
# default: `[::]:1790`
bind_address = '{{ verloc.bind_address }}'

# If applicable, custom port announced in the self-described API that other clients and nodes
# will use.
# Useful when the node is behind a proxy.
# (default: 0 - disabled)
announce_port ={{#if verloc.announce_port }} {{ verloc.announce_port }} {{else}} 0 {{/if}}


##### gateway tasks config options #####

[gateway_tasks]
# Indicates whether this gateway is accepting only coconut credentials for accessing the mixnet
# or if it also accepts non-paying clients
enforce_zk_nyms = {{ gateway_tasks.enforce_zk_nyms }}

# Socket address this node will use for binding its client websocket API.
# default: `[::]:9000`
ws_bind_address = '{{ gateway_tasks.ws_bind_address }}'

# Custom announced port for listening for websocket client traffic.
# If unspecified, the value from the `bind_address` will be used instead
# (default: 0 - unspecified)
announce_ws_port = {{#if gateway_tasks.announce_ws_port }} {{ gateway_tasks.announce_ws_port }} {{else}} 0 {{/if}}

# If applicable, announced port for listening for secure websocket client traffic.
# (default: 0 - disabled)
announce_wss_port = {{#if gateway_tasks.announce_wss_port }} {{ gateway_tasks.announce_wss_port }} {{else}} 0 {{/if}}


[gateway_tasks.storage_paths]
# Path to sqlite database containing all persistent data: messages for offline clients,
# derived shared keys, available client bandwidths and wireguard peers.
clients_storage = '{{ gateway_tasks.storage_paths.clients_storage }}'

# Path to sqlite database containing all persistent stats data.
stats_storage = '{{ gateway_tasks.storage_paths.stats_storage }}'

# Path to file containing cosmos account mnemonic used for zk-nym redemption.
cosmos_mnemonic = '{{ gateway_tasks.storage_paths.cosmos_mnemonic }}'

##### service providers nym-node config options #####

[service_providers]

# specifies whether this exit node should run in 'open-proxy' mode
# and thus would attempt to resolve **ANY** request it receives.
open_proxy = {{ service_providers.open_proxy }}

# Specifies the custom url for an upstream source of the exit policy used by this node.
upstream_exit_policy_url = '{{ service_providers.upstream_exit_policy_url }}'

[service_providers.network_requester]
# currently empty (there are some debug options one might want to configure)

[service_providers.ip_packet_router]
# currently empty (there are some debug options one might want to configure)

[service_providers.authenticator]
# currently empty (there are some debug options one might want to configure)

[service_providers.storage_paths]

# Path to sqlite database containing all persistent data: messages for offline clients,
# derived shared keys, available client bandwidths and wireguard peers.
clients_storage = '{{ service_providers.storage_paths.clients_storage }}'

# Path to sqlite database containing all persistent stats data.
stats_storage = '{{ service_providers.storage_paths.stats_storage }}'

[service_providers.storage_paths.network_requester]
# Path to file containing network requester ed25519 identity private key.
private_ed25519_identity_key_file = '{{ service_providers.storage_paths.network_requester.private_ed25519_identity_key_file }}'

# Path to file containing network requester ed25519 identity public key.
public_ed25519_identity_key_file = '{{ service_providers.storage_paths.network_requester.public_ed25519_identity_key_file }}'

# Path to file containing network requester x25519 diffie hellman private key.
private_x25519_diffie_hellman_key_file = '{{ service_providers.storage_paths.network_requester.private_x25519_diffie_hellman_key_file }}'

# Path to file containing network requester x25519 diffie hellman public key.
public_x25519_diffie_hellman_key_file = '{{ service_providers.storage_paths.network_requester.public_x25519_diffie_hellman_key_file }}'

# Path to file containing key used for encrypting and decrypting the content of an
# acknowledgement so that nobody besides the client knows which packet it refers to.
ack_key_file = '{{ service_providers.storage_paths.network_requester.ack_key_file }}'

# Path to the persistent store for received reply surbs, unused encryption keys and used sender tags.
reply_surb_database = '{{ service_providers.storage_paths.network_requester.reply_surb_database }}'

# Normally this is a path to the file containing information about gateways used by this client,
# i.e. details such as their public keys, owner addresses or the network information.
# but in this case it just has the basic information of "we're using custom gateway".
# Due to how clients are started up, this file has to exist.
gateway_registrations = '{{ service_providers.storage_paths.network_requester.gateway_registrations }}'

[service_providers.storage_paths.ip_packet_router]
# Path to file containing ip packet router ed25519 identity private key.
private_ed25519_identity_key_file = '{{ service_providers.storage_paths.ip_packet_router.private_ed25519_identity_key_file }}'

# Path to file containing ip packet router ed25519 identity public key.
public_ed25519_identity_key_file = '{{ service_providers.storage_paths.ip_packet_router.public_ed25519_identity_key_file }}'

# Path to file containing ip packet router x25519 diffie hellman private key.
private_x25519_diffie_hellman_key_file = '{{ service_providers.storage_paths.ip_packet_router.private_x25519_diffie_hellman_key_file }}'

# Path to file containing ip packet router x25519 diffie hellman public key.
public_x25519_diffie_hellman_key_file = '{{ service_providers.storage_paths.ip_packet_router.public_x25519_diffie_hellman_key_file }}'

# Path to file containing key used for encrypting and decrypting the content of an
# acknowledgement so that nobody besides the client knows which packet it refers to.
ack_key_file = '{{ service_providers.storage_paths.ip_packet_router.ack_key_file }}'

# Path to the persistent store for received reply surbs, unused encryption keys and used sender tags.
reply_surb_database = '{{ service_providers.storage_paths.ip_packet_router.reply_surb_database }}'

# Normally this is a path to the file containing information about gateways used by this client,
# i.e. details such as their public keys, owner addresses or the network information.
# but in this case it just has the basic information of "we're using custom gateway".
# Due to how clients are started up, this file has to exist.
gateway_registrations = '{{ service_providers.storage_paths.ip_packet_router.gateway_registrations }}'

[service_providers.storage_paths.authenticator]
# Path to file containing authenticator ed25519 identity private key.
private_ed25519_identity_key_file = '{{ service_providers.storage_paths.authenticator.private_ed25519_identity_key_file }}'

# Path to file containing authenticator ed25519 identity public key.
public_ed25519_identity_key_file = '{{ service_providers.storage_paths.authenticator.public_ed25519_identity_key_file }}'

# Path to file containing authenticator x25519 diffie hellman private key.
private_x25519_diffie_hellman_key_file = '{{ service_providers.storage_paths.authenticator.private_x25519_diffie_hellman_key_file }}'

# Path to file containing authenticator x25519 diffie hellman public key.
public_x25519_diffie_hellman_key_file = '{{ service_providers.storage_paths.authenticator.public_x25519_diffie_hellman_key_file }}'

# Path to file containing key used for encrypting and decrypting the content of an
# acknowledgement so that nobody besides the client knows which packet it refers to.
ack_key_file = '{{ service_providers.storage_paths.authenticator.ack_key_file }}'

# Path to the persistent store for received reply surbs, unused encryption keys and used sender tags.
reply_surb_database = '{{ service_providers.storage_paths.authenticator.reply_surb_database }}'

# Normally this is a path to the file containing information about gateways used by this client,
# i.e. details such as their public keys, owner addresses or the network information.
# but in this case it just has the basic information of "we're using custom gateway".
# Due to how clients are started up, this file has to exist.
gateway_registrations = '{{ service_providers.storage_paths.authenticator.gateway_registrations }}'


"#;
