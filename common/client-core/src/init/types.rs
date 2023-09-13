// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::base_client::storage::gateway_details::{
    GatewayDetailsStore, PersistedCustomGatewayDetails, PersistedGatewayDetails,
};
use crate::client::key_manager::persistence::KeyStore;
use crate::client::key_manager::ManagedKeys;
use crate::config::{Config, GatewayEndpointConfig};
use crate::error::ClientCoreError;
use crate::init::helpers::{choose_gateway_by_latency, current_gateways, uniformly_random_gateway};
use crate::init::{_load_gateway_details, _load_managed_keys};
use nym_crypto::asymmetric::identity;
use nym_gateway_client::client::InitOnly;
use nym_gateway_client::GatewayClient;
use nym_gateway_requests::registration::handshake::SharedKeys;
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::addressing::nodes::NodeIdentity;
use nym_topology::gateway;
use nym_validator_client::client::IdentityKey;
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::sync::Arc;
use url::Url;

/// Result of registering with a gateway:
/// - shared keys derived between ourselves and the node
/// - an authenticated handle of an ephemeral handle created for the purposes of registration
pub struct RegistrationResult {
    pub shared_keys: Arc<SharedKeys>,
    pub authenticated_ephemeral_client: GatewayClient<InitOnly>,
}

/// Result of fully initialising a client:
/// - details of the associated gateway
/// - all loaded (or derived) keys
/// - an optional authenticated handle of an ephemeral gateway handle created for the purposes of registration,
///   if this was the first time this client registered
pub struct InitialisationResult {
    pub gateway_details: GatewayDetails,
    pub managed_keys: ManagedKeys,
    pub authenticated_ephemeral_client: Option<GatewayClient<InitOnly>>,
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
        let loaded_details = _load_gateway_details(details_store).await?;
        let loaded_keys = _load_managed_keys(key_store).await?;

        match &loaded_details {
            PersistedGatewayDetails::Default(loaded_default) => {
                if !loaded_default.verify(&loaded_keys.must_get_gateway_shared_key()) {
                    return Err(ClientCoreError::MismatchedGatewayDetails {
                        gateway_id: loaded_default.details.gateway_id.clone(),
                    });
                }
            }
            PersistedGatewayDetails::Custom(_) => {}
        }

        Ok(InitialisationResult {
            gateway_details: loaded_details.into(),
            managed_keys: loaded_keys,
            authenticated_ephemeral_client: None,
        })
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
pub enum GatewayDetails<T = EmptyCustomDetails> {
    /// Standard details of a remote gateway
    Configured(GatewayEndpointConfig),

    /// Custom gateway setup, such as for a client embedded inside gateway itself
    Custom(CustomGatewayDetails<T>),
}

#[derive(Debug, Default, Copy, Clone, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct EmptyCustomDetails {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomGatewayDetails<T = EmptyCustomDetails> {
    // whatever custom method is used, gateway's identity must be known
    pub gateway_id: String,

    #[serde(flatten)]
    pub additional_data: T,
}

impl<T> GatewayDetails<T> {
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

impl<T> From<GatewayEndpointConfig> for GatewayDetails<T> {
    fn from(value: GatewayEndpointConfig) -> Self {
        GatewayDetails::Configured(value)
    }
}

impl<T> From<PersistedCustomGatewayDetails<T>> for CustomGatewayDetails<T> {
    fn from(value: PersistedCustomGatewayDetails<T>) -> Self {
        CustomGatewayDetails {
            gateway_id: value.gateway_id,
            additional_data: value.additional_data,
        }
    }
}

impl<T> From<CustomGatewayDetails<T>> for PersistedCustomGatewayDetails<T> {
    fn from(value: CustomGatewayDetails<T>) -> Self {
        PersistedCustomGatewayDetails {
            gateway_id: value.gateway_id,
            additional_data: value.additional_data,
        }
    }
}

impl<T> From<PersistedGatewayDetails<T>> for GatewayDetails<T> {
    fn from(value: PersistedGatewayDetails<T>) -> Self {
        match value {
            PersistedGatewayDetails::Default(default) => {
                GatewayDetails::Configured(default.details)
            }
            PersistedGatewayDetails::Custom(custom) => GatewayDetails::Custom(custom.into()),
        }
    }
}

#[derive(Clone, Default)]
pub enum GatewaySelectionSpecification<T = EmptyCustomDetails> {
    /// Uniformly choose a random remote gateway.
    #[default]
    UniformRemote,

    /// Should the new, remote, gateway be selected based on latency.
    RemoteByLatency,

    /// This client will handle the selection by itself
    Custom {
        gateway_identity: String,
        additional_data: T,
    },
}

impl<T> GatewaySelectionSpecification<T> {
    pub(crate) fn is_custom(&self) -> bool {
        matches!(self, GatewaySelectionSpecification::Custom { .. })
    }
}

pub enum GatewaySetup<T = EmptyCustomDetails> {
    /// The gateway specification MUST BE loaded from the underlying storage.
    MustLoad,

