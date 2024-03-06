// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::base_client::storage::gateway_details::{
    GatewayDetailsStore, PersistedCustomGatewayDetails, PersistedGatewayDetails,
};
use crate::client::key_manager::persistence::KeyStore;
use crate::client::key_manager::ManagedKeys;
use crate::config::{Config, GatewayEndpointConfig};
use crate::error::ClientCoreError;
use crate::init::{_load_gateway_details, _load_managed_keys, setup_gateway};
use nym_gateway_client::client::InitGatewayClient;
use nym_gateway_requests::registration::handshake::SharedKeys;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::addressing::nodes::NodeIdentity;
use nym_topology::gateway;
use nym_validator_client::client::IdentityKey;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::sync::Arc;

/// Result of registering with a gateway:
/// - shared keys derived between ourselves and the node
/// - an authenticated handle of an ephemeral handle created for the purposes of registration
pub struct RegistrationResult {
    pub shared_keys: Arc<SharedKeys>,
    pub authenticated_ephemeral_client: InitGatewayClient,
}

/// Result of fully initialising a client:
/// - details of the associated gateway
/// - all loaded (or derived) keys
/// - an optional authenticated handle of an ephemeral gateway handle created for the purposes of registration,
///   if this was the first time this client registered
pub struct InitialisationResult {
    pub gateway_details: GatewayDetails,
    pub managed_keys: ManagedKeys,
    pub authenticated_ephemeral_client: Option<InitGatewayClient>,
}

impl InitialisationResult {
    pub fn new_loaded(gateway_details: GatewayDetails, managed_keys: ManagedKeys) -> Self {
        InitialisationResult {
            gateway_details,
            managed_keys,
            authenticated_ephemeral_client: None,
        }
    }

    pub async fn try_load<K, D>(key_store: &K, details_store: &D) -> Result<Self, ClientCoreError>
    where
        K: KeyStore,
        D: GatewayDetailsStore,
        K::StorageError: Send + Sync + 'static,
        D::StorageError: Send + Sync + 'static,
    {
        todo!()
        // let loaded_details = _load_gateway_details(details_store).await?;
        // let loaded_keys = _load_managed_keys(key_store).await?;
        //
        // match &loaded_details {
        //     PersistedGatewayDetails::Default(loaded_default) => {
        //         if !loaded_default.verify(&loaded_keys.must_get_gateway_shared_key()) {
        //             return Err(ClientCoreError::MismatchedGatewayDetails {
        //                 gateway_id: loaded_default.details.gateway_id.clone(),
        //             });
        //         }
        //     }
        //     PersistedGatewayDetails::Custom(_) => {}
        // }
        //
        // Ok(InitialisationResult {
        //     gateway_details: loaded_details.into(),
        //     managed_keys: loaded_keys,
        //     authenticated_ephemeral_client: None,
        // })
    }

    pub fn client_address(&self) -> Result<Recipient, ClientCoreError> {
        let client_recipient = Recipient::new(
            *self.managed_keys.identity_public_key(),
            *self.managed_keys.encryption_public_key(),
            // TODO: below only works under assumption that gateway address == gateway id
            // (which currently is true)
            NodeIdentity::from_base58_string(self.gateway_details.gateway_id())?,
        );

        Ok(client_recipient)
    }
}

/// Details of particular gateway client got registered with
#[derive(Debug, Clone)]
pub enum GatewayDetails {
    /// Standard details of a remote gateway
    Configured(GatewayEndpointConfig),

    /// Custom gateway setup, such as for a client embedded inside gateway itself
    Custom(CustomGatewayDetails),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomGatewayDetails {
    // whatever custom method is used, gateway's identity must be known
    pub gateway_id: String,

    pub additional_data: Vec<u8>,
}

impl CustomGatewayDetails {
    pub fn new(gateway_id: String, additional_data: Vec<u8>) -> Self {
        Self {
            gateway_id,
            additional_data,
        }
    }
}

impl GatewayDetails {
    pub fn try_get_configured_endpoint(&self) -> Option<&GatewayEndpointConfig> {
        if let GatewayDetails::Configured(endpoint) = &self {
            Some(endpoint)
        } else {
            None
        }
    }

    pub fn is_custom(&self) -> bool {
        matches!(self, GatewayDetails::Custom(_))
    }

    pub fn gateway_id(&self) -> &str {
        match self {
            GatewayDetails::Configured(cfg) => &cfg.gateway_id,
            GatewayDetails::Custom(custom) => &custom.gateway_id,
        }
    }
}

impl From<GatewayEndpointConfig> for GatewayDetails {
    fn from(value: GatewayEndpointConfig) -> Self {
        GatewayDetails::Configured(value)
    }
}

impl From<PersistedCustomGatewayDetails> for CustomGatewayDetails {
    fn from(value: PersistedCustomGatewayDetails) -> Self {
        CustomGatewayDetails {
            gateway_id: value.gateway_id,
            additional_data: value.additional_data,
        }
    }
}

impl From<CustomGatewayDetails> for PersistedCustomGatewayDetails {
    fn from(value: CustomGatewayDetails) -> Self {
        PersistedCustomGatewayDetails {
            gateway_id: value.gateway_id,
            additional_data: value.additional_data,
        }
    }
}

impl From<PersistedGatewayDetails> for GatewayDetails {
    fn from(value: PersistedGatewayDetails) -> Self {
        match value {
            PersistedGatewayDetails::Default(default) => {
                GatewayDetails::Configured(default.details)
            }
            PersistedGatewayDetails::Custom(custom) => GatewayDetails::Custom(custom.into()),
        }
    }
}

#[derive(Clone, Debug)]
pub enum GatewaySelectionSpecification {
    /// Uniformly choose a random remote gateway.
    UniformRemote { must_use_tls: bool },

