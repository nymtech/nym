// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::GatewayEndpointConfig;
use async_trait::async_trait;
use nym_gateway_requests::registration::handshake::SharedKeys;
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
pub struct PersistedGatewayDetails {
    // TODO: should we also verify correctness of the details themselves?
    // i.e. we could include a checksum or tag (via the shared keys)
    // counterargument: if we wanted to modify, say, the host information in the stored file on disk,
    // in order to actually use it, we'd have to recompute the whole checksum which would be a huge pain.
    /// The hash of the shared keys to ensure the correct ones are used with those gateway details.
    #[serde(with = "base64")]
    key_hash: Vec<u8>,

    /// Actual gateway details being persisted.
    pub(crate) details: GatewayEndpointConfig,
}

impl From<PersistedGatewayDetails> for GatewayEndpointConfig {
    fn from(value: PersistedGatewayDetails) -> Self {
        value.details
    }
}

impl PersistedGatewayDetails {
    pub fn new(details: GatewayEndpointConfig, shared_key: &SharedKeys) -> Self {
        let key_bytes = Zeroizing::new(shared_key.to_bytes());

        let mut key_hasher = Sha256::new();
        key_hasher.update(&key_bytes);
        let key_hash = key_hasher.finalize().to_vec();

        PersistedGatewayDetails { key_hash, details }
    }

    pub fn verify(&self, shared_key: &SharedKeys) -> bool {
        let key_bytes = Zeroizing::new(shared_key.to_bytes());

        let mut key_hasher = Sha256::new();
        key_hasher.update(&key_bytes);
        let key_hash = key_hasher.finalize();

        self.key_hash == key_hash.deref()
    }
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
        let s = <&str>::deserialize(deserializer)?;
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
