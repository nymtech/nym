// Copyright 2021-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::Config;
use crate::epoch_operations::MixnodeToReward;
use config::defaults::{NymNetworkDetails, DEFAULT_VALIDATOR_API_PORT};
use mixnet_contract_common::mixnode::MixNodeDetails;
use mixnet_contract_common::reward_params::RewardingParams;
use mixnet_contract_common::{
    CurrentIntervalResponse, ExecuteMsg, GatewayBond, NodeId, RewardedSetNodeStatus,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use validator_client::nymd::traits::{MixnetQueryClient, MixnetSigningClient};
use validator_client::nymd::{
    hash::{Hash, SHA256_HASH_SIZE},
    Coin, CosmWasmClient, QueryNymdClient, SigningCosmWasmClient, SigningNymdClient,
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
    AccountId, Fee,
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

    pub(crate) async fn send_rewarding_messages(
        &self,
        nodes: &[MixnodeToReward],
    ) -> Result<(), ValidatorClientError>
    where
        C: SigningCosmWasmClient + Sync + Send,
    {
        // for some reason, compiler complains if this is explicitly inline in code ¯\_(ツ)_/¯
        #[inline]
        #[allow(unused_variables)]
        fn generate_reward_messages(
            eligible_mixnodes: &[MixnodeToReward],
        ) -> Vec<(ExecuteMsg, Vec<Coin>)> {
            cfg_if::cfg_if! {
                if #[cfg(feature = "no-reward")] {
                    vec![]
                } else {
                    eligible_mixnodes
                        .iter()
                    .map(|node| (*node).into())
                        .zip(std::iter::repeat(Vec::new()))
                        .collect()
                }
            }
        }

        let contract = self.0.read().await.get_mixnet_contract_address();

        let msgs = generate_reward_messages(nodes);

        self.0
            .write()
            .await
            .nymd
            .execute_multiple(
                &contract,
                msgs,
                Default::default(),
                format!("rewarding {} mixnodes", nodes.len()),
            )
            .await?;
        Ok(())
    }

    pub(crate) async fn advance_current_epoch(
        &self,
        new_rewarded_set: Vec<NodeId>,
        expected_active_set_size: u32,
    ) -> Result<(), ValidatorClientError>
    where
        C: SigningCosmWasmClient + Sync + Send,
    {
        self.0
            .write()
            .await
            .nymd
            .advance_current_epoch(new_rewarded_set, expected_active_set_size, None)
            .await?;
        Ok(())
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
