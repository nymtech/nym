// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::rewarded_set_updater::error::RewardingError;
use config::defaults::{NymNetworkDetails, DEFAULT_VALIDATOR_API_PORT};
use mixnet_contract_common::mixnode::MixNodeDetails;
use mixnet_contract_common::reward_params::RewardingParams;
use mixnet_contract_common::{
    CurrentIntervalResponse, ExecuteMsg, GatewayBond, IdentityKey, Interval, MixNodeBond, NodeId,
    RewardedSetNodeStatus,
};
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;
use validator_client::nymd::traits::{MixnetQueryClient, MixnetSigningClient};
use validator_client::nymd::{
    hash::{Hash, SHA256_HASH_SIZE},
    Coin, CosmWasmClient, Fee, QueryNymdClient, SigningCosmWasmClient, SigningNymdClient,
    TendermintTime,
};
use validator_client::ValidatorClientError;

#[cfg(feature = "coconut")]
use crate::coconut::error::CoconutError;
#[cfg(feature = "coconut")]
use async_trait::async_trait;
#[cfg(feature = "coconut")]
use coconut_bandwidth_contract_common::spend_credential::SpendCredentialResponse;
#[cfg(feature = "coconut")]
use multisig_contract_common::msg::ProposalResponse;
#[cfg(feature = "coconut")]
use validator_client::nymd::{
    traits::{CoconutBandwidthQueryClient, MultisigQueryClient, MultisigSigningClient},
    AccountId,
};

pub(crate) struct Client<C>(pub(crate) Arc<RwLock<validator_client::Client<C>>>);

impl<C> Clone for Client<C> {
    fn clone(&self) -> Self {
        Client(Arc::clone(&self.0))
    }
}

impl Client<QueryNymdClient> {
    pub(crate) fn new_query(config: &Config) -> Self {
        // the api address is irrelevant here as **WE ARE THE API**
        // and we won't be talking on the socket here.
        let api_url = format!("http://localhost:{}", DEFAULT_VALIDATOR_API_PORT)
            .parse()
            .unwrap();
        let nymd_url = config.get_nymd_validator_url();

        let details = NymNetworkDetails::new_from_env()
            .with_mixnet_contract(Some(config.get_mixnet_contract_address()));

        let client_config = validator_client::Config::try_from_nym_network_details(&details)
            .expect("failed to construct valid validator client config with the provided network")
            .with_urls(nymd_url, api_url);

        let inner =
            validator_client::Client::new_query(client_config).expect("Failed to connect to nymd!");

        Client(Arc::new(RwLock::new(inner)))
    }
}

impl Client<SigningNymdClient> {
    pub(crate) fn new_signing(config: &Config) -> Self {
        // the api address is irrelevant here as **WE ARE THE API**
        // and we won't be talking on the socket here.
        let api_url = format!("http://localhost:{}", DEFAULT_VALIDATOR_API_PORT)
            .parse()
            .unwrap();
        let nymd_url = config.get_nymd_validator_url();

        let details = NymNetworkDetails::new_from_env()
            .with_mixnet_contract(Some(config.get_mixnet_contract_address()));

        let client_config = validator_client::Config::try_from_nym_network_details(&details)
            .expect("failed to construct valid validator client config with the provided network")
            .with_urls(nymd_url, api_url);

        let mnemonic = config
            .get_mnemonic()
            .parse()
            .expect("the mnemonic is invalid!");

        let inner = validator_client::Client::new_signing(client_config, mnemonic)
            .expect("Failed to connect to nymd!");

        Client(Arc::new(RwLock::new(inner)))
    }
}

impl<C> Client<C> {
    // a helper function for the future to obtain the current block timestamp
    #[allow(dead_code)]
    pub(crate) async fn current_block_timestamp(
        &self,
    ) -> Result<TendermintTime, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        let time = self
            .0
            .read()
            .await
            .nymd
            .get_current_block_timestamp()
            .await?;

