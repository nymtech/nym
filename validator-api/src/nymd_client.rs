// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::rewarding::{
    error::RewardingError, GatewayToReward, MixnodeToReward, GATEWAY_REWARD_OP_BASE_GAS_LIMIT,
    MIXNODE_REWARD_OP_BASE_GAS_LIMIT, PER_GATEWAY_DELEGATION_GAS_INCREASE,
    PER_MIXNODE_DELEGATION_GAS_INCREASE, REWARDING_GAS_LIMIT_MULTIPLIER,
};
use config::defaults::DEFAULT_VALIDATOR_API_PORT;
use mixnet_contract::{Delegation, ExecuteMsg, GatewayBond, IdentityKey, MixNodeBond, StateParams};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;
use validator_client::nymd::{
    CosmWasmClient, Fee, QueryNymdClient, SigningCosmWasmClient, SigningNymdClient, TendermintTime,
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

        let mixnet_contract = config
            .get_mixnet_contract_address()
            .parse()
            .expect("the mixnet contract address is invalid!");

        let client_config = validator_client::Config::new(nymd_url, api_url, Some(mixnet_contract));
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

        let mixnet_contract = config
            .get_mixnet_contract_address()
            .parse()
            .expect("the mixnet contract address is invalid!");
        let mnemonic = config
            .get_mnemonic()
            .parse()
            .expect("the mnemonic is invalid!");

        let client_config = validator_client::Config::new(nymd_url, api_url, Some(mixnet_contract));
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

    pub(crate) async fn get_state_params(&self) -> Result<StateParams, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        self.0.read().await.get_state_params().await
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

    pub(crate) async fn get_gateway_delegations(
        &self,
        identity: IdentityKey,
    ) -> Result<Vec<Delegation>, ValidatorClientError>
    where
        C: CosmWasmClient + Sync,
    {
        self.0
            .read()
            .await
            .get_all_nymd_single_gateway_delegations(identity)
            .await
    }

    async fn estimate_mixnode_reward_fees(&self, nodes: usize, total_delegations: usize) -> Fee {
        let base_gas_limit = MIXNODE_REWARD_OP_BASE_GAS_LIMIT * nodes as u64
            + PER_MIXNODE_DELEGATION_GAS_INCREASE * total_delegations as u64;

        let total_gas_limit = (base_gas_limit as f64 * REWARDING_GAS_LIMIT_MULTIPLIER) as u64;

        self.0
            .read()
            .await
            .nymd
            .calculate_custom_fee(total_gas_limit)
    }

    async fn estimate_gateway_reward_fees(&self, nodes: usize, total_delegations: usize) -> Fee {
        let base_gas_limit = GATEWAY_REWARD_OP_BASE_GAS_LIMIT * nodes as u64
            + PER_GATEWAY_DELEGATION_GAS_INCREASE * total_delegations as u64;

        let total_gas_limit = (base_gas_limit as f64 * REWARDING_GAS_LIMIT_MULTIPLIER) as u64;

        self.0
            .read()
            .await
            .nymd
            .calculate_custom_fee(total_gas_limit)
    }

    pub(crate) async fn reward_mixnodes(
        &self,
        nodes: &[MixnodeToReward],
    ) -> Result<(), RewardingError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let total_delegations = nodes.iter().map(|node| node.total_delegations).sum();
        let fee = self
            .estimate_mixnode_reward_fees(nodes.len(), total_delegations)
            .await;
        let msgs: Vec<(ExecuteMsg, _)> = nodes
            .iter()
            .map(Into::into)
            .zip(std::iter::repeat(Vec::new()))
            .collect();

        let memo = format!("rewarding {} mixnodes", msgs.len());

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

    pub(crate) async fn reward_gateways(
        &self,
        nodes: &[GatewayToReward],
    ) -> Result<(), RewardingError>
    where
        C: SigningCosmWasmClient + Sync,
    {
        let total_delegations = nodes.iter().map(|node| node.total_delegations).sum();
        let fee = self
            .estimate_gateway_reward_fees(nodes.len(), total_delegations)
            .await;
        let msgs: Vec<(ExecuteMsg, _)> = nodes
            .iter()
            .map(Into::into)
            .zip(std::iter::repeat(Vec::new()))
            .collect();

        let memo = format!("rewarding {} gateways", msgs.len());

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
