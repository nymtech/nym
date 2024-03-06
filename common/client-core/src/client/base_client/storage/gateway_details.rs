// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::GatewayEndpointConfig;
use crate::error::ClientCoreError;
use async_trait::async_trait;
use log::error;
use nym_gateway_requests::registration::handshake::SharedKeys;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::ops::Deref;
use tokio::sync::Mutex;
use zeroize::Zeroizing;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait GatewayDetailsStore {
    type StorageError: Error;

    async fn load_gateway_details(&self) -> Result<PersistedGatewayDetails, Self::StorageError>;

    async fn store_gateway_details(
        &self,
        details: &PersistedGatewayDetails,
    ) -> Result<(), Self::StorageError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PersistedGatewayDetails {
    /// Standard details of a remote gateway
    Default(PersistedGatewayConfig),

    /// Custom gateway setup, such as for a client embedded inside gateway itself
    Custom(PersistedCustomGatewayDetails),
}

impl PersistedGatewayDetails {
    // TODO: this should probably allow for custom verification over T
    pub fn validate(&self, shared_key: Option<&SharedKeys>) -> Result<(), ClientCoreError> {
        match self {
            PersistedGatewayDetails::Default(details) => {
                if !details.verify(shared_key.ok_or(ClientCoreError::UnavailableSharedKey)?) {
                    Err(ClientCoreError::MismatchedGatewayDetails {
                        gateway_id: details.details.gateway_id.clone(),
                    })
                } else {
                    Ok(())
                }
            }
            PersistedGatewayDetails::Custom(_) => {
                if shared_key.is_some() {
                    error!("using custom persisted gateway setup with shared key present - are you sure that's what you want?");
                    // but technically we could still continue. just ignore the key
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PersistedGatewayConfig {
    // TODO: should we also verify correctness of the details themselves?
    // i.e. we could include a checksum or tag (via the shared keys)
    // counterargument: if we wanted to modify, say, the host information in the stored file on disk,
    // in order to actually use it, we'd have to recompute the whole checksum which would be a huge pain.
    /// The hash of the shared keys to ensure the correct ones are used with those gateway details.
    #[serde(with = "base64")]
    key_hash: Vec<u8>,

    /// Actual gateway details being persisted.
    pub details: GatewayEndpointConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedCustomGatewayDetails {
    // whatever custom method is used, gateway's identity must be known
    pub gateway_id: String,

    #[serde(flatten)]
    pub additional_data: Vec<u8>,
}

impl PersistedGatewayConfig {
    pub fn new(details: GatewayEndpointConfig, shared_key: &SharedKeys) -> Self {
        let key_bytes = Zeroizing::new(shared_key.to_bytes());

        let mut key_hasher = Sha256::new();
        key_hasher.update(&key_bytes);
        let key_hash = key_hasher.finalize().to_vec();

        PersistedGatewayConfig { key_hash, details }
    }

    pub fn verify(&self, shared_key: &SharedKeys) -> bool {
        let key_bytes = Zeroizing::new(shared_key.to_bytes());

        let mut key_hasher = Sha256::new();
        key_hasher.update(&key_bytes);
        let key_hash = key_hasher.finalize();

        self.key_hash == key_hash.deref()
    }
}

impl PersistedGatewayDetails {
    // pub fn new(
    //     details: GatewayDetails,
    //     shared_key: Option<&SharedKeys>,
    // ) -> Result<Self, ClientCoreError> {
    //     match details {
    //         GatewayDetails::Configured(cfg) => {
    //             let shared_key = shared_key.ok_or(ClientCoreError::UnavailableSharedKey)?;
    //             Ok(PersistedGatewayDetails::Default(
    //                 PersistedGatewayConfig::new(cfg, shared_key),
    //             ))
    //         }
    //         GatewayDetails::Custom(custom) => Ok(PersistedGatewayDetails::Custom(custom.into())),
    //     }
    // }
    //
    // pub fn is_custom(&self) -> bool {
    //     matches!(self, PersistedGatewayDetails::Custom(..))
    // }
    //
    // pub fn matches(&self, other: &GatewayDetails) -> bool {
    //     match self {
    //         PersistedGatewayDetails::Default(default) => {
    //             if let GatewayDetails::Configured(other_configured) = other {
    //                 &default.details == other_configured
    //             } else {
    //                 false
    //             }
    //         }
    //         PersistedGatewayDetails::Custom(custom) => {
    //             if let GatewayDetails::Custom(other_custom) = other {
    //                 custom.gateway_id == other_custom.gateway_id
    //                     && custom.additional_data == other_custom.additional_data
    //             } else {
    //                 false
    //             }
    //         }
    //     }
    // }
}

// helper to make Vec<u8> serialization use base64 representation to make it human readable
// so that it would be easier for users to copy contents from the disk if they wanted to use it elsewhere
mod base64 {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        let s = <String>::deserialize(deserializer)?;
        STANDARD.decode(s).map_err(serde::de::Error::custom)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, thiserror::Error)]
pub enum OnDiskGatewayDetailsError {
    #[error("JSON failure: {0}")]
    SerializationFailure(#[from] serde_json::Error),

    #[error("failed to store gateway details to {path}: {err}")]
    StoreFailure {
        path: String,
        #[source]
        err: std::io::Error,
    },

    #[error("failed to load gateway details from {path}: {err}")]
    LoadFailure {
        path: String,
        #[source]
        err: std::io::Error,
    },
}

#[cfg(not(target_arch = "wasm32"))]
pub struct OnDiskGatewayDetails {
    file_location: std::path::PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
impl OnDiskGatewayDetails {
    pub fn new<P: AsRef<std::path::Path>>(path: P) -> Self {
        OnDiskGatewayDetails {
            file_location: path.as_ref().to_owned(),
        }
    }

    pub fn load_from_disk(&self) -> Result<PersistedGatewayDetails, OnDiskGatewayDetailsError> {
        let file = std::fs::File::open(&self.file_location).map_err(|err| {
            OnDiskGatewayDetailsError::LoadFailure {
                path: self.file_location.display().to_string(),
                err,
            }
        })?;

        Ok(serde_json::from_reader(file)?)
    }

    pub fn store_to_disk(
        &self,
        details: &PersistedGatewayDetails,
    ) -> Result<(), OnDiskGatewayDetailsError> {
        // ensure the whole directory structure exists
        if let Some(parent_dir) = &self.file_location.parent() {
            std::fs::create_dir_all(parent_dir).map_err(|err| {
                OnDiskGatewayDetailsError::StoreFailure {
                    path: self.file_location.display().to_string(),
                    err,
                }
            })?
        }

        let file = std::fs::File::create(&self.file_location).map_err(|err| {
            OnDiskGatewayDetailsError::StoreFailure {
                path: self.file_location.display().to_string(),
                err,
            }
        })?;

        Ok(serde_json::to_writer_pretty(file, details)?)
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl GatewayDetailsStore for OnDiskGatewayDetails {
    type StorageError = OnDiskGatewayDetailsError;

    async fn load_gateway_details(&self) -> Result<PersistedGatewayDetails, Self::StorageError> {
        self.load_from_disk()
    }

    async fn store_gateway_details(
        &self,
        gateway_details: &PersistedGatewayDetails,
    ) -> Result<(), Self::StorageError> {
        self.store_to_disk(gateway_details)
    }
}

#[derive(Default)]
pub struct InMemGatewayDetails {
    details: Mutex<Option<PersistedGatewayDetails>>,
}

#[derive(Debug, thiserror::Error)]
#[error("old ephemeral gateway details can't be loaded from storage")]
pub struct EphemeralGatewayDetailsError;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl GatewayDetailsStore for InMemGatewayDetails {
    type StorageError = EphemeralGatewayDetailsError;

    async fn load_gateway_details(&self) -> Result<PersistedGatewayDetails, Self::StorageError> {
        self.details
            .lock()
            .await
            .clone()
            .ok_or(EphemeralGatewayDetailsError)
    }

    async fn store_gateway_details(
        &self,
        gateway_details: &PersistedGatewayDetails,
    ) -> Result<(), Self::StorageError> {
        *self.details.lock().await = Some(gateway_details.clone());
        Ok(())
    }
}
