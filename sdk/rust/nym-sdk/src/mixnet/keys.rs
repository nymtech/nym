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
