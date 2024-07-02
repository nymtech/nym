// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::Config;
use crate::error::NymRewarderError;
use crate::rewarder::credential_issuance::types::{addr_to_account_id, CredentialIssuer};
use nym_coconut_dkg_common::types::Epoch;
use nym_compact_ecash::{Base58, VerificationKeyAuth};
use nym_crypto::asymmetric::ed25519;
use nym_network_defaults::NymNetworkDetails;
use nym_validator_client::nyxd::contract_traits::ecash_query_client::{Deposit, DepositId};
use nym_validator_client::nyxd::contract_traits::{
    DkgQueryClient, EcashQueryClient, PagedDkgQueryClient,
};
use nym_validator_client::nyxd::module_traits::staking::{
    QueryHistoricalInfoResponse, QueryValidatorsResponse,
};
use nym_validator_client::nyxd::{
    AccountId, Coin, CosmWasmClient, Hash, PageRequest, StakingQueryClient,
};
use nym_validator_client::{nyxd, DirectSigningHttpRpcNyxdClient};
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct NyxdClient {
    inner: Arc<RwLock<DirectSigningHttpRpcNyxdClient>>,
}

impl NyxdClient {
    pub(crate) fn new(config: &Config) -> Result<Self, NymRewarderError> {
        let client_config =
            nyxd::Config::try_from_nym_network_details(&NymNetworkDetails::new_from_env())?;
        let nyxd_url = config.base.upstream_nyxd.as_str();

        let mnemonic = config.base.mnemonic.clone();

        let inner = DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
            client_config,
            nyxd_url,
            mnemonic,
        )?;

        Ok(NyxdClient {
            inner: Arc::new(RwLock::new(inner)),
        })
    }

    pub(crate) async fn balance(&self, denom: &str) -> Result<Coin, NymRewarderError> {
        let guard = self.inner.read().await;
        let address = guard.address();
        Ok(guard
            .get_balance(&address, denom.to_string())
            .await?
            .unwrap_or(Coin::new(0, denom)))
    }

    pub(crate) async fn send_rewards(
        &self,
        epoch: crate::rewarder::Epoch,
        amounts: Vec<(AccountId, Vec<Coin>)>,
    ) -> Result<Hash, NymRewarderError> {
        self.inner
            .write()
            .await
            .send_multiple(amounts, format!("sending rewards for {epoch:?}"), None)
            .await
            .map(|res| res.hash)
            .map_err(Into::into)
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
        let guard = self.inner.read().await;
        Ok(StakingQueryClient::validators(guard.deref(), "".to_string(), pagination).await?)
    }

    pub(crate) async fn dkg_epoch(&self) -> Result<Epoch, NymRewarderError> {
        Ok(self.inner.read().await.get_current_epoch().await?)
    }

    pub(crate) async fn get_credential_issuers(
        &self,
        dkg_epoch: u64,
    ) -> Result<Vec<CredentialIssuer>, NymRewarderError> {
        let guard = self.inner.read().await;
        let mut dealers_map = HashMap::new();
        let dealers = guard.get_all_current_dealers().await?;
        for dealer in dealers {
            dealers_map.insert(dealer.address.to_string(), dealer);
        }
        let vk_shares = guard.get_all_verification_key_shares(dkg_epoch).await?;

        let mut issuers = Vec::with_capacity(vk_shares.len());
        for share in vk_shares {
            if let Some(info) = dealers_map.remove(&share.owner.to_string()) {
                issuers.push(CredentialIssuer {
                    public_key: ed25519::PublicKey::from_base58_string(&info.ed25519_identity)?,
                    operator_account: addr_to_account_id(share.owner),
                    api_runner: share.announce_address,
                    verification_key: VerificationKeyAuth::try_from_bs58(share.share).map_err(
                        |source| NymRewarderError::MalformedPartialVerificationKey {
                            runner: info.address.to_string(),
                            source,
                        },
                    )?,
                })
            }
        }

        Ok(issuers)
    }

    pub(crate) async fn get_deposit_details(
        &self,
        deposit_id: DepositId,
    ) -> Result<Deposit, NymRewarderError> {
        let res = self.inner.read().await.get_deposit(deposit_id).await?;
        res.deposit
            .ok_or(NymRewarderError::DepositNotFound { deposit_id })
    }
}
