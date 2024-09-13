// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod v1_1_33 {
    use crate::client::base_client::{
        non_wasm_helpers::setup_fs_gateways_storage,
        storage::helpers::{set_active_gateway, store_gateway_details},
    };
    use crate::config::disk_persistence::old_v1_1_33::CommonClientPathsV1_1_33;
    use crate::config::disk_persistence::CommonClientPaths;
    use crate::config::old_config_v1_1_33::OldGatewayEndpointConfigV1_1_33;
    use crate::error::ClientCoreError;
    use nym_client_core_gateways_storage::{
        CustomGatewayDetails, GatewayDetails, GatewayRegistration, RemoteGatewayDetails,
    };
    use nym_gateway_requests::registration::handshake::LegacySharedKeys;
    use serde::{Deserialize, Serialize};
    use sha2::{digest::Digest, Sha256};
    use std::ops::Deref;
    use std::path::Path;
    use std::sync::Arc;
    use zeroize::Zeroizing;

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

    impl PersistedGatewayConfig {
        fn verify(&self, shared_key: &LegacySharedKeys) -> bool {
            let key_bytes = Zeroizing::new(shared_key.to_bytes());

            let mut key_hasher = Sha256::new();
            key_hasher.update(&key_bytes);
            let key_hash = key_hasher.finalize();

            self.key_hash == key_hash.deref()
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct PersistedCustomGatewayDetails {
        gateway_id: String,
    }

    fn load_shared_key<P: AsRef<Path>>(path: P) -> Result<LegacySharedKeys, ClientCoreError> {
        // the shared key was a simple pem file
        Ok(nym_pemstore::load_key(path)?)
    }

    fn gateway_details_from_raw(
        gateway_id: String,
        gateway_owner: String,
        gateway_listener: String,
        gateway_shared_key: LegacySharedKeys,
    ) -> Result<GatewayDetails, ClientCoreError> {
        Ok(GatewayDetails::Remote(RemoteGatewayDetails {
            gateway_id: gateway_id
                .parse()
                .map_err(|err| ClientCoreError::UpgradeFailure {
                    message: format!("the stored gateway id was malformed: {err}"),
                })?,
            derived_aes128_ctr_blake3_hmac_keys: Arc::new(gateway_shared_key),
            gateway_owner_address: Some(gateway_owner.parse().map_err(|err| {
                ClientCoreError::UpgradeFailure {
                    message: format!("the stored gateway owner address was malformed: {err}"),
                }
            })?),
            gateway_listener: gateway_listener.parse().map_err(|err| {
                ClientCoreError::UpgradeFailure {
                    message: format!("the stored gateway listener address was malformed: {err}"),
                }
            })?,
        }))
    }

    // helper to extract shared key and gateway details into the new GatewayRegistration
    fn extract_gateway_registration(
        storage_paths: &CommonClientPathsV1_1_33,
    ) -> Result<GatewayRegistration, ClientCoreError> {
        let details_file = std::fs::File::open(&storage_paths.gateway_details).map_err(|err| {
            ClientCoreError::UpgradeFailure {
                message: format!(
                    "failed to open gateway details file at {}: {err}",
                    storage_paths.gateway_details.display()
                ),
            }
        })?;

        // in v1.1.33 of the clients, the gateway details struct was saved as json
        let details: PersistedGatewayDetails =
            serde_json::from_reader(details_file).map_err(|err| {
                ClientCoreError::UpgradeFailure {
                    message: format!(
                        "failed to deserialize gateway details from {}: {err}",
                        storage_paths.gateway_details.display()
                    ),
                }
            })?;

        let details = match details {
            PersistedGatewayDetails::Default(config) => {
                let gateway_shared_key =
                    load_shared_key(&storage_paths.keys.gateway_shared_key_file)?;
                if !config.verify(&gateway_shared_key) {
                    return Err(ClientCoreError::UpgradeFailure {
                        message: "failed to verify consistency of the existing gateway details"
                            .to_string(),
                    });
                }
                gateway_details_from_raw(
                    config.details.gateway_id,
                    config.details.gateway_owner,
                    config.details.gateway_listener,
                    gateway_shared_key,
                )?
            }
            PersistedGatewayDetails::Custom(custom) => {
                GatewayDetails::Custom(CustomGatewayDetails {
                    gateway_id: custom.gateway_id.parse().map_err(|err| {
                        ClientCoreError::UpgradeFailure {
                            message: format!("the stored gateway id was malformed: {err}"),
                        }
                    })?,
                    data: None,
                })
            }
        };

        Ok(details.into())
    }

    // it's responsibility of the caller to ensure this is called **after** new registration has already been saved
    fn remove_old_gateway_details(storage_paths: &CommonClientPathsV1_1_33) -> std::io::Result<()> {
        std::fs::remove_file(&storage_paths.gateway_details)?;

        if storage_paths.keys.gateway_shared_key_file.exists() {
            std::fs::remove_file(&storage_paths.keys.gateway_shared_key_file)?;
        }
        Ok(())
    }

    pub async fn migrate_gateway_details(
        old_storage_paths: &CommonClientPathsV1_1_33,
        new_storage_paths: &CommonClientPaths,
        preloaded_config: Option<OldGatewayEndpointConfigV1_1_33>,
    ) -> Result<(), ClientCoreError> {
        let gateway_registration = match preloaded_config {
            Some(config) => {
                let gateway_shared_key =
                    load_shared_key(&old_storage_paths.keys.gateway_shared_key_file)?;
                gateway_details_from_raw(
                    config.gateway_id,
                    config.gateway_owner,
                    config.gateway_listener,
                    gateway_shared_key,
                )?
                .into()
            }
            None => extract_gateway_registration(old_storage_paths)?,
        };

        // since we're migrating to a brand new store, the store should be empty
        // and thus set the 'new' gateway as the active one
        let details_store =
            setup_fs_gateways_storage(&new_storage_paths.gateway_registrations).await?;
        store_gateway_details(&details_store, &gateway_registration).await?;
        set_active_gateway(
            &details_store,
            &gateway_registration.details.gateway_id().to_base58_string(),
        )
        .await?;

        remove_old_gateway_details(old_storage_paths).map_err(|err| {
            ClientCoreError::UpgradeFailure {
                message: format!("failed to remove old data: {err}"),
            }
        })
    }
}