    /// Specifies usage of a new, random, gateway.
    New {
        specification: GatewaySelectionSpecification<T>,
    },

    Specified {
        /// Identity key of the gateway we want to try to use, if applicable
        gateway_identity: IdentityKey,
    },

    Predefined {
        /// Full gateway configuration
        details: GatewayDetails<T>,
    },

    ReuseConnection {
        /// The authenticated ephemeral client that was created during `init`
        authenticated_ephemeral_client: GatewayClient<InitOnly>,

        // Details of this pre-initialised client (i.e. gateway and keys)
        gateway_details: GatewayDetails,
        
        #[deprecated]
        // rethink it. this field shouldn't be required as if you're reusing a connection, you must already have loaded keys in memory
        managed_keys: ManagedKeys,
    },
}

impl From<GatewayDetails> for GatewaySetup {
    fn from(details: GatewayDetails) -> Self {
        GatewaySetup::Predefined { details }
    }
}

impl From<IdentityKey> for GatewaySetup {
    fn from(gateway_identity: IdentityKey) -> Self {
        GatewaySetup::Specified { gateway_identity }
    }
}

impl Default for GatewaySetup {
    fn default() -> Self {
        GatewaySetup::New {
            specification: Default::default(),
        }
    }
}

impl<T> GatewaySetup<T> {
    pub fn new_fresh(
        gateway_identity: Option<String>,
        latency_based_selection: Option<bool>,
    ) -> Self {
        if let Some(gateway_identity) = gateway_identity {
            GatewaySetup::Specified { gateway_identity }
        } else {
            let specification = if let Some(true) = latency_based_selection {
                GatewaySelectionSpecification::RemoteByLatency
            } else {
                GatewaySelectionSpecification::UniformRemote
            };

            GatewaySetup::New { specification }
        }
    }

    pub fn is_must_load(&self) -> bool {
        matches!(self, GatewaySetup::MustLoad)
    }

    pub fn has_full_details(&self) -> bool {
        matches!(self, GatewaySetup::Predefined { .. }) || self.is_must_load()
    }

    pub fn is_custom_new(&self) -> bool {
        if let GatewaySetup::New { specification } = self {
            specification.is_custom()
        } else {
            false
        }
    }

    pub async fn choose_gateway(
        self,
        gateways: &[gateway::Node],
    ) -> Result<GatewayDetails<T>, ClientCoreError> {
        let cfg: GatewayEndpointConfig = match self {
            GatewaySetup::New { specification } => match specification {
                GatewaySelectionSpecification::UniformRemote => {
                    let mut rng = OsRng;
                    uniformly_random_gateway(&mut rng, gateways)
                }
                GatewaySelectionSpecification::RemoteByLatency => {
                    let mut rng = OsRng;
                    choose_gateway_by_latency(&mut rng, gateways).await
                }
                GatewaySelectionSpecification::Custom {
                    gateway_identity,
                    additional_data,
                } => {
                    return Ok(GatewayDetails::Custom(CustomGatewayDetails {
                        gateway_id: gateway_identity,
                        additional_data,
                    }))
                }
            }
            .map(Into::into),
            GatewaySetup::Specified { gateway_identity } => {
                let user_gateway = identity::PublicKey::from_base58_string(&gateway_identity)
                    .map_err(ClientCoreError::UnableToCreatePublicKeyFromGatewayId)?;

                gateways
                    .iter()
                    .find(|gateway| gateway.identity_key == user_gateway)
                    .ok_or_else(|| ClientCoreError::NoGatewayWithId(gateway_identity.to_string()))
                    .cloned()
            }
            .map(Into::into),
            _ => Err(ClientCoreError::UnexpectedGatewayDetails),
        }?;
        Ok(cfg.into())
    }

    pub async fn try_get_new_gateway_details(
        self,
        validator_servers: &[Url],
    ) -> Result<GatewayDetails<T>, ClientCoreError> {
        let mut rng = OsRng;
        let gateways = current_gateways(&mut rng, validator_servers).await?;
        self.choose_gateway(&gateways).await
    }
}

/// Struct describing the results of the client initialization procedure.
#[derive(Debug, Serialize)]
pub struct InitResults {
    version: String,
    id: String,
    identity_key: String,
    encryption_key: String,
    gateway_id: String,
    gateway_listener: String,
}

impl InitResults {
    pub fn new(config: &Config, address: &Recipient, gateway: &GatewayEndpointConfig) -> Self {
        Self {
            version: config.client.version.clone(),
            id: config.client.id.clone(),
            identity_key: address.identity().to_base58_string(),
            encryption_key: address.encryption_key().to_base58_string(),
            gateway_id: gateway.gateway_id.clone(),
            gateway_listener: gateway.gateway_listener.clone(),
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
