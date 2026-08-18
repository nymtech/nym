// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use async_trait::async_trait;
use nym_geolocation_contract_common::{
    ExecuteMsg as GeolocationExecuteMsg, QueryMsg as GeolocationQueryMsg,
};
use nym_network_defaults::NymNetworkDetails;
use nym_validator_client::DirectSigningHttpRpcNyxdClient;
use nym_validator_client::nyxd::contract_traits::{
    GeolocationQueryClient, GeolocationSigningClient, MixnetQueryClient,
};
use nym_validator_client::nyxd::cosmwasm_client::types::ExecuteResult;
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::nym_mixnet_contract_common::QueryMsg as MixnetQueryMsg;
use nym_validator_client::nyxd::{AccountId, Coin, Fee, bip39};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use url::Url;

#[derive(Clone)]
pub struct NyxClient {
    inner: Arc<RwLock<DirectSigningHttpRpcNyxdClient>>,
}

impl NyxClient {
    pub(crate) fn new(
        endpoint: Url,
        network: NymNetworkDetails,
        mnemonic: bip39::Mnemonic,
    ) -> anyhow::Result<NyxClient> {
        let nyxd_client =
            DirectSigningHttpRpcNyxdClient::connect_with_mnemonic_and_network_details(
                endpoint.as_ref(),
                network,
                mnemonic.clone(),
            )?;
        Ok(NyxClient {
            inner: Arc::new(RwLock::new(nyxd_client)),
        })
    }

    pub(crate) async fn address(&self) -> AccountId {
        self.inner.read().await.address()
    }

    pub(crate) async fn write(&self) -> RwLockWriteGuard<'_, DirectSigningHttpRpcNyxdClient> {
        self.inner.write().await
    }

    pub(crate) async fn read(&self) -> RwLockReadGuard<'_, DirectSigningHttpRpcNyxdClient> {
        self.inner.read().await
    }
}

#[async_trait]
impl GeolocationQueryClient for NyxClient {
    async fn query_geolocation_contract<T>(
        &self,
        query: GeolocationQueryMsg,
    ) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        let client = self.read().await;
        client.query_geolocation_contract(query).await
    }
}

#[async_trait]
impl GeolocationSigningClient for NyxClient {
    async fn execute_geolocation_contract(
        &self,
        fee: Option<Fee>,
        msg: GeolocationExecuteMsg,
        memo: String,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError> {
        let client = self.write().await;
        client
            .execute_geolocation_contract(fee, msg, memo, funds)
            .await
    }
}

#[async_trait]
impl MixnetQueryClient for NyxClient {
    async fn query_mixnet_contract<T>(&self, query: MixnetQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        let client = self.read().await;
        client.query_mixnet_contract(query).await
    }
}
