// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub(crate) fn config_template() -> &'static str {
  r#"
# This is a TOML config file.
# For more information, see https://github.com/toml-lang/toml

##### main base tauri-wallet config options #####

[base]

# Validator server to which the API will be getting information about the network.
validator_url = '{{ base.validator_url }}'

"#
}
