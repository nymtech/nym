// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use nym_client_core::client::base_client::storage::{InMemGatewaysDetails, MixnetClientStorage};
use nym_client_core::client::key_manager::persistence::InMemEphemeralKeys;
use nym_client_core::client::replies::reply_storage;
use nym_credential_storage::ephemeral_storage::EphemeralStorage as EphemeralCredentialStorage;

pub struct MobileClientStorage {
    // the key storage is now useless without gateway details store. so use ephemeral for everything.
    key_store: InMemEphemeralKeys,
    gateway_details_store: InMemGatewaysDetails,

    reply_store: reply_storage::Empty,
    credential_store: EphemeralCredentialStorage,
}

impl MixnetClientStorage for MobileClientStorage {
    type KeyStore = InMemEphemeralKeys;
    type ReplyStore = reply_storage::Empty;
    type CredentialStore = EphemeralCredentialStorage;
    type GatewaysDetailsStore = InMemGatewaysDetails;

    fn into_runtime_stores(self) -> (Self::ReplyStore, Self::CredentialStore) {
        (self.reply_store, self.credential_store)
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

    fn gateway_details_store(&self) -> &Self::GatewaysDetailsStore {
        &self.gateway_details_store
    }
}

impl MobileClientStorage {
    pub fn new(_config: &Config) -> Self {
        MobileClientStorage {
            key_store: Default::default(),
            gateway_details_store: Default::default(),
            reply_store: Default::default(),
            credential_store: Default::default(),
        }
    }
}