    /// Should the new, remote, gateway be selected based on latency.
    RemoteByLatency { must_use_tls: bool },

    /// Gateway with this specific identity should be chosen.
    // JS: I don't really like the name of this enum variant but couldn't think of anything better at the time
    Specified {
        must_use_tls: bool,
        identity: IdentityKey,
    },

    // TODO: this doesn't really fit in here..., but where else to put it?
    /// This client has handled the selection by itself
    Custom {
        gateway_identity: String,
        additional_data: Vec<u8>,
    },
}

impl Default for GatewaySelectionSpecification {
    fn default() -> Self {
        GatewaySelectionSpecification::UniformRemote {
            must_use_tls: false,
        }
    }
}

impl GatewaySelectionSpecification {
    pub fn new(
        gateway_identity: Option<String>,
        latency_based_selection: Option<bool>,
        must_use_tls: bool,
    ) -> Self {
        if let Some(identity) = gateway_identity {
            GatewaySelectionSpecification::Specified {
                identity,
                must_use_tls,
            }
        } else if let Some(true) = latency_based_selection {
            GatewaySelectionSpecification::RemoteByLatency { must_use_tls }
        } else {
            GatewaySelectionSpecification::UniformRemote { must_use_tls }
        }
    }
}

pub enum GatewaySetup {
    /// The gateway specification (details + keys) MUST BE loaded from the underlying storage.
    MustLoad,

    /// Specifies usage of a new gateway
    New {
        specification: GatewaySelectionSpecification,

        // TODO: seems to be a bit inefficient to pass them by value
        available_gateways: Vec<gateway::Node>,

        /// Specifies whether old data should be overwritten whilst setting up new gateway client.
        overwrite_data: bool,
    },

    ReuseConnection {
        /// The authenticated ephemeral client that was created during `init`
        authenticated_ephemeral_client: InitGatewayClient,

        // Details of this pre-initialised client (i.e. gateway and keys)
        gateway_details: GatewayDetails,

        managed_keys: ManagedKeys,
    },
}

impl GatewaySetup {
    pub fn try_reuse_connection(init_res: InitialisationResult) -> Result<Self, ClientCoreError> {
        if let Some(authenticated_ephemeral_client) = init_res.authenticated_ephemeral_client {
            Ok(GatewaySetup::ReuseConnection {
                authenticated_ephemeral_client,
                gateway_details: init_res.gateway_details,
                managed_keys: init_res.managed_keys,
            })
        } else {
            Err(ClientCoreError::NoInitClientPresent)
        }
    }

    pub async fn try_setup<K, D>(
        self,
        key_store: &K,
        details_store: &D,
    ) -> Result<InitialisationResult, ClientCoreError>
    where
        K: KeyStore,
        D: GatewayDetailsStore,
        K::StorageError: Send + Sync + 'static,
        D::StorageError: Send + Sync + 'static,
    {
        todo!()
        // setup_gateway(self, key_store, details_store).await
    }

    pub fn is_must_load(&self) -> bool {
        matches!(self, GatewaySetup::MustLoad)
    }

    pub fn has_full_details(&self) -> bool {
        self.is_must_load()
    }
}

/// Struct describing the results of the client initialization procedure.
#[derive(Debug, Serialize)]
pub struct InitResults {
    pub version: String,
    pub id: String,
    pub identity_key: String,
    pub encryption_key: String,
    pub gateway_id: String,
    pub gateway_listener: String,
    pub address: Recipient,
}

impl InitResults {
    pub fn new(config: &Config, address: Recipient, gateway: &GatewayEndpointConfig) -> Self {
        Self {
            version: config.client.version.clone(),
            id: config.client.id.clone(),
            identity_key: address.identity().to_base58_string(),
            encryption_key: address.encryption_key().to_base58_string(),
            gateway_id: gateway.gateway_id.clone(),
            gateway_listener: gateway.gateway_listener.clone(),
            address,
        }
    }
}

impl Display for InitResults {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Version: {}", self.version)?;
        writeln!(f, "ID: {}", self.id)?;
        writeln!(f, "Identity key: {}", self.identity_key)?;
        writeln!(f, "Encryption: {}", self.encryption_key)?;
        writeln!(f, "Gateway ID: {}", self.gateway_id)?;
        write!(f, "Gateway: {}", self.gateway_listener)
    }
}
