// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_client_core::client::base_client::storage::MixnetClientStorage;
use nym_client_core::client::replies::reply_storage;
use nym_credential_storage::ephemeral_storage::EphemeralStorage as EphemeralCredentialStorage;
use nym_socks5_client_core::config::Config as Socks5Config;

#[cfg(target_os = "android")]
use nym_client_core::client::key_manager::persistence::InMemEphemeralKeys;

#[cfg(not(target_os = "android"))]
use nym_client_core::client::key_manager::persistence::OnDiskKeys;
#[cfg(not(target_os = "android"))]
use nym_client_core::config::persistence::key_pathfinder::ClientKeyPathfinder;

pub struct MobileClientStorage {
    #[cfg(not(target_os = "android"))]
    key_store: OnDiskKeys,

    #[cfg(target_os = "android")]
    key_store: InMemEphemeralKeys,

    reply_store: reply_storage::Empty,
    credential_store: EphemeralCredentialStorage,
}

impl MixnetClientStorage for MobileClientStorage {
    #[cfg(not(target_os = "android"))]
    type KeyStore = OnDiskKeys;

    #[cfg(target_os = "android")]
    type KeyStore = InMemEphemeralKeys;

    type ReplyStore = reply_storage::Empty;
    type CredentialStore = EphemeralCredentialStorage;

    fn into_split(self) -> (Self::KeyStore, Self::ReplyStore, Self::CredentialStore) {
        (self.key_store, self.reply_store, self.credential_store)
    }

    fn key_store(&self) -> &Self::KeyStore {
        &self.key_store
    }

    fn reply_store(&self) -> &Self::ReplyStore {
        &self.reply_store
    }

    fn credential_store(&self) -> &Self::CredentialStore {
        &self.credential_store
    }
}

impl MobileClientStorage {
    pub fn new(config: &Socks5Config) -> Self {
        #[cfg(target_os = "android")]
        let key_store = InMemEphemeralKeys;

        #[cfg(not(target_os = "android"))]
        let key_store = {
            let pathfinder = ClientKeyPathfinder::new_from_config(config.get_base());
            OnDiskKeys::new(pathfinder)
        };

        MobileClientStorage {
            key_store,
            reply_store: Default::default(),
            credential_store: Default::default(),
        }
    }
}
