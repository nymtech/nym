// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::error::NymRewarderError;
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::module_traits::staking::{
    QueryHistoricalInfoResponse, QueryValidatorResponse, QueryValidatorsResponse,
};
use nym_validator_client::nyxd::{AccountId, CosmWasmClient, PageRequest, StakingQueryClient};
use nym_validator_client::{nyxd, DirectSigningHttpRpcNyxdClient};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct NyxdClient {
    inner: Arc<RwLock<DirectSigningHttpRpcNyxdClient>>,
}

impl NyxdClient {
    pub(crate) fn new(config: &Config) -> Self {
        let client_config = config.rpc_client_config();
        let nyxd_url = config.base.upstream_nyxd.as_str();

        let mnemonic = config.base.mnemonic.clone();

        let inner = DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
            client_config,
            nyxd_url,
            mnemonic,
        )
        .expect("Failed to connect to nyxd!");

        NyxdClient {
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    pub(crate) async fn address(&self) -> AccountId {
        self.inner.read().await.address()
    }

    pub(crate) async fn historical_info(
        &self,
        height: i64,
    ) -> Result<QueryHistoricalInfoResponse, NymRewarderError> {
        Ok(self.inner.read().await.historical_info(height).await?)
    }

    pub(crate) async fn validators(
        &self,
        pagination: Option<PageRequest>,
    ) -> Result<QueryValidatorsResponse, NymRewarderError> {
        Ok(self
            .inner
            .read()
            .await
            .validators("".to_string(), pagination)
            .await?)
    }
}
