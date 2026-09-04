// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

pub(crate) const CONFIG_TEMPLATE: &str = r#"
# This is a TOML config file.
# For more information, see https://github.com/toml-lang/toml

##### main base nym-api config options #####

[base]

# ID specifies the human readable ID of this particular nym-api.
id = '{{ base.id }}'

# Validator server to which the API will be getting information about the network.
local_validator = '{{ base.local_validator }}'

# Socket address this api will use for binding its http API.
bind_address = '{{ base.bind_address }}'

# Bearer token for exposing and accessing additional utility routes
utility_routes_bearer = '{{ base.utility_routes_bearer }}'

# Mnemonic used for rewarding and validator interaction
mnemonic = '{{ base.mnemonic }}'

[base.storage_paths]

# Path to directory containing persistent caches of, for example,
# the describe information, performance, etc.
# It is used for restarting the nym-api and preserving the data
persistent_cache_directory = '{{ base.storage_paths.persistent_cache_directory }}'

# Path to file containing private identity key of the nym-api.
private_identity_key_file = '{{ base.storage_paths.private_identity_key_file }}'

# Path to file containing public identity key of the nym-api.
public_identity_key_file = '{{ base.storage_paths.public_identity_key_file }}'

##### network monitor config options #####

[network_monitor]
# Specifies whether network monitoring service is enabled in this process.
enabled = {{ network_monitor.enabled }}

[network_monitor.storage_paths]

# Path to the database containing bandwidth credentials of this client.
credentials_database_path = '{{ network_monitor.storage_paths.credentials_database_path }}'

[network_monitor.debug]

# Indicates whether this validator api is running in a disabled credentials mode, thus attempting
# to claim bandwidth without presenting bandwidth credentials.
disabled_credentials_mode = {{ network_monitor.debug.disabled_credentials_mode }}

# Specifies the interval at which the network monitor sends the test packets.
run_interval = '{{ network_monitor.debug.run_interval }}'

# Desired number of test routes to be constructed (and working) during a monitor test run.
test_routes = {{ network_monitor.debug.test_routes }}

# The minimum number of test routes that need to be constructed (and working) in order for
# a monitor test run to be valid.
minimum_test_routes = {{ network_monitor.debug.minimum_test_routes }}

# Number of test packets sent via each pseudorandom route to verify whether they work correctly,
# before using them for testing the rest of the network.
route_test_packets = {{ network_monitor.debug.route_test_packets }}

# Number of test packets sent to each node during regular monitor test run.
per_node_test_packets = {{ network_monitor.debug.per_node_test_packets }}


##### node status api config options #####

[node_status_api.storage_paths]

# Path to the database file containing uptime statuses for all mixnodes and gateways.
database_path = '{{ node_status_api.storage_paths.database_path }}'


##### performance provider config options #####
[performance_provider]

# Specifies whether this nym-api should attempt to retrieve node performance
# information from the performance contract.
use_performance_contract_data = {{ performance_provider.use_performance_contract_data }}

# Which properties contribute to a node's score, and in what proportion.
#
# The properties all measure the SAME thing - whether a node carries traffic - from different
# sources, so their weights are proportions of one figure rather than independent axes. That figure
# is then multiplied by the node's config score, so configuration gates every property equally.
#
# Rules, all enforced at startup:
#   - the ENABLED weights must sum to 1.0, so enabling one means restating another's share
#   - at least one property must be enabled with a non-zero weight
#   - either legacy_v1_routing or liveness must be enabled: stress testing applies to mixnodes
#     alone, so on its own it would leave every gateway unscoreable
#   - none may be enabled while use_performance_contract_data is set, which serves none of them
#
# A property that does not APPLY to a given node, or that its availability threshold drops while an
# orchestrator is down, is renormalised away rather than deflating that node's score. A gateway is
# never stress-tested, so it is scored on what it does have. That does mean the effective weight
# differs by role: a declared weight is a share of whatever applies.

# The network monitor v1 routing score. Turning this off is how v1 eventually gets retired, at
# which point liveness must be carrying the measurement in its place.
[performance_provider.scoring.legacy_v1_routing]
enabled = {{ performance_provider.scoring.legacy_v1_routing.enabled }}
weight = {{ performance_provider.scoring.legacy_v1_routing.weight }}

# Mixnode stress testing, from the v3 network monitor. Never applies to gateways.
[performance_provider.scoring.stress_testing]
enabled = {{ performance_provider.scoring.stress_testing.enabled }}
weight = {{ performance_provider.scoring.stress_testing.weight }}

# Minimal-hop liveness testing, from the v3 network monitor. Covers mixnodes and gateways.
#
# The liveness score is served on each node's annotation whether or not this is enabled; this only
# controls whether it carries weight. Enabling takes effect immediately at the weight below, so
# consult the divergence surface (/v3/unstable/nym-nodes/liveness-divergence) BEFORE switching it
# on: nodes that have not yet ingested their agents' authorisations, and gateways not yet carrying
# the monitor-session behaviour, score zero for reasons unrelated to their forwarding.
[performance_provider.scoring.liveness]
enabled = {{ performance_provider.scoring.liveness.enabled }}
weight = {{ performance_provider.scoring.liveness.weight }}

[performance_provider.debug]
# If stress testing is enabled, this specifies the minimum % of nodes,
# that must have their stress data available in the `stress_testing_data_period`,
# in order to include that metric in performance calculation.
# This is done to protect against Network Monitor failures and not receiving any data.
minimum_available_stress_testing_results = {{ performance_provider.debug.minimum_available_stress_testing_results }}

# Specifies the duration of the rolling average used for stress testing score.
stress_testing_data_period = '{{ performance_provider.debug.stress_testing_data_period }}'

# Specifies the duration of the rolling average used for the liveness score.
liveness_data_period = '{{ performance_provider.debug.liveness_data_period }}'

# If liveness is enabled, this specifies the minimum % of liveness-eligible nodes
# that must have their liveness data available in the `liveness_data_period`,
# in order to include that metric in performance calculation.
minimum_available_liveness_results = {{ performance_provider.debug.minimum_available_liveness_results }}

##### rewarding config options #####

[rewarding]

# Specifies whether rewarding service is enabled in this process.
enabled = {{ rewarding.enabled }}

[rewarding.debug]

# Specifies the minimum percentage of monitor test run data present in order to
# distribute rewards for given interval.
# Note, only values in range 0-100 are valid
minimum_interval_monitor_threshold = {{ rewarding.debug.minimum_interval_monitor_threshold }}


[ecash_signer]

# Specifies whether ecash signing protocol is enabled in this process.
enabled = {{ ecash_signer.enabled }}

# address of this nym-api as announced to other instances for the purposes of performing the DKG.
announce_address = '{{ ecash_signer.announce_address }}'

[ecash_signer.storage_paths]

# Path to a JSON file where state is persisted between different stages of DKG.
dkg_persistent_state_path = '{{ ecash_signer.storage_paths.dkg_persistent_state_path }}'

# Path to the ecash key.
ecash_key_path = '{{ ecash_signer.storage_paths.ecash_key_path }}'

# Path to the dkg dealer decryption key
decryption_key_path = '{{ ecash_signer.storage_paths.decryption_key_path }}'

# Path to the dkg dealer public key with proof
public_key_with_proof_path = '{{ ecash_signer.storage_paths.public_key_with_proof_path }}'

"#;
