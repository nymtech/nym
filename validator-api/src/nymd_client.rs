// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::rewarded_set_updater::error::RewardingError;
#[cfg(feature = "coconut")]
use async_trait::async_trait;
use config::defaults::{DEFAULT_NETWORK, DEFAULT_VALIDATOR_API_PORT};
use mixnet_contract_common::Interval;
use mixnet_contract_common::{
    reward_params::EpochRewardParams, ContractStateParams, Delegation, ExecuteMsg, GatewayBond,
    IdentityKey, MixNodeBond, MixnodeRewardingStatusResponse, RewardedSetNodeStatus,
};
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;
use validator_client::nymd::{
    hash::{Hash, SHA256_HASH_SIZE},
    CosmWasmClient, CosmosCoin, Fee, QueryNymdClient, SigningCosmWasmClient, SigningNymdClient,
    TendermintTime,
};
use validator_client::ValidatorClientError;

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
        let network = DEFAULT_NETWORK;

        let mixnet_contract = config
            .get_mixnet_contract_address()
            .parse()
            .expect("the mixnet contract address is invalid!");

        let client_config = validator_client::Config::new(
            network,
            nymd_url,
            api_url,
            Some(mixnet_contract),
            None,
            None,
        );
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
        let network = DEFAULT_NETWORK;

        let mixnet_contract = config
            .get_mixnet_contract_address()
            .parse()
            .expect("the mixnet contract address is invalid!");
        let mnemonic = config
            .get_mnemonic()
            .parse()
            .expect("the mnemonic is invalid!");

        let client_config = validator_client::Config::new(
            network,
            nymd_url,
            api_url,
            Some(mixnet_contract),
            None,
            None,
        );
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

    pub(crate) async fn get_mixnodes(&self) -> Result<Vec<MixNodeBond>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        self.0.read().await.get_all_nymd_mixnodes().await
    }

    pub(crate) async fn get_gateways(&self) -> Result<Vec<GatewayBond>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        self.0.read().await.get_all_nymd_gateways().await
    }

    #[allow(dead_code)]
    // I've got a feeling we will need this again very soon, so I'd rather not remove this
    // (and all subcalls in the various clients) just yet
    pub(crate) async fn get_contract_settings(
        &self,
    ) -> Result<ContractStateParams, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        self.0.read().await.get_contract_settings().await
    }

    #[allow(dead_code)]
    pub(crate) async fn get_operator_rewards(
        &self,
        address: String,
    ) -> Result<u128, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        self.0.read().await.get_operator_rewards(address).await
    }

    #[allow(dead_code)]
    pub(crate) async fn get_delegator_rewards(
        &self,
        address: String,
        mix_identity: IdentityKey,
    ) -> Result<u128, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        self.0
            .read()
            .await
            .get_delegator_rewards(address, mix_identity)
            .await
    }

    pub(crate) async fn get_current_epoch(&self) -> Result<Interval, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        self.0.read().await.get_current_epoch().await
    }

    pub(crate) async fn get_current_epoch_reward_params(
        &self,
    ) -> Result<EpochRewardParams, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        let this = self.0.read().await;

        let state = this.get_contract_settings().await?;
        let reward_pool = this.get_reward_pool().await?;
        let interval_reward_percent = this.get_interval_reward_percent().await?;

        let epoch_reward_params = EpochRewardParams::new(
            (reward_pool / 100 / this.get_epochs_in_interval().await? as u128)
                * interval_reward_percent as u128,
            state.mixnode_rewarded_set_size as u128,
            state.mixnode_active_set_size as u128,
            this.get_circulating_supply().await?,
            this.get_sybil_resistance_percent().await?,
            this.get_active_set_work_factor().await?,
        );

        Ok(epoch_reward_params)
    }

    #[allow(dead_code)]
    pub(crate) async fn get_rewarding_status(
        &self,
        mix_identity: mixnet_contract_common::IdentityKey,
        interval_id: u32,
    ) -> Result<MixnodeRewardingStatusResponse, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        self.0
            .read()
            .await
            .get_rewarding_status(mix_identity, interval_id)
            .await
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

    #[allow(dead_code)]
    pub(crate) async fn get_mixnode_delegations(
        &self,
        identity: IdentityKey,
    ) -> Result<Vec<Delegation>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        self.0
            .read()
            .await
            .get_all_nymd_single_mixnode_delegations(identity)
            .await
    }

    pub(crate) async fn get_rewarded_set_identities(
        &self,
    ) -> Result<Vec<(IdentityKey, RewardedSetNodeStatus)>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        self.0
            .read()
            .await
            .get_all_nymd_rewarded_set_mixnode_identities()
            .await
    }

    #[allow(dead_code)]
    pub(crate) async fn advance_current_epoch(&self) -> Result<(), ValidatorClientError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        self.0.write().await.nymd.advance_current_epoch().await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) async fn checkpoint_mixnodes(&self) -> Result<(), ValidatorClientError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        self.0.write().await.nymd.checkpoint_mixnodes().await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) async fn reconcile_delegations(&self) -> Result<(), ValidatorClientError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        self.0.write().await.nymd.reconcile_delegations().await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) async fn write_rewarded_set(
        &self,
        rewarded_set: Vec<IdentityKey>,
        expected_active_set_size: u32,
    ) -> Result<(), ValidatorClientError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        self.0
            .write()
            .await
            .nymd
            .write_rewarded_set(rewarded_set, expected_active_set_size)
            .await?;
        Ok(())
    }

    pub(crate) async fn epoch_operations(
        &self,
        rewarded_set: Vec<IdentityKey>,
        expected_active_set_size: u32,
        reward_msgs: Vec<(ExecuteMsg, Vec<CosmosCoin>)>,
    ) -> Result<(), RewardingError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let mut msgs = reward_msgs;

        let epoch_msgs = vec![
            (ExecuteMsg::ReconcileDelegations {}, vec![]),
            (ExecuteMsg::CheckpointMixnodes {}, vec![]),
            (ExecuteMsg::AdvanceCurrentEpoch {}, vec![]),
            (
                ExecuteMsg::WriteRewardedSet {
                    rewarded_set,
                    expected_active_set_size,
                },
                vec![],
            ),
        ];

        msgs.extend_from_slice(&epoch_msgs);

        let memo = "Performing epoch operations".to_string();

        self.execute_multiple_with_retry(msgs, Default::default(), memo)
            .await?;
        Ok(())
    }

    async fn execute_multiple_with_retry<M>(
        &self,
        msgs: Vec<(M, Vec<CosmosCoin>)>,
        fee: Fee,
        memo: String,
    ) -> Result<(), RewardingError>
    where
        C: SigningCosmWasmClient + Sync,
        M: Serialize + Clone + Send,
    {
        let contract = self
            .0
            .read()
            .await
            .get_mixnet_contract_address()
            .ok_or(RewardingError::UnspecifiedContractAddress)?;

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
    C: CosmWasmClient + Sync + Send,
{
    async fn get_tx(
        &self,
        tx_hash: &str,
    ) -> crate::coconut::error::Result<validator_client::nymd::TxResponse> {
        let tx_hash = tx_hash
            .parse::<validator_client::nymd::tx::Hash>()
            .map_err(|_| crate::coconut::error::CoconutError::TxHashParseError)?;
        Ok(self.0.read().await.nymd.get_tx(tx_hash).await?)
    }
}
