// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::key_manager::ClientKeys;
use crate::client::key_manager::gateway_keys::GatewayClientKeys;
use crate::client::key_manager::persistence::KeyStore;
use crate::config::Config;
use crate::error::ClientCoreError;
use crate::init::{setup_gateway, use_loaded_gateway_details};
use nym_client_core_gateways_storage::{
    GatewayRegistration, GatewaysDetailsStore, RemoteGatewayDetails,
};
use nym_crypto::asymmetric::ed25519;
use nym_gateway_client::SharedSymmetricKey;
use nym_gateway_client::client::{GatewayListeners, InitGatewayClient};
use nym_sphinx::addressing::clients::Recipient;
use nym_topology::node::RoutingNode;
use nym_validator_client::client::IdentityKey;
use serde::Serialize;
use std::fmt::{Debug, Display};
#[cfg(unix)]
use std::os::fd::RawFd;
use std::sync::Arc;
use time::OffsetDateTime;
use url::Url;

pub enum SelectedGateway {
    Remote {
        gateway_id: ed25519::PublicKey,

        gateway_listeners: GatewayListeners,
    },
    Custom {
        gateway_id: ed25519::PublicKey,
        additional_data: Option<Vec<u8>>,
    },
}

impl SelectedGateway {
    pub fn from_topology_node(
        node: RoutingNode,
        must_use_tls: bool,
        no_hostname: bool,
    ) -> Result<Self, ClientCoreError> {
        // for now, let's use 'old' behaviour, if you want to change it, you can pass it up the enum stack yourself : )
        let prefer_ipv6 = false;

        let (gateway_listener, fallback_listener) = if must_use_tls {
            // WSS main, no fallback
            let primary =
                node.ws_entry_address_tls()
                    .ok_or(ClientCoreError::UnsupportedWssProtocol {
                        gateway: node.identity_key.to_base58_string(),
                    })?;
            (primary, None)
        } else {
            let (maybe_primary, fallback) =
                node.ws_entry_address_with_fallback(prefer_ipv6, no_hostname);
            (
                maybe_primary.ok_or(ClientCoreError::UnsupportedEntry {
                    id: node.node_id,
                    identity: node.identity_key.to_base58_string(),
                })?,
                fallback,
            )
        };

        let fallback_listener_url = fallback_listener.and_then(|address| {
            Url::parse(&address)
                .inspect_err(|err| {
                    tracing::warn!("Malformed fallback listener, none will be used : {err}")
                })
                .ok()
        });

        let gateway_listener_url =
            Url::parse(&gateway_listener).map_err(|source| ClientCoreError::MalformedListener {
                gateway_id: node.identity_key.to_base58_string(),
                raw_listener: gateway_listener,
                source,
            })?;

        Ok(SelectedGateway::Remote {
            gateway_id: node.identity_key,
            gateway_listeners: GatewayListeners {
                primary: gateway_listener_url,
                fallback: fallback_listener_url,
            },
        })
    }

    pub fn custom(
        gateway_id: String,
        additional_data: Option<Vec<u8>>,
    ) -> Result<Self, ClientCoreError> {
        let gateway_id = ed25519::PublicKey::from_base58_string(&gateway_id)
            .map_err(|source| ClientCoreError::MalformedGatewayIdentity { gateway_id, source })?;

        Ok(SelectedGateway::Custom {
            gateway_id,
            additional_data,
        })
    }

    pub fn gateway_id(&self) -> &ed25519::PublicKey {
        match self {
            SelectedGateway::Remote { gateway_id, .. } => gateway_id,
            SelectedGateway::Custom { gateway_id, .. } => gateway_id,
        }
    }
}

/// Result of registering with a gateway:
/// - shared keys derived between ourselves and the node
/// - an authenticated handle of an ephemeral handle created for the purposes of registration
pub struct RegistrationResult {
    pub shared_keys: Arc<SharedSymmetricKey>,
    pub authenticated_ephemeral_client: InitGatewayClient,
}