        Ok(time)
    }

    /// Obtains the hash of a block specified by the provided height.
    /// If the resulting digest is empty, a `None` is returned instead.
    ///
    /// # Arguments
    ///
    /// * `height`: height of the block for which we want to obtain the hash.
    #[allow(dead_code)]
    pub(crate) async fn get_block_hash(
        &self,
        height: u32,
    ) -> Result<Option<[u8; SHA256_HASH_SIZE]>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        let hash = match self.0.read().await.nymd.get_block_hash(height).await? {
            Hash::Sha256(hash) => Some(hash),
            Hash::None => None,
        };

        Ok(hash)
    }

    pub(crate) async fn get_mixnodes(&self) -> Result<Vec<MixNodeDetails>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        self.0.read().await.get_all_nymd_mixnodes_detailed().await
    }

    pub(crate) async fn get_gateways(&self) -> Result<Vec<GatewayBond>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        self.0.read().await.get_all_nymd_gateways().await
    }

    pub(crate) async fn get_current_interval(
        &self,
    ) -> Result<CurrentIntervalResponse, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        Ok(self.0.read().await.get_current_interval_details().await?)
    }

    pub(crate) async fn get_current_rewarding_parameters(
        &self,
    ) -> Result<RewardingParams, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        Ok(self.0.read().await.get_rewarding_parameters().await?)
    }

    pub(crate) async fn get_rewarded_set_mixnodes(
        &self,
    ) -> Result<Vec<(NodeId, RewardedSetNodeStatus)>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync + Send,
    {
        self.0
            .read()
            .await
            .get_all_nymd_rewarded_set_mixnodes()
            .await
    }

    pub(crate) async fn advance_current_epoch(&self) -> Result<(), ValidatorClientError>
    where
        C: SigningCosmWasmClient + Sync + Send,
    {
        todo!()
        // self.0
        //     .write()
        //     .await
        //     .nymd
        //     .advance_current_epoch(None)
        //     .await?;
        // Ok(())
    }

    pub(crate) async fn reconcile_epoch_events(&self) -> Result<(), ValidatorClientError>
    where
        C: SigningCosmWasmClient + Sync + Send,
    {
        self.0
            .write()
            .await
            .nymd
            .reconcile_epoch_events(None, None)
            .await?;
        Ok(())
    }

    pub(crate) async fn epoch_operations(
        &self,
        rewarded_set: Vec<IdentityKey>,
        expected_active_set_size: u32,
        reward_msgs: Vec<(ExecuteMsg, Vec<Coin>)>,
    ) -> Result<(), RewardingError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        todo!()
        // // // First we create the checkpoint, all subsequent changes to a node will be made to the checkpoint
        // let mut msgs = vec![(ExecuteMsg::CheckpointMixnodes {}, vec![])];
        // msgs.extend(reward_msgs);
        //
        // let epoch_msgs = vec![
        //     (ExecuteMsg::ReconcileDelegations {}, vec![]),
        //     (ExecuteMsg::AdvanceCurrentEpoch {}, vec![]),
        //     (
        //         ExecuteMsg::WriteRewardedSet {
        //             rewarded_set,
        //             expected_active_set_size,
        //         },
        //         vec![],
        //     ),
        // ];
        //
        // msgs.extend_from_slice(&epoch_msgs);
        //
        // let memo = "Performing epoch operations".to_string();
        //
        // self.execute_multiple_with_retry(msgs, None, memo)
        //     .await?;
        // Ok(())
    }

    async fn execute_multiple_with_retry<M>(
        &self,
        msgs: Vec<(M, Vec<Coin>)>,
        fee: Option<Fee>,
        memo: String,
    ) -> Result<(), RewardingError>
    where
        C: SigningCosmWasmClient + Sync,
        M: Serialize + Clone + Send,
    {
        let contract = self.0.read().await.get_mixnet_contract_address();

        // grab the write lock here so we're sure nothing else is executing anything on the contract
        // in the meantime
        // however, we're not 100% guarded against everything
        // for example somebody might have taken the mnemonic used by the validator
        // and sent a transaction manually using the same account. The sequence number
        // would have gotten incremented, yet the rewarding transaction might have actually not
        // been included in the block. sadly we can't do much about that.
        let client_guard = self.0.write().await;
        let pre_sequence = client_guard.nymd.account_sequence().await?;

        let res = client_guard
            .nymd
            .execute_multiple(&contract, msgs.clone(), fee.clone(), memo.clone())
            .await;

        match res {
            Ok(_) => Ok(()),
            Err(err) => {
                if err.is_tendermint_response_timeout() {
                    // wait until we're sure we're into the next block (remember we're holding the lock)
                    sleep(Duration::from_secs(11)).await;
                    let curr_sequence = client_guard.nymd.account_sequence().await?;
                    if curr_sequence.sequence > pre_sequence.sequence {
                        // unless somebody was messing around doing stuff manually in that tiny time interval
                        // we're good. It was a false negative.
                        Ok(())
                    } else {
                        // the sequence number has not increased, meaning the transaction was not executed
                        // so attempt to send it again
                        client_guard
                            .nymd
                            .execute_multiple(&contract, msgs, fee, memo)
                            .await?;
                        Ok(())
                    }
                } else {
                    Err(err.into())
                }
            }
        }
    }
}

#[async_trait]
#[cfg(feature = "coconut")]
impl<C> crate::coconut::client::Client for Client<C>
where
    C: SigningCosmWasmClient + Sync + Send,
{
    async fn address(&self) -> AccountId {
        self.0.read().await.nymd.address().clone()
    }

    async fn get_tx(
        &self,
        tx_hash: &str,
    ) -> crate::coconut::error::Result<validator_client::nymd::TxResponse> {
        let tx_hash = tx_hash
            .parse::<validator_client::nymd::tx::Hash>()
            .map_err(|_| CoconutError::TxHashParseError)?;
        Ok(self.0.read().await.nymd.get_tx(tx_hash).await?)
    }

    async fn get_proposal(
        &self,
        proposal_id: u64,
    ) -> crate::coconut::error::Result<ProposalResponse> {
        Ok(self.0.read().await.nymd.get_proposal(proposal_id).await?)
    }

    async fn get_spent_credential(
        &self,
        blinded_serial_number: String,
    ) -> crate::coconut::error::Result<SpendCredentialResponse> {
        Ok(self
            .0
            .read()
            .await
            .nymd
            .get_spent_credential(blinded_serial_number)
            .await?)
    }

    async fn vote_proposal(
        &self,
        proposal_id: u64,
        vote_yes: bool,
        fee: Option<Fee>,
    ) -> Result<(), CoconutError> {
        self.0
            .read()
            .await
            .nymd
            .vote_proposal(proposal_id, vote_yes, fee)
            .await?;
        Ok(())
    }
}
