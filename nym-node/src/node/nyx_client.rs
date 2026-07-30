// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config;
use crate::error::NymNodeError;
use nym_config::defaults::NymNetworkDetails;
use nym_validator_client::nyxd::AccountId;
use nym_validator_client::{DirectSigningHttpRpcNyxdClient, QueryHttpRpcNyxdClient, nyxd};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct QueryNyxClientWithAddress {
    client: QueryHttpRpcNyxdClient,
    address: AccountId,
}

#[derive(Clone)]
pub struct NyxClient {
    inner: Arc<RwLock<DirectSigningHttpRpcNyxdClient>>,
}

impl NyxClient {
    pub(crate) fn new(
        config: &config::Nyx,
        network: &NymNetworkDetails,
        mnemonic: &bip39::Mnemonic,
    ) -> Result<NyxClient, NymNodeError> {
        let endpoint = config
            .nyxd_urls
            .choose(&mut thread_rng())
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

    pub(crate) async fn clone_query_client(&self) -> QueryNyxClientWithAddress {
        let inner_guard = self.inner.read().await;
        let client = inner_guard.clone_query_client();
        let address = inner_guard.address();
        QueryNyxClientWithAddress { client, address }
    }

    pub(crate) async fn address(&self) -> AccountId {
        self.inner.read().await.address()
    }
}