/// Result of fully initialising a client:
/// - details of the associated gateway
/// - the keys the client uses with that gateway
/// - an optional authenticated handle of an ephemeral gateway handle created for the purposes of registration,
///   if this was the first time this client registered
pub struct InitialisationResult {
    pub gateway_registration: GatewayRegistration,
    pub gateway_keys: GatewayClientKeys,
    pub authenticated_ephemeral_client: Option<InitGatewayClient>,
}

impl InitialisationResult {
    /// Resolves the keys from the registration's gateway and generation.
    pub fn new_loaded(gateway_registration: GatewayRegistration, client_keys: &ClientKeys) -> Self {
        let details = &gateway_registration.details;
        let gateway_keys = client_keys.gateway_keys(details.gateway_id(), details.generation_id());
        InitialisationResult {
            gateway_registration,
            gateway_keys,
            authenticated_ephemeral_client: None,
        }
    }

    pub async fn try_load<K, D>(key_store: &K, details_store: &D) -> Result<Self, ClientCoreError>
    where
        K: KeyStore,
        D: GatewaysDetailsStore,
    {
        use_loaded_gateway_details(key_store, details_store, None).await
    }

    pub fn client_address(&self) -> Recipient {
        Recipient::new(
            *self.gateway_keys.identity_keypair().public_key(),
            *self.gateway_keys.encryption_keypair().public_key(),
            // TODO: below only works under assumption that gateway address == gateway id
            // (which currently is true)
            self.gateway_id(),
        )
    }

    pub fn gateway_id(&self) -> ed25519::PublicKey {
        self.gateway_registration.details.gateway_id()
    }

    // indicates if the remote gateway details TTL has expired
    pub fn exipred_details(&self) -> bool {
        if let Some(expiration_timestamp) = self.gateway_registration.details.details_exipration() {
            OffsetDateTime::now_utc() > expiration_timestamp
        } else {
            false
        }
    }
}

#[derive(Clone, Debug)]
pub enum GatewaySelectionSpecification {
    /// Uniformly choose a random remote gateway.
    UniformRemote {
        must_use_tls: bool,
        no_hostname: bool,
    },

    /// Should the new, remote, gateway be selected based on latency.
    RemoteByLatency {
        must_use_tls: bool,
        no_hostname: bool,
    },

    /// Gateway with this specific identity should be chosen.
    // JS: I don't really like the name of this enum variant but couldn't think of anything better at the time
    Specified {
        must_use_tls: bool,
        no_hostname: bool,
        identity: IdentityKey,
    },

    // TODO: this doesn't really fit in here..., but where else to put it?
    /// This client has handled the selection by itself
    Custom {
        gateway_identity: String,
        additional_data: Option<Vec<u8>>,
    },
}

impl Default for GatewaySelectionSpecification {
    fn default() -> Self {
        GatewaySelectionSpecification::UniformRemote {
            must_use_tls: false,
            no_hostname: false,
        }
    }
}

impl GatewaySelectionSpecification {
    pub fn new(
        gateway_identity: Option<String>,
        latency_based_selection: Option<bool>,
        must_use_tls: bool,
        no_hostname: bool,
    ) -> Self {
        if let Some(identity) = gateway_identity {
            GatewaySelectionSpecification::Specified {
                identity,
                must_use_tls,
                no_hostname,
            }
        } else if let Some(true) = latency_based_selection {
            GatewaySelectionSpecification::RemoteByLatency {
                must_use_tls,
                no_hostname,
            }
        } else {
            GatewaySelectionSpecification::UniformRemote {
                must_use_tls,
                no_hostname,
            }
        }
    }
}

pub enum GatewaySetup {
    /// The gateway specification (details + keys) MUST BE loaded from the underlying storage.
    MustLoad {
        /// Optionally specify concrete gateway id. If none is selected, the current active gateway will be used.
        gateway_id: Option<String>,
    },

