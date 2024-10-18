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

# Duration of block signing epoch.
block_signing_epoch_duration = '{{ rewarding.epoch_duration }}'

[rewarding.ratios]
# The percent of the epoch reward being awarded for block signing.
block_signing = {{ rewarding.ratios.block_signing }}

# The percent of the epoch reward being awarded for credential issuance.
credential_issuance = {{ rewarding.ratios.credential_issuance }}

# The percent of the epoch reward being awarded for credential verification.
credential_verification = {{ rewarding.ratios.credential_verification }}
    
    
[block_signing]
# Specifies whether rewarding for block signing is enabled.
enabled = {{ block_signing.enabled }}

# Specifies whether to only monitor and not send rewards.
monitor_only = {{ block_signing.monitor_only }}

# List of validators that will receive rewards for block signing.
# If not on the list, the validator will be treated as if it had 0 voting power.
whitelist = [
    # needs to be manually populated; expects nvalcons1... addresses.
    # you can get them from, for example, `/cosmos/base/tendermint/v1beta1/validatorsets/latest` endpoint
]
 
    
[issuance_monitor]
# Specifies whether credential issuance monitoring (and associated rewards) are enabled.
enabled = {{ issuance_monitor.enabled }}

run_interval = '{{ issuance_monitor.run_interval }}'

# Defines the minimum number of credentials the monitor will validate
# regardless of the sampling rate
min_validate_per_issuer = {{ issuance_monitor.min_validate_per_issuer }}

# The sampling rate of the issued credentials
sampling_rate = {{ issuance_monitor.sampling_rate }}

# List of validators that will receive rewards for credential issuance.
# If not on the list, the validator will be treated as if it hadn't issued a single credential.
whitelist = [
    # needs to be manually populated; expects n1... addresses
]
    
[nyxd_scraper]
# Url to the websocket endpoint of a validator, for example `wss://rpc.nymtech.net/websocket`
websocket_url = '{{ nyxd_scraper.websocket_url }}'

# default: the last 362880 states are kept, pruning at 10 block intervals
# nothing: all historic states will be saved, nothing will be deleted (i.e. archiving)
# everything: 2 latest states will be kept; pruning at 10 block intervals.
# custom: allow pruning options to be manually specified through 'pruning.keep_recent' and 'pruning.interval'
pruning.strategy = '{{ nyxd_scraper.pruning.strategy }}'

# These are applied if and only if the pruning strategy is custom.
pruning.keep_recent = {{ nyxd_scraper.pruning.keep_recent }}
pruning.interval = {{ nyxd_scraper.pruning.interval }}
"#;
