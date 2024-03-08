// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::old_config_v1_1_20_2::{
    ConfigV1_1_20_2, CoreConfigV1_1_20_2, SocksClientPathsV1_1_20_2,
};
use nym_bin_common::logging::LoggingSettings;
use nym_client_core::config::disk_persistence::old_v1_1_20_2::CommonClientPathsV1_1_20_2;
use nym_client_core::config::disk_persistence::old_v1_1_33::ClientKeysPathsV1_1_33;
use nym_client_core::config::old_config_v1_1_20::ConfigV1_1_20 as BaseConfigV1_1_20;
use nym_client_core::config::old_config_v1_1_20_2::ClientV1_1_20_2;
use nym_config::legacy_helpers::nym_config::MigrationNymConfig;
use nym_config::must_get_home;
use nym_socks5_client_core::config::old_config_v1_1_20_2::{
    BaseClientConfigV1_1_20_2, Socks5DebugV1_1_20_2, Socks5V1_1_20_2,
};
use nym_socks5_client_core::config::{ProviderInterfaceVersion, Socks5ProtocolVersion};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::path::PathBuf;

const DEFAULT_CONNECTION_START_SURBS: u32 = 20;
const DEFAULT_PER_REQUEST_SURBS: u32 = 3;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigV1_1_20 {
    #[serde(flatten)]
    pub base: BaseConfigV1_1_20<ConfigV1_1_20>,

    pub socks5: Socks5V1_1_20,
}

impl From<ConfigV1_1_20> for ConfigV1_1_20_2 {
    fn from(value: ConfigV1_1_20) -> Self {
        ConfigV1_1_20_2 {
            core: CoreConfigV1_1_20_2 {
                base: BaseClientConfigV1_1_20_2 {
                    client: ClientV1_1_20_2 {
                        version: value.base.client.version,
                        id: value.base.client.id,
                        disabled_credentials_mode: value.base.client.disabled_credentials_mode,
                        nyxd_urls: value.base.client.nyxd_urls,
                        nym_api_urls: value.base.client.nym_api_urls,
                        gateway_endpoint: value.base.client.gateway_endpoint.into(),
                    },
                    debug: value.base.debug.into(),
                },
                socks5: value.socks5.into(),
            },
            storage_paths: SocksClientPathsV1_1_20_2 {
                common_paths: CommonClientPathsV1_1_20_2 {
                    keys: ClientKeysPathsV1_1_33 {
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
            },
            logging: LoggingSettings::default(),
        }
    }
}

impl MigrationNymConfig for ConfigV1_1_20 {
    fn default_root_directory() -> PathBuf {
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        let base_dir = must_get_home();
        #[cfg(any(target_os = "android", target_os = "ios"))]
        let base_dir = PathBuf::from("/tmp");

        base_dir.join(".nym").join("socks5-clients")
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Socks5V1_1_20 {
    pub listening_port: u16,

    pub provider_mix_address: String,

    #[serde(default = "ProviderInterfaceVersion::new_legacy")]
    pub provider_interface_version: ProviderInterfaceVersion,

    #[serde(default = "Socks5ProtocolVersion::new_legacy")]
    pub socks5_protocol_version: Socks5ProtocolVersion,

    #[serde(default)]
    pub send_anonymously: bool,

    #[serde(default)]
    pub socks5_debug: Socks5DebugV1_1_20,
}

impl From<Socks5V1_1_20> for Socks5V1_1_20_2 {
    fn from(value: Socks5V1_1_20) -> Self {
        Socks5V1_1_20_2 {
            listening_port: value.listening_port,
            provider_mix_address: value.provider_mix_address,
            provider_interface_version: value.provider_interface_version,
            socks5_protocol_version: value.socks5_protocol_version,
            send_anonymously: value.send_anonymously,
            socks5_debug: value.socks5_debug.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Socks5DebugV1_1_20 {
    connection_start_surbs: u32,
    per_request_surbs: u32,
}

impl From<Socks5DebugV1_1_20> for Socks5DebugV1_1_20_2 {
    fn from(value: Socks5DebugV1_1_20) -> Self {
        Socks5DebugV1_1_20_2 {
            connection_start_surbs: value.connection_start_surbs,
            per_request_surbs: value.per_request_surbs,
        }
    }
}

impl Default for Socks5DebugV1_1_20 {
    fn default() -> Self {
        Socks5DebugV1_1_20 {
            connection_start_surbs: DEFAULT_CONNECTION_START_SURBS,
            per_request_surbs: DEFAULT_PER_REQUEST_SURBS,
        }
    }
}
