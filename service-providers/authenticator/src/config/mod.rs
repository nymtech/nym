// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(flatten)]
    pub base: BaseClientConfig,

    #[serde(default)]
    pub network_requester: NetworkRequester,

    pub storage_paths: NetworkRequesterPaths,

    #[serde(default)]
    pub network_requester_debug: Debug,

    pub logging: LoggingSettings,
}
