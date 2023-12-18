// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

// While using normal toml marshalling would have been way simpler with less overhead,
// I think it's useful to have comments attached to the saved config file to explain behaviour of
// particular fields.
// Note: any changes to the template must be reflected in the appropriate structs.
pub(crate) const CONFIG_TEMPLATE: &str = r#"
# This is a TOML config file.
# For more information, see https://github.com/toml-lang/toml

# Url to the upstream instance of nyxd to use for any queries and rewarding.
upstream_nyxd = '{{ upstream_nyxd }}'

# Mnemonic to the nyx account distributing the rewards
mnemonic = '{{ mnemonic }}'

[storage_paths]

nyxd_scraper = '{{ storage_paths.nyxd_scraper }}'
reward_history = '{{ storage_paths.reward_history }}'

[rewarding]
# Specifies total budget for the epoch
epoch_budget = '{{ rewarding.epoch_budget }}'

epoch_duration = '{{ rewarding.epoch_duration }}'

[rewarding.ratios]
# The percent of the epoch reward being awarded for block signing.
block_signing = {{ rewarding.ratios.block_signing }}

# The percent of the epoch reward being awarded for credential issuance.
credential_issuance = {{ rewarding.ratios.credential_issuance }}

# The percent of the epoch reward being awarded for credential verification.
credential_verification = {{ rewarding.ratios.credential_verification }}
    
    
[block_signing]
# Specifies whether credential issuance for block signing is enabled.
enabled = {{ block_signing.enabled }}
    
    
[issuance_monitor]
# Specifies whether credential issuance monitoring (and associated rewards) are enabled.
enabled = {{ issuance_monitor.enabled }}

run_interval = '{{ issuance_monitor.run_interval }}'

# Defines the minimum number of credentials the monitor will validate
# regardless of the sampling rate
min_validate_per_issuer = {{ issuance_monitor.min_validate_per_issuer }}

# The sampling rate of the issued credentials
sampling_rate = {{ issuance_monitor.sampling_rate }}
    
[nyxd_scraper]
# Url to the websocket endpoint of a validator, for example `wss://rpc.nymtech.net/websocket`
websocket_url = '{{ nyxd_scraper.websocket_url }}'
"#;