    /// Specifies usage of a new gateway
    New {
        specification: GatewaySelectionSpecification,

        // TODO: seems to be a bit inefficient to pass them by value
        available_gateways: Vec<RoutingNode>,

        /// Callback useful for allowing initial connection to gateway
        #[cfg(unix)]
        connection_fd_callback: Option<Arc<dyn Fn(RawFd) + Send + Sync>>,
    },

    ReuseConnection {
        /// The authenticated ephemeral client that was created during `init`
        authenticated_ephemeral_client: Box<InitGatewayClient>,

        // Details of this pre-initialised client (i.e. gateway and keys)
        gateway_details: Box<GatewayRegistration>,

        gateway_keys: GatewayClientKeys,
    },
}

impl Debug for GatewaySetup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GatewaySetup::MustLoad { gateway_id } => f
                .debug_struct("GatewaySetup::MustLoad")
                .field("gateway_id", gateway_id)
                .finish(),
            GatewaySetup::New {
                specification,
                available_gateways,
                #[cfg(unix)]
                    connection_fd_callback: _,
            } => f
                .debug_struct("GatewaySetup::New")
                .field("specification", specification)
                .field("available_gateways", available_gateways)
                .field("gateways", specification)
                .finish(),
            GatewaySetup::ReuseConnection {
                gateway_details, ..
            } => f
                .debug_struct("GatewaySetup::ReuseConnection")
                .field("authenticated_ephemeral_client", &"***")
                .field("gateway_details", gateway_details)
                .field("gateway_keys", &"***")
                .finish(),
        }
    }
}

impl GatewaySetup {
    pub fn try_reuse_connection(init_res: InitialisationResult) -> Result<Self, ClientCoreError> {
        if let Some(authenticated_ephemeral_client) = init_res.authenticated_ephemeral_client {
            Ok(GatewaySetup::ReuseConnection {
                authenticated_ephemeral_client: Box::new(authenticated_ephemeral_client),
                gateway_details: Box::new(init_res.gateway_registration),
                gateway_keys: init_res.gateway_keys,
            })
        } else {
            Err(ClientCoreError::NoInitClientPresent)
        }
    }

    /// new gateway setup performed by each client that's inbuilt in a gateway (like NR or IPR)
    pub fn new_inbuilt(identity: ed25519::PublicKey) -> Self {
        GatewaySetup::New {
            specification: GatewaySelectionSpecification::Custom {
                gateway_identity: identity.to_base58_string(),
                additional_data: None,
            },
            available_gateways: vec![],
            #[cfg(unix)]
            connection_fd_callback: None,
        }
    }

    pub async fn try_setup<K, D>(
        self,
        key_store: &K,
        details_store: &D,
    ) -> Result<InitialisationResult, ClientCoreError>
    where
        K: KeyStore,
        D: GatewaysDetailsStore,
    {
        setup_gateway(self, key_store, details_store).await
    }

