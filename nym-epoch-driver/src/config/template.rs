// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

// While using normal toml marshalling would have been way simpler with less overhead,
// I think it's useful to have comments attached to the saved config file to explain behaviour of
// particular fields.
// Note: any changes to the template must be reflected in the appropriate structs.
// The bearer token is rendered raw (`{{{ }}}`): the default `{{ }}` would HTML-escape its
// characters, e.g. the `=` of base64 padding.
pub(crate) const CONFIG_TEMPLATE: &str = r#"
# This is a TOML config file.
# For more information, see https://github.com/toml-lang/toml

# Url to the nyxd instance the driver queries and sends its transactions through.
nyxd_url = '{{ nyxd_url }}'

# Mnemonic of the mixnet contract's rewarding validator account.
mnemonic = '{{ mnemonic }}'

[metrics]
# Socket address the metrics listener binds to.
bind_address = '{{ metrics.bind_address }}'

# Bearer token every request to the metrics route must carry.
bearer_token = '{{{ metrics.bearer_token }}}'

[debug]
# Number of blocks a cadence boundary must lie below the tip before the network view is
# produced at it.
settle_lag = {{ debug.settle_lag }}
"#;
