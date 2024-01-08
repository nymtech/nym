// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::old_config_v1_1_20::{
    ConfigV1_1_20, DebugV1_1_20, NetworkRequesterPathsV1_1_20,
};
use nym_client_core::config::disk_persistence::keys_paths::ClientKeysPaths;
use nym_client_core::config::disk_persistence::old_v1_1_20::CommonClientPathsV1_1_20;
use nym_client_core::config::old_config_v1_1_19::ConfigV1_1_19 as BaseConfigV1_1_19;
use nym_client_core::config::old_config_v1_1_20::{
    ClientV1_1_20, ConfigV1_1_20 as BaseClientConfigV1_1_20,
};
use nym_config::legacy_helpers::nym_config::MigrationNymConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

const DEFAULT_STANDARD_LIST_UPDATE_INTERVAL: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV1_1_19 {
    #[serde(flatten)]
    pub base: BaseConfigV1_1_19<ConfigV1_1_19>,

    #[serde(default)]
    pub network_requester: NetworkRequster,

    #[serde(default)]
    pub network_requester_debug: DebugV1_1_19,
}

impl From<ConfigV1_1_19> for ConfigV1_1_20 {
    fn from(value: ConfigV1_1_19) -> Self {
        ConfigV1_1_20 {
            base: BaseClientConfigV1_1_20 {
                client: ClientV1_1_20 {
                    version: value.base.client.version,
                    id: value.base.client.id,
                    disabled_credentials_mode: value.base.client.disabled_credentials_mode,
                    nyxd_urls: value.base.client.nyxd_urls,
                    nym_api_urls: value.base.client.nym_api_urls,
                    gateway_endpoint: value.base.client.gateway_endpoint.into(),
                },
                debug: Default::default(),
            },
            network_requester: Default::default(),
            storage_paths: NetworkRequesterPathsV1_1_20 {
                common_paths: CommonClientPathsV1_1_20 {
                    keys: ClientKeysPaths {
                        private_identity_key_file: value.base.client.private_identity_key_file,
                        public_identity_key_file: value.base.client.public_identity_key_file,
                        private_encryption_key_file: value.base.client.private_encryption_key_file,
                        public_encryption_key_file: value.base.client.public_encryption_key_file,
                        gateway_shared_key_file: value.base.client.gateway_shared_key_file,
                        ack_key_file: value.base.client.ack_key_file,
                    },
                    credentials_database: value.base.client.database_path,
                    reply_surb_database: value.base.client.reply_surb_database_path,
                },
                allowed_list_location: value.network_requester.allowed_list_location,
                unknown_list_location: value.network_requester.unknown_list_location,
            },
            network_requester_debug: value.network_requester_debug.into(),
            logging: Default::default(),
        }
    }
}

impl MigrationNymConfig for ConfigV1_1_19 {
    fn default_root_directory() -> PathBuf {
        dirs::home_dir()
            .expect("Failed to evaluate $HOME value")
            .join(".nym")
            .join("service-providers")
            .join("network-requester")
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct NetworkRequster {
    pub allowed_list_location: PathBuf,
    pub unknown_list_location: PathBuf,
}

impl Default for NetworkRequster {
    fn default() -> Self {
        // same defaults as we had in <= v1.1.13
        NetworkRequster {
            allowed_list_location: <ConfigV1_1_19 as MigrationNymConfig>::default_root_directory()
                .join("allowed.list"),
            unknown_list_location: <ConfigV1_1_19 as MigrationNymConfig>::default_root_directory()
                .join("unknown.list"),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DebugV1_1_19 {
    #[serde(with = "humantime_serde")]
    pub standard_list_update_interval: Duration,
}

impl From<DebugV1_1_19> for DebugV1_1_20 {
    fn from(value: DebugV1_1_19) -> Self {
        DebugV1_1_20 {
            standard_list_update_interval: value.standard_list_update_interval,
        }
    }
}

impl Default for DebugV1_1_19 {
    fn default() -> Self {
        DebugV1_1_19 {
            standard_list_update_interval: DEFAULT_STANDARD_LIST_UPDATE_INTERVAL,
        }
    }
}