    pub fn is_must_load(&self) -> bool {
        matches!(self, GatewaySetup::MustLoad { .. })
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
    pub fallback_listener: Option<String>,
    pub gateway_registration: OffsetDateTime,
    pub address: Recipient,
}

impl InitResults {
    pub fn new(
        config: &Config,
        address: Recipient,
        gateway: &RemoteGatewayDetails,
        registration: OffsetDateTime,
    ) -> Self {
        Self {
            version: config.client.version.clone(),
            id: config.client.id.clone(),
            identity_key: address.identity().to_base58_string(),
            encryption_key: address.encryption_key().to_base58_string(),
            gateway_id: gateway.gateway_id.to_base58_string(),
            gateway_listener: gateway.published_data.listeners.primary.to_string(),
            fallback_listener: gateway
                .published_data
                .listeners
                .fallback
                .as_ref()
                .map(|uri| uri.to_string()),
            gateway_registration: registration,
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
        writeln!(f, "Gateway: {}", self.gateway_listener)?;
        write!(f, "Registered at: {}", self.gateway_registration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nym_client_core_gateways_storage::{GatewayDetails, GatewayPublishedData, GenerationId};
    use nym_test_utils::helpers::{deterministic_rng, dummy_ed25519_keypair};

    fn gateway_id(seed: u64) -> ed25519::PublicKey {
        *dummy_ed25519_keypair(seed).public_key()
    }

    fn remote_registration(
        gateway_id: ed25519::PublicKey,
        generation_id: Option<GenerationId>,
    ) -> GatewayRegistration {
        let listeners = GatewayListeners {
            primary: "ws://127.0.0.1:9000".parse().unwrap(),
            fallback: None,
        };
        GatewayDetails::Remote(RemoteGatewayDetails {
            gateway_id,
            shared_key: Arc::new(SharedSymmetricKey::try_from_bytes(&[1; 32]).unwrap()),
            published_data: GatewayPublishedData::new(listeners),
            generation_id,
        })
        .into()
    }

    #[test]
    fn a_legacy_registration_resolves_to_the_root_keys() {
        let root = ClientKeys::generate_new(&mut deterministic_rng());

        let loaded =
            InitialisationResult::new_loaded(remote_registration(gateway_id(1), None), &root);

        assert_eq!(
            loaded.gateway_keys.identity_keypair().public_key(),
            root.identity_keypair().public_key()
        );
        assert_eq!(
            loaded.gateway_keys.encryption_keypair().public_key(),
            root.encryption_keypair().public_key()
        );
    }

    #[test]
    fn a_custom_registration_resolves_to_the_root_keys() {
        let root = ClientKeys::generate_new(&mut deterministic_rng());
        let registration = GatewayDetails::new_custom(gateway_id(1), None).into();

        let loaded = InitialisationResult::new_loaded(registration, &root);

        assert_eq!(
            loaded.gateway_keys.identity_keypair().public_key(),
            root.identity_keypair().public_key()
        );
        assert_eq!(
            loaded.gateway_keys.encryption_keypair().public_key(),
            root.encryption_keypair().public_key()
        );
    }

    #[test]
    fn a_derived_registration_resolves_to_the_keys_for_its_gateway_and_generation() {
        let mut rng = deterministic_rng();
        let root = ClientKeys::generate_new(&mut rng);
        let generation_id = GenerationId::new_random(&mut rng);

        let loaded = InitialisationResult::new_loaded(
            remote_registration(gateway_id(1), Some(generation_id)),
            &root,
        );
        let expected = root.gateway_keys(gateway_id(1), Some(generation_id));

        assert_eq!(
            loaded.gateway_keys.identity_keypair().public_key(),
            expected.identity_keypair().public_key()
        );
        assert_eq!(
            loaded.gateway_keys.encryption_keypair().public_key(),
            expected.encryption_keypair().public_key()
        );
        assert_ne!(
            loaded.gateway_keys.identity_keypair().public_key(),
            root.identity_keypair().public_key()
        );
        assert_ne!(
            loaded.gateway_keys.encryption_keypair().public_key(),
            root.encryption_keypair().public_key()
        );
    }

    #[test]
    fn two_gateways_resolve_to_different_keys() {
        let mut rng = deterministic_rng();
        let root = ClientKeys::generate_new(&mut rng);
        let generation_id = GenerationId::new_random(&mut rng);

        let first = InitialisationResult::new_loaded(
            remote_registration(gateway_id(1), Some(generation_id)),
            &root,
        );
        let second = InitialisationResult::new_loaded(
            remote_registration(gateway_id(2), Some(generation_id)),
            &root,
        );

        assert_ne!(
            first.gateway_keys.identity_keypair().public_key(),
            second.gateway_keys.identity_keypair().public_key()
        );
        assert_ne!(
            first.gateway_keys.encryption_keypair().public_key(),
            second.gateway_keys.encryption_keypair().public_key()
        );
    }
}
