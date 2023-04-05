use std::sync::Arc;

use nym_client_core::client::key_manager::KeyManager;
use nym_gateway_requests::registration::handshake::SharedKeys;
use nym_crypto::asymmetric::{encryption, identity};
use nym_sphinx::acknowledgements::AckKey;

/// The set of keys used by the client. Identity, encryption and ack keys are generated at creating
/// unless specified to loaded from storage or somehow explictly specified. The gateway shared key
/// is generated when registering with a gateway.
pub struct Keys {
    /// The identity key of the client.
    pub identity_keypair: identity::KeyPair,
    /// The encryption key of the client.
    pub encryption_keypair: encryption::KeyPair,
    /// The ack key used by the client.
    pub ack_key: AckKey,

    /// The gateway shared key that is obtained after registering with a gateway.
    pub gateway_shared_key: SharedKeys,
}

/// The set of keys used by the client, but where each key is stored in an [`std::sync::Arc`] for
/// easy cloning.
pub struct KeysArc {
    /// The identity key of the client.
    pub identity_keypair: Arc<identity::KeyPair>,
    /// The encryption key of the client.
    pub encryption_keypair: Arc<encryption::KeyPair>,
    /// The ack key used by the client.
    pub ack_key: Arc<AckKey>,

    /// The gateway shared key that is obtained after registering with a gateway.
    pub gateway_shared_key: Arc<SharedKeys>,
}

impl From<Keys> for KeyManager {
    fn from(keys: Keys) -> Self {
        KeyManager::from_keys(
            keys.identity_keypair,
            keys.encryption_keypair,
            keys.gateway_shared_key,
            keys.ack_key,
        )
    }
}

impl From<Keys> for KeysArc {
    fn from(keys: Keys) -> Self {
        KeysArc {
            identity_keypair: keys.identity_keypair.into(),
            encryption_keypair: keys.encryption_keypair.into(),
            ack_key: keys.ack_key.into(),
            gateway_shared_key: keys.gateway_shared_key.into(),
        }
    }
}

impl From<&KeyManager> for KeysArc {
    fn from(key_manager: &KeyManager) -> Self {
        KeysArc {
            identity_keypair: key_manager.identity_keypair(),
            encryption_keypair: key_manager.encryption_keypair(),
            ack_key: key_manager.ack_key(),
            gateway_shared_key: key_manager.gateway_shared_key(),
        }
    }
}
