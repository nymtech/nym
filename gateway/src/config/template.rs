// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

// While using normal toml marshalling would have been way simpler with less overhead,
// I think it's useful to have comments attached to the saved config file to explain behaviour of
// particular fields.
// Note: any changes to the template must be reflected in the appropriate structs.
pub(crate) const CONFIG_TEMPLATE: &str = r#"
# This is a TOML config file.
# For more information, see https://github.com/toml-lang/toml

##### main base gateway config options #####

[host]
# Ip address(es) of this host, such as 1.1.1.1 that external clients will use for connections.
public_ips = [
    {{#each host.public_ips }}
        '{{this}}',
    {{/each}}
]

# (temporary) Optional hostname of this node, for example nymtech.net.
hostname = '{{ host.hostname }}'

[gateway]
# Version of the gateway for which this configuration was created.
version = '{{ gateway.version }}'

# Human readable ID of this particular gateway.
id = '{{ gateway.id }}'

# Indicates whether this gateway is accepting only coconut credentials for accessing the
# the mixnet, or if it also accepts non-paying clients
only_coconut_credentials = {{ gateway.only_coconut_credentials }}

# Socket address to which this gateway will bind to and will be listening for packets.
listening_address = '{{ gateway.listening_address }}'

# Port used for listening for all mixnet traffic.
# (default: 1789)
mix_port = {{ gateway.mix_port }}

# Port used for listening for all client websocket traffic.
# (default: 9000)
clients_port = {{ gateway.clients_port }}

# If applicable, announced port for listening for secure websocket client traffic.
# (default: 0 - disabled)
clients_wss_port ={{#if gateway.clients_wss_port }} {{ gateway.clients_wss_port }} {{else}} 0 {{/if}}
    
# Wheather gateway collects and sends anonymized statistics
enabled_statistics = {{ gateway.enabled_statistics }}

# Domain address of the statistics service
statistics_service_url = '{{ gateway.statistics_service_url }}'

# Addresses to APIs running on validator from which the node gets the view of the network.
nym_api_urls = [
    {{#each gateway.nym_api_urls }}
        '{{this}}',
    {{/each}}
]

# Addresses to validators which the node uses to check for double spending of nym tokens.
nyxd_urls = [
    {{#each gateway.nyxd_urls }}
        '{{this}}',
    {{/each}}
]

cosmos_mnemonic = '{{ gateway.cosmos_mnemonic }}'

[http]
# Socket address this node will use for binding its http API.
# default: `0.0.0.0:8080`
bind_address = '{{ http.bind_address }}'

# Path to assets directory of custom landing page of this node
landing_page_assets_path = '{{ http.landing_page_assets_path }}'

[network_requester]
# Specifies whether network requester service is enabled in this process.
enabled = {{ network_requester.enabled }}

[ip_packet_router]
# Specifies whether ip packet router service is enabled in this process.
enabled = {{ ip_packet_router.enabled }}

[storage_paths] 

# Path to file containing private identity key.
keys.private_identity_key_file = '{{ storage_paths.keys.private_identity_key_file }}'

# Path to file containing public identity key.
keys.public_identity_key_file = '{{ storage_paths.keys.public_identity_key_file }}'

# Path to file containing private identity key.
keys.private_sphinx_key_file = '{{ storage_paths.keys.private_sphinx_key_file }}'

# Path to file containing public sphinx key.
keys.public_sphinx_key_file = '{{ storage_paths.keys.public_sphinx_key_file }}'

# Path to sqlite database containing all persistent data: messages for offline clients,
# derived shared keys and available client bandwidths.
clients_storage = '{{ storage_paths.clients_storage }}'

# Path to the configuration of the embedded network requester.
network_requester_config = '{{ storage_paths.network_requester_config }}'

# Path to the configuration of the embedded ip packet router.
ip_packet_router_config = '{{ storage_paths.ip_packet_router_config }}'

##### logging configuration options #####

[logging]

# TODO

"#;
