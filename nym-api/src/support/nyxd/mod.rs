// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::error::CoconutError;
use crate::epoch_operations::MixnodeToReward;
use crate::support::config::Config;
use anyhow::Result;
use async_trait::async_trait;
use coconut_bandwidth_contract_common::spend_credential::SpendCredentialResponse;
use coconut_dkg_common::{
    dealer::{ContractDealing, DealerDetails, DealerDetailsResponse},
    types::{EncodedBTEPublicKeyWithProof, Epoch, EpochId},
    verification_key::{ContractVKShare, VerificationKeyShare},
};
use config::defaults::{ChainDetails, NymNetworkDetails, DEFAULT_NYM_API_PORT};
use contracts_common::dealings::ContractSafeBytes;
use cw3::ProposalResponse;
use cw4::MemberResponse;
use mixnet_contract_common::families::{Family, FamilyHead};
use mixnet_contract_common::mixnode::MixNodeDetails;
use mixnet_contract_common::reward_params::RewardingParams;
use mixnet_contract_common::{
    CurrentIntervalResponse, ExecuteMsg, GatewayBond, IdentityKey, LayerAssignment, MixId,
    RewardedSetNodeStatus,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use validator_client::nyxd::error::NyxdError;
use validator_client::nyxd::traits::{MixnetQueryClient, MixnetSigningClient};
use validator_client::nyxd::{
    cosmwasm_client::types::ExecuteResult,
    traits::{
        CoconutBandwidthQueryClient, DkgQueryClient, DkgSigningClient, GroupQueryClient,
        MultisigQueryClient, MultisigSigningClient,
    },
    Fee,
};
use validator_client::nyxd::{
    hash::{Hash, SHA256_HASH_SIZE},
    AccountId, Coin, SigningNyxdClient, TendermintTime, VestingQueryClient,
};
use validator_client::ValidatorClientError;
use vesting_contract_common::AccountVestingCoins;

pub(crate) struct Client(pub(crate) Arc<RwLock<validator_client::Client<SigningNyxdClient>>>);

impl Clone for Client {
    fn clone(&self) -> Self {
        Client(Arc::clone(&self.0))
    }
}

impl Client {
    pub(crate) fn new(config: &Config) -> Self {
        // the api address is irrelevant here as **WE ARE THE API**
        // and we won't be talking on the socket here.
        let api_url = format!("http://localhost:{}", DEFAULT_NYM_API_PORT)
            .parse()
            .unwrap();
        let nyxd_url = config.get_nyxd_url();

        let details = NymNetworkDetails::new_from_env()
            .with_mixnet_contract(Some(config.get_mixnet_contract_address().as_ref()))
            .with_vesting_contract(Some(config.get_vesting_contract_address().as_ref()));

        let client_config = validator_client::Config::try_from_nym_network_details(&details)
            .expect("failed to construct valid validator client config with the provided network")
            .with_urls(nyxd_url, api_url);

        let mnemonic = config.get_mnemonic();

        let inner = validator_client::Client::new_signing(client_config, mnemonic)
            .expect("Failed to connect to nyxd!");

        Client(Arc::new(RwLock::new(inner)))
    }

    pub(crate) async fn client_address(&self) -> AccountId {
        self.0.read().await.nyxd.address().clone()
    }

    pub(crate) async fn chain_details(&self) -> ChainDetails {
        self.0.read().await.nyxd.current_chain_details().clone()
    }

    pub(crate) async fn get_rewarding_validator_address(
        &self,
    ) -> Result<AccountId, ValidatorClientError> {
        let cosmwasm_addr = self
            .0
            .read()
            .await
            .nyxd
            .get_mixnet_contract_state()
            .await?
            .rewarding_validator_address
            .into_string();

        // this should never fail otherwise it implies either
        // 1) our mixnet contract state is invalid
        // 2) cosmwasm accepts invalid addresses
        // 3) cosmrs fails to parse valid addresses
        // all of those options are BAD
        cosmwasm_addr
            .clone()
            .parse()
            .map_err(|_| NyxdError::MalformedAccountAddress(cosmwasm_addr).into())
    }

    // a helper function for the future to obtain the current block timestamp
    #[allow(dead_code)]
    pub(crate) async fn current_block_timestamp(
        &self,
    ) -> Result<TendermintTime, ValidatorClientError> {
        let time = self
            .0
            .read()
            .await
            .nyxd
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
    ) -> Result<Option<[u8; SHA256_HASH_SIZE]>, ValidatorClientError> {
        let hash = match self.0.read().await.nyxd.get_block_hash(height).await? {
            Hash::Sha256(hash) => Some(hash),
            Hash::None => None,
        };

        Ok(hash)
    }

    pub(crate) async fn get_mixnodes(&self) -> Result<Vec<MixNodeDetails>, ValidatorClientError> {
        self.0.read().await.get_all_nyxd_mixnodes_detailed().await
    }

    pub(crate) async fn get_gateways(&self) -> Result<Vec<GatewayBond>, ValidatorClientError> {
        self.0.read().await.get_all_nyxd_gateways().await
    }

    pub(crate) async fn get_current_interval(
        &self,
    ) -> Result<CurrentIntervalResponse, ValidatorClientError> {
        Ok(self.0.read().await.get_current_interval_details().await?)
    }

    pub(crate) async fn get_current_rewarding_parameters(
        &self,
    ) -> Result<RewardingParams, ValidatorClientError> {
        Ok(self.0.read().await.get_rewarding_parameters().await?)
    }

    pub(crate) async fn get_rewarded_set_mixnodes(
        &self,
    ) -> Result<Vec<(MixId, RewardedSetNodeStatus)>, ValidatorClientError> {
        self.0
            .read()
            .await
            .get_all_nyxd_rewarded_set_mixnodes()
            .await
    }

    pub(crate) async fn get_current_vesting_account_storage_key(
        &self,
    ) -> Result<u32, ValidatorClientError> {
        let guard = self.0.read().await;
        let vesting_contract = guard.nyxd.vesting_contract_address();
        // TODO: I don't like the usage of the hardcoded value here
        let res = guard
            .nyxd
            .query_contract_raw(vesting_contract, b"key".to_vec())
            .await?;

        Ok(serde_json::from_slice(&res).map_err(NyxdError::from)?)
    }

    pub(crate) async fn get_all_vesting_coins(
        &self,
    ) -> Result<Vec<AccountVestingCoins>, ValidatorClientError> {
        Ok(self
            .0
            .read()
            .await
            .nyxd
            .get_all_accounts_vesting_coins()
            .await?)
    }

    #[allow(dead_code)]
    pub(crate) async fn get_all_node_families(&self) -> Result<Vec<Family>, ValidatorClientError> {
        self.0.read().await.get_all_node_families().await
    }

    pub(crate) async fn get_all_family_members(
        &self,
    ) -> Result<Vec<(IdentityKey, FamilyHead)>, ValidatorClientError> {
        self.0.read().await.get_all_family_members().await
    }

    pub(crate) async fn send_rewarding_messages(
        &self,
        nodes: &[MixnodeToReward],
    ) -> Result<(), ValidatorClientError> {
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
            .nyxd
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
        new_rewarded_set: Vec<LayerAssignment>,
        expected_active_set_size: u32,
    ) -> Result<(), ValidatorClientError> {
        self.0
            .write()
            .await
            .nyxd
            .advance_current_epoch(new_rewarded_set, expected_active_set_size, None)
            .await?;
        Ok(())
    }

    pub(crate) async fn reconcile_epoch_events(&self) -> Result<(), ValidatorClientError> {
        self.0
            .write()
            .await
            .nyxd
            .reconcile_epoch_events(None, None)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl crate::coconut::client::Client for Client {
    async fn address(&self) -> AccountId {
        self.client_address().await
    }

    async fn get_tx(
        &self,
        tx_hash: &str,
    ) -> crate::coconut::error::Result<validator_client::nyxd::TxResponse> {
        let tx_hash = tx_hash
            .parse::<validator_client::nyxd::tx::Hash>()
            .map_err(|_| CoconutError::TxHashParseError)?;
        Ok(self.0.read().await.nyxd.get_tx(tx_hash).await?)
    }

    async fn get_proposal(
        &self,
        proposal_id: u64,
    ) -> crate::coconut::error::Result<ProposalResponse> {
        Ok(self.0.read().await.nyxd.get_proposal(proposal_id).await?)
    }

    async fn list_proposals(&self) -> crate::coconut::error::Result<Vec<ProposalResponse>> {
        Ok(self.0.read().await.get_all_nyxd_proposals().await?)
    }

    async fn get_spent_credential(
        &self,
        blinded_serial_number: String,
    ) -> crate::coconut::error::Result<SpendCredentialResponse> {
        Ok(self
            .0
            .read()
            .await
            .nyxd
            .get_spent_credential(blinded_serial_number)
            .await?)
    }

    async fn get_current_epoch(&self) -> crate::coconut::error::Result<Epoch> {
        Ok(self.0.read().await.nyxd.get_current_epoch().await?)
    }

    async fn group_member(&self, addr: String) -> crate::coconut::error::Result<MemberResponse> {
        Ok(self.0.read().await.nyxd.member(addr).await?)
    }

    async fn get_current_epoch_threshold(
        &self,
    ) -> crate::coconut::error::Result<Option<dkg::Threshold>> {
        Ok(self
            .0
            .read()
            .await
            .nyxd
            .get_current_epoch_threshold()
            .await?)
    }

    async fn get_self_registered_dealer_details(
        &self,
    ) -> crate::coconut::error::Result<DealerDetailsResponse> {
        let self_address = &self.address().await;
        Ok(self
            .0
            .read()
            .await
            .nyxd
            .get_dealer_details(self_address)
            .await?)
    }

    async fn get_current_dealers(&self) -> crate::coconut::error::Result<Vec<DealerDetails>> {
        Ok(self.0.read().await.get_all_nyxd_current_dealers().await?)
    }

    async fn get_dealings(
        &self,
        idx: usize,
    ) -> crate::coconut::error::Result<Vec<ContractDealing>> {
        Ok(self.0.read().await.get_all_nyxd_epoch_dealings(idx).await?)
    }

    async fn get_verification_key_shares(
        &self,
        epoch_id: EpochId,
    ) -> crate::coconut::error::Result<Vec<ContractVKShare>> {
        Ok(self
            .0
            .read()
            .await
            .get_all_nyxd_verification_key_shares(epoch_id)
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
            .nyxd
            .vote_proposal(proposal_id, vote_yes, fee)
            .await?;
        Ok(())
    }

    async fn execute_proposal(&self, proposal_id: u64) -> crate::coconut::error::Result<()> {
        self.0
            .read()
            .await
            .nyxd
            .execute_proposal(proposal_id, None)
            .await?;
        Ok(())
    }

    async fn advance_epoch_state(&self) -> crate::coconut::error::Result<()> {
        self.0
            .write()
            .await
            .nyxd
            .advance_dkg_epoch_state(None)
            .await?;
        Ok(())
    }

    async fn register_dealer(
        &self,
        bte_key: EncodedBTEPublicKeyWithProof,
        announce_address: String,
    ) -> Result<ExecuteResult, CoconutError> {
        Ok(self
            .0
            .write()
            .await
            .nyxd
            .register_dealer(bte_key, announce_address, None)
            .await?)
    }

    async fn submit_dealing(
        &self,
        dealing_bytes: ContractSafeBytes,
    ) -> Result<ExecuteResult, CoconutError> {
        Ok(self
            .0
            .write()
            .await
            .nyxd
            .submit_dealing_bytes(dealing_bytes, None)
            .await?)
    }

    async fn submit_verification_key_share(
        &self,
        share: VerificationKeyShare,
    ) -> crate::coconut::error::Result<ExecuteResult> {
        Ok(self
            .0
            .write()
            .await
            .nyxd
            .submit_verification_key_share(share, None)
            .await?)
    }
}
