// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::rewarding::{error::RewardingError, IntervalRewardParams, MixnodeToReward};
use config::defaults::{default_network, DEFAULT_VALIDATOR_API_PORT};
use mixnet_contract_common::{
    ContractStateParams, Delegation, ExecuteMsg, GatewayBond, IdentityKey, Interval, MixNodeBond,
    MixnodeRewardingStatusResponse, RewardedSetNodeStatus, RewardedSetUpdateDetails,
    MIXNODE_DELEGATORS_PAGE_LIMIT,
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

pub(crate) struct Client<C>(Arc<RwLock<validator_client::Client<C>>>);

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
        let network = default_network();

        let mixnet_contract = config
            .get_mixnet_contract_address()
            .parse()
            .expect("the mixnet contract address is invalid!");

        let client_config =
            validator_client::Config::new(network, nymd_url, api_url, Some(mixnet_contract), None);
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
        let network = default_network();

        let mixnet_contract = config
            .get_mixnet_contract_address()
            .parse()
            .expect("the mixnet contract address is invalid!");
        let mnemonic = config
            .get_mnemonic()
            .parse()
            .expect("the mnemonic is invalid!");

        let client_config =
            validator_client::Config::new(network, nymd_url, api_url, Some(mixnet_contract), None);
        let inner = validator_client::Client::new_signing(client_config, mnemonic)
            .expect("Failed to connect to nymd!");

        Client(Arc::new(RwLock::new(inner)))
    }
}

impl<C> Client<C> {
    // a helper function for the future to obtain the current block timestamp
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

    pub(crate) async fn get_current_interval(&self) -> Result<Interval, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        Ok(self.0.read().await.get_current_interval().await?)
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

    pub(crate) async fn get_current_interval_reward_params(
        &self,
    ) -> Result<IntervalRewardParams, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        let this = self.0.read().await;

        let state = this.get_contract_settings().await?;
        let reward_pool = this.get_reward_pool().await?;
        let interval_reward_percent = this.get_interval_reward_percent().await?;

        let interval_reward_params = IntervalRewardParams {
            reward_pool,
            circulating_supply: this.get_circulating_supply().await?,
            sybil_resistance_percent: this.get_sybil_resistance_percent().await?,
            rewarded_set_size: state.mixnode_rewarded_set_size,
            active_set_size: state.mixnode_active_set_size,
            period_reward_pool: (reward_pool / 100) * interval_reward_percent as u128,
            active_set_work_factor: this.get_active_set_work_factor().await?,
        };

        Ok(interval_reward_params)
    }

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

    pub(crate) async fn get_current_rewarded_set_update_details(
        &self,
    ) -> Result<RewardedSetUpdateDetails, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        self.0
            .read()
            .await
            .get_current_rewarded_set_update_details()
            .await
    }

    pub(crate) async fn advance_current_interval(&self) -> Result<(), ValidatorClientError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        self.0.write().await.nymd.advance_current_interval().await?;
        Ok(())
    }

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

    pub(crate) async fn reward_mixnode_and_all_delegators(
        &self,
        node: &MixnodeToReward,
        interval_id: u32,
    ) -> Result<(), RewardingError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        // start with the base call to reward operator and first page of delegators
        let msgs = vec![(node.to_reward_execute_msg(interval_id), vec![])];
        let memo = format!(
            "operator + {} delegators rewarding",
            MIXNODE_DELEGATORS_PAGE_LIMIT
        );
        self.execute_multiple_with_retry(msgs, Default::default(), memo)
            .await?;

        // reward rest of delegators (if applicable)
        if node.total_delegations > MIXNODE_DELEGATORS_PAGE_LIMIT {
            // the first 'batch' of delegators has been rewarded while rewarding the operator
            let mut remaining_delegators = node.total_delegations - MIXNODE_DELEGATORS_PAGE_LIMIT;
            let delegator_rewarding_msg = (
                node.to_next_delegator_reward_execute_msg(interval_id),
                vec![],
            );

            while remaining_delegators > 0 {
                let delegators_in_call = remaining_delegators.min(MIXNODE_DELEGATORS_PAGE_LIMIT);
                let msgs = vec![delegator_rewarding_msg.clone()];
                let memo = format!("rewarding another {} delegators", delegators_in_call);
                self.execute_multiple_with_retry(msgs, Default::default(), memo)
                    .await?;

                remaining_delegators =
                    remaining_delegators.saturating_sub(MIXNODE_DELEGATORS_PAGE_LIMIT)
            }
        }

        Ok(())
    }

    pub(crate) async fn reward_mix_delegators(
        &self,
        node: &MixnodeToReward,
        interval_id: u32,
    ) -> Result<(), RewardingError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        // the fee is a tricky subject here because we don't know exactly how many delegators we missed,
        // let's aim for the worst case scenario and assume it was the entire page
        let delegator_rewarding_msg = (
            node.to_next_delegator_reward_execute_msg(interval_id),
            vec![],
        );

        let memo = "rewarding delegators".to_string();
        self.execute_multiple_with_retry(vec![delegator_rewarding_msg], Default::default(), memo)
            .await
    }

    pub(crate) async fn reward_mixnodes_with_single_page_of_delegators(
        &self,
        nodes: &[MixnodeToReward],
        interval_id: u32,
    ) -> Result<(), RewardingError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let msgs: Vec<(ExecuteMsg, _)> = nodes
            .iter()
            .map(|node| node.to_reward_execute_msg(interval_id))
            .zip(std::iter::repeat(Vec::new()))
            .collect();

        let memo = format!("rewarding {} mixnodes", msgs.len());

        self.execute_multiple_with_retry(msgs, Default::default(), memo)
            .await
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
