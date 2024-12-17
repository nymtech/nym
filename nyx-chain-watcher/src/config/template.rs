// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

// While using normal toml marshalling would have been way simpler with less overhead,
// I think it's useful to have comments attached to the saved config file to explain behaviour of
// particular fields.
// Note: any changes to the template must be reflected in the appropriate structs.
pub(crate) const CONFIG_TEMPLATE: &str = r#"
# This is a TOML config file.
# For more information, see https://github.com/toml-lang/toml

[payment_watcher_config]
{{#each payment_watcher_config.watchers }}
[[watchers]]
id={{this.id}}
description='{{this.description}}'
webhook_url='{{this.webhook_url}}'
{{/each}}




##### logging configuration options #####

[logging]

# TODO

"#;
