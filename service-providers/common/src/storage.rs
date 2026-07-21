// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Storage backend for service providers embedded inside a nym-node gateway.
//!
//! The embedded providers (network requester, IP packet router, authenticator) are built as
//! mixnet clients, but they route their outbound traffic through the host gateway directly and
//! never spend ecash for it. They therefore have no use for a persistent credential store.
//!
//! [`EmbeddedProviderStorage`] keeps keys, reply-SURBs and gateway registrations on disk (which the
//! providers genuinely need to persist) while using an in-memory (ephemeral) credential store.

use nym_client_core::client::base_client::non_wasm_helpers;
use nym_client_core::client::base_client::storage::{MixnetClientStorage, OnDiskGatewaysDetails};
use nym_client_core::client::key_manager::persistence::OnDiskKeys;
use nym_client_core::client::replies::reply_storage::fs_backend;
use nym_client_core::config::disk_persistence::CommonClientPaths;
use nym_client_core::config::DebugConfig;
use nym_client_core::error::ClientCoreError;
use nym_credential_storage::ephemeral_storage::EphemeralStorage as EphemeralCredentialStorage;

/// On-disk client storage that uses an ephemeral, in-memory credential store.
///
/// Everything except credentials is persisted, matching what embedded gateway service providers
/// require.
#[derive(Clone)]
pub struct EmbeddedProviderStorage {
    key_store: OnDiskKeys,
    reply_store: fs_backend::Backend,
    credential_store: EphemeralCredentialStorage,
    gateway_details_store: OnDiskGatewaysDetails,
}

impl EmbeddedProviderStorage {
    /// Builds the storage from the provider's client paths.
    ///
    /// The `credentials_database` and `credential_requests_database` entries in `paths` are
    /// intentionally ignored: the credential store is always ephemeral.
    pub async fn from_paths(
        paths: CommonClientPaths,
        debug_config: &DebugConfig,
    ) -> Result<Self, ClientCoreError> {
        let key_store = OnDiskKeys::new(paths.keys);

        let reply_store = non_wasm_helpers::setup_fs_reply_surb_backend(
            paths.reply_surb_database,
            &debug_config.reply_surbs,
        )
        .await?;

        let credential_store = nym_credential_storage::initialise_ephemeral_storage();

        let gateway_details_store =
            non_wasm_helpers::setup_fs_gateways_storage(paths.gateway_registrations).await?;

        Ok(EmbeddedProviderStorage {
            key_store,
            reply_store,
            credential_store,
            gateway_details_store,
        })
    }
}

impl MixnetClientStorage for EmbeddedProviderStorage {
    type KeyStore = OnDiskKeys;
    type ReplyStore = fs_backend::Backend;
    type CredentialStore = EphemeralCredentialStorage;
    type GatewaysDetailsStore = OnDiskGatewaysDetails;

    fn into_runtime_stores(
        self,
    ) -> (
        Self::ReplyStore,
        Self::CredentialStore,
        Self::GatewaysDetailsStore,
    ) {
        (
            self.reply_store,
            self.credential_store,
            self.gateway_details_store,
        )
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
