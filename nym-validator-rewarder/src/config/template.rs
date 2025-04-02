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

private_ed25519_identity_key_file = '{{ storage_paths.private_ed25519_identity_key_file }}'
public_ed25519_identity_key_file = '{{ storage_paths.public_ed25519_identity_key_file }}'

[rewarding]
# Specifies total budget for a 24h period.
daily_budget = '{{ rewarding.daily_budget }}'

[rewarding.ratios]
# The percent of the epoch reward being awarded for block signing.
block_signing = {{ rewarding.ratios.block_signing }}

# The percent of the epoch reward being awarded for ticketbook issuance.
ticketbook_issuance = {{ rewarding.ratios.ticketbook_issuance }}

# The percent of the epoch reward being awarded for ticketbook verification.
ticketbook_verification = {{ rewarding.ratios.ticketbook_verification }}
    
    
[block_signing]
# Specifies whether rewarding for block signing is enabled.
enabled = {{ block_signing.enabled }}

# Duration of block signing epoch.
epoch_duration = '{{ block_signing.epoch_duration }}'

# Specifies whether to only monitor and not send rewards.
monitor_only = {{ block_signing.monitor_only }}

# List of validators that will receive rewards for block signing.
# If not on the list, the validator will be treated as if it had 0 voting power.
whitelist = [
    # needs to be manually populated; expects nvalcons1... addresses.
    # you can get them from, for example, `/cosmos/base/tendermint/v1beta1/validatorsets/latest` endpoint
]
 
    
[ticketbook_issuance]
# Specifies whether rewarding for ticketbook issuance is enabled.
enabled = {{ ticketbook_issuance.enabled }}

# Specifies whether to only monitor and not send rewards.
monitor_only = {{ ticketbook_issuance.monitor_only }}

# Defines the minimum number of credentials the rewarder will validate
# regardless of the sampling rate
min_validate_per_issuer = {{ ticketbook_issuance.min_validate_per_issuer }}

# The sampling rate of the issued ticketbooks
sampling_rate = {{ ticketbook_issuance.sampling_rate }}

# Ratio of issuers that will undergo full verification as opposed to being let through.
full_verification_ratio = {{ ticketbook_issuance.full_verification_ratio }}

# List of validators that will receive rewards for ticketbook issuance.
# If not on the list, the validator will be treated as if it hadn't issued a single ticketbook.
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
