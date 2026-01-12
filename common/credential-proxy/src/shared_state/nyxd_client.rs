// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::CredentialProxyError;
use crate::helpers::LockTimer;
use nym_ecash_contract_common::msg::ExecuteMsg;
use nym_validator_client::nyxd::contract_traits::NymContractsProvider;
use nym_validator_client::nyxd::cosmwasm_client::types::ExecuteResult;
use nym_validator_client::nyxd::{Coin, Config, CosmWasmClient, NyxdClient};
use nym_validator_client::{DirectSigningHttpRpcNyxdClient, nyxd};
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::{instrument, warn};

#[derive(Clone)]
pub struct ChainClient(Arc<RwLock<DirectSigningHttpRpcNyxdClient>>);

impl ChainClient {
    pub fn new(mnemonic: bip39::Mnemonic) -> Result<Self, CredentialProxyError> {
        let network_details = nym_network_defaults::NymNetworkDetails::new_from_env();
        let client_config = nyxd::Config::try_from_nym_network_details(&network_details)?;

        let nyxd_url = network_details
            .endpoints
            .first()
            .ok_or_else(|| CredentialProxyError::NoNyxEndpointsAvailable)?
            .nyxd_url
            .as_str();

        Self::new_with_config(client_config, nyxd_url, mnemonic)
    }

    pub fn new_with_config(
        client_config: Config,
        nyxd_url: &str,
        mnemonic: bip39::Mnemonic,
    ) -> Result<Self, CredentialProxyError> {
        let client = NyxdClient::connect_with_mnemonic(client_config, nyxd_url, mnemonic)?;

        if client.ecash_contract_address().is_none() {
            return Err(CredentialProxyError::UnavailableEcashContract);
        }

        if client.dkg_contract_address().is_none() {
            return Err(CredentialProxyError::UnavailableDKGContract);
        }

        Ok(ChainClient(Arc::new(RwLock::new(client))))
    }

    pub async fn query_chain(&self) -> ChainReadPermit<'_> {
        let _acquire_timer = LockTimer::new("acquire chain query permit");
        self.0.read().await
    }

    pub async fn start_chain_tx(&self) -> ChainWritePermit<'_> {
        let _acquire_timer = LockTimer::new("acquire exclusive chain write permit");

        ChainWritePermit {
            lock_timer: LockTimer::new("exclusive chain access permit"),
            inner: self.0.write().await,
        }
    }
}

pub type ChainReadPermit<'a> = RwLockReadGuard<'a, DirectSigningHttpRpcNyxdClient>;

// explicitly wrap the WriteGuard for extra information regarding time taken
pub struct ChainWritePermit<'a> {
    // it's not really dead, we only care about it being dropped
    #[allow(dead_code)]
    lock_timer: LockTimer,
    inner: RwLockWriteGuard<'a, DirectSigningHttpRpcNyxdClient>,
}

impl ChainWritePermit<'_> {
    #[instrument(skip(self, memo, info), err(Display))]
    pub async fn make_deposits(
        self,
        memo: String,
        info: Vec<(String, Coin)>,
    ) -> Result<ExecuteResult, CredentialProxyError> {
        let address = self.inner.address();
        let starting_sequence = self.inner.get_sequence(&address).await?.sequence;

        let ecash_contract = self
            .inner
            .ecash_contract_address()
            .ok_or(CredentialProxyError::UnavailableEcashContract)?;
        let deposit_messages = info
            .into_iter()
            .map(|(identity_key, amount)| {
                (
                    ExecuteMsg::DepositTicketBookFunds { identity_key },
                    vec![amount],
                )
            })
            .collect::<Vec<_>>();

        let res = self
            .inner
            .execute_multiple(ecash_contract, deposit_messages, None, memo)
            .await?;

        loop {
            let updated_sequence = self.inner.get_sequence(&address).await?.sequence;

            if updated_sequence > starting_sequence {
                break;
            }
            warn!("wrong sequence number... waiting before releasing chain lock");
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        Ok(res)
    }
}

impl Deref for ChainWritePermit<'_> {
    type Target = DirectSigningHttpRpcNyxdClient;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}
