// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::NymNodeError;
use nym_config::defaults::NymNetworkDetails;
use nym_validator_client::models::KeyRotationDetails;
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::nyxd::contract_traits::{MixnetQueryClient, TypedNymContracts};
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::{DirectSigningHttpRpcNyxdClient, QueryHttpRpcNyxdClient, nyxd};
use rand::rng;
use rand::seq::IndexedRandom;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use url::Url;

#[derive(Clone)]
pub struct NyxClient {
    inner: Arc<RwLock<DirectSigningHttpRpcNyxdClient>>,
}

impl NyxClient {
    pub(crate) fn new(
        nyxd_urls: &[Url],
        network: &NymNetworkDetails,
        mnemonic: &bip39::Mnemonic,
    ) -> Result<NyxClient, NymNodeError> {
        let endpoint = nyxd_urls
            .choose(&mut rng())
            .ok_or(NymNodeError::NoNyxEndpoints)?;

        let client_config = nyxd::Config::try_from_nym_network_details(network)?;

        let nyxd_client = DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
            client_config,
            endpoint.as_ref(),
            mnemonic.clone(),
        )?;
        Ok(NyxClient {
            inner: Arc::new(RwLock::new(nyxd_client)),
        })
    }

    #[allow(dead_code)]
    pub(crate) async fn clone_query_client(&self) -> QueryHttpRpcNyxdClient {
        self.inner.read().await.clone_query_client()
    }

    pub(crate) async fn write(&self) -> RwLockWriteGuard<'_, DirectSigningHttpRpcNyxdClient> {
        self.inner.write().await
    }

    pub(crate) async fn read(&self) -> RwLockReadGuard<'_, DirectSigningHttpRpcNyxdClient> {
        self.inner.read().await
    }

    pub(crate) async fn address(&self) -> AccountId {
        self.inner.read().await.address()
    }

    pub(crate) async fn get_nym_contracts(&self) -> TypedNymContracts {
        self.inner.read().await.get_nym_contracts()
    }

    /// Reads the current interval and key rotation state from the mixnet contract.
    pub(crate) async fn get_key_rotation_details(&self) -> Result<KeyRotationDetails, NyxdError> {
        let client = self.inner.read().await;
        let current_interval = client.get_current_interval_details().await?.interval;
        let key_rotation_state = client.get_key_rotation_state().await?;
        Ok(KeyRotationDetails::new(
            key_rotation_state,
            &current_interval,
        ))
    }
}
