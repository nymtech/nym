// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod v1_1_33 {
    use crate::config::disk_persistence::old_v1_1_33::CommonClientPathsV1_1_33;
    use crate::config::disk_persistence::CommonClientPaths;
    use crate::config::old_config_v1_1_33::OldGatewayEndpointConfigV1_1_33;
    use crate::error::ClientCoreError;
    use serde::{Deserialize, Serialize};

    mod base64 {
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        use serde::{Deserialize, Deserializer, Serializer};

        pub fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
            serializer.serialize_str(&STANDARD.encode(bytes))
        }

        pub fn deserialize<'de, D: Deserializer<'de>>(
            deserializer: D,
        ) -> Result<Vec<u8>, D::Error> {
            let s = <String>::deserialize(deserializer)?;
            STANDARD.decode(s).map_err(serde::de::Error::custom)
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(untagged)]
    enum PersistedGatewayDetails {
        /// Standard details of a remote gateway
        Default(PersistedGatewayConfig),

        /// Custom gateway setup, such as for a client embedded inside gateway itself
        Custom(PersistedCustomGatewayDetails),
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct PersistedGatewayConfig {
        /// The hash of the shared keys to ensure the correct ones are used with those gateway details.
        #[serde(with = "base64")]
        key_hash: Vec<u8>,

        /// Actual gateway details being persisted.
        details: OldGatewayEndpointConfigV1_1_33,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct PersistedCustomGatewayDetails {
        gateway_id: String,
    }

    pub async fn migrate_gateway_details(
        _old_storage_paths: &CommonClientPathsV1_1_33,
        _new_storage_paths: &CommonClientPaths,
        _preloaded_config: Option<OldGatewayEndpointConfigV1_1_33>,
    ) -> Result<(), ClientCoreError> {
        Err(ClientCoreError::UnsupportedMigration(
            "migration of legacy keys has been removed and is no longer supported".into(),
        ))
    }
}
