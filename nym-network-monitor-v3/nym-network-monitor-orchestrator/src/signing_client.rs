// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use async_trait::async_trait;
use nym_validator_client::nyxd::bip39;
use nym_validator_client::nyxd::contract_traits::performance_query_client::PerformanceQueryMsg;
use nym_validator_client::nyxd::contract_traits::{
    PerformanceQueryClient, PerformanceSigningClient,
};
use nym_validator_client::nyxd::cosmwasm_client::types::ExecuteResult;
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::nym_performance_contract_common::ExecuteMsg as PerformanceExecuteMsg;
use nym_validator_client::nyxd::{Coin, Fee};
use nym_validator_client::{DirectSigningHttpRpcValidatorClient, ValidatorClientError, client};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError};

/// The orchestrator's one signing client, shared by every task that transacts from its account.
///
/// A transaction takes the write lock until it is included, so two never race for the account
/// sequence; the contract traits implemented here take the lock for that one call.
#[derive(Clone)]
pub(crate) struct SigningClient(Arc<RwLock<DirectSigningHttpRpcValidatorClient>>);

impl SigningClient {
    pub(crate) fn new(
        config: client::Config,
        mnemonic: bip39::Mnemonic,
    ) -> Result<Self, ValidatorClientError> {
        let client = DirectSigningHttpRpcValidatorClient::new_signing(config, mnemonic)?;
        Ok(SigningClient(Arc::new(RwLock::new(client))))
    }

    pub(crate) async fn read(&self) -> RwLockReadGuard<'_, DirectSigningHttpRpcValidatorClient> {
        self.0.read().await
    }

    pub(crate) async fn write(&self) -> RwLockWriteGuard<'_, DirectSigningHttpRpcValidatorClient> {
        self.0.write().await
    }

    pub(crate) fn try_read(
        &self,
    ) -> Result<RwLockReadGuard<'_, DirectSigningHttpRpcValidatorClient>, TryLockError> {
        self.0.try_read()
    }
}

#[async_trait]
impl PerformanceQueryClient for SigningClient {
    async fn query_performance_contract<T>(
        &self,
        query: PerformanceQueryMsg,
    ) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        self.read()
            .await
            .nyxd
            .query_performance_contract(query)
            .await
    }
}

#[async_trait]
impl PerformanceSigningClient for SigningClient {
    async fn execute_performance_contract(
        &self,
        fee: Option<Fee>,
        msg: PerformanceExecuteMsg,
        memo: String,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError> {
        self.write()
            .await
            .nyxd
            .execute_performance_contract(fee, msg, memo, funds)
            .await
    }
}
