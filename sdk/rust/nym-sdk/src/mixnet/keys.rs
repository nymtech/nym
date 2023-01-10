use std::sync::Arc;

use client_core::client::key_manager::KeyManager;
use crypto::asymmetric::{encryption, identity};
use gateway_requests::registration::handshake::SharedKeys;
use nymsphinx::acknowledgements::AckKey;

pub struct Keys {
    pub identity_keypair: identity::KeyPair,
    pub encryption_keypair: encryption::KeyPair,
    pub ack_key: AckKey,
    pub gateway_shared_key: SharedKeys,
}

pub struct KeysArc {
    pub identity_keypair: Arc<identity::KeyPair>,
    pub encryption_keypair: Arc<encryption::KeyPair>,
    pub ack_key: Arc<AckKey>,
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
