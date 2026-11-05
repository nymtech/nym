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

# Mnemonic used for the ecash signer's DKG and multisig operations
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

##### node status api config options #####

[node_status_api.storage_paths]

# Path to the database file containing uptime statuses for all mixnodes and gateways.
database_path = '{{ node_status_api.storage_paths.database_path }}'

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
