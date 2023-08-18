// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::error::CoconutError;
use crate::epoch_operations::MixnodeWithPerformance;
use crate::support::config::Config;
use anyhow::Result;
use async_trait::async_trait;
use cw3::ProposalResponse;
use cw4::MemberResponse;
use nym_coconut_bandwidth_contract_common::spend_credential::SpendCredentialResponse;
use nym_coconut_dkg_common::msg::QueryMsg as DkgQueryMsg;
use nym_coconut_dkg_common::types::InitialReplacementData;
use nym_coconut_dkg_common::{
    dealer::{ContractDealing, DealerDetails, DealerDetailsResponse},
    types::{EncodedBTEPublicKeyWithProof, Epoch, EpochId},
    verification_key::{ContractVKShare, VerificationKeyShare},
};
use nym_config::defaults::ChainDetails;
use nym_contracts_common::dealings::ContractSafeBytes;
use nym_ephemera_common::msg::QueryMsg as EphemeraQueryMsg;
use nym_ephemera_common::types::JsonPeerInfo;
use nym_mixnet_contract_common::families::FamilyHead;
use nym_mixnet_contract_common::mixnode::MixNodeDetails;
use nym_mixnet_contract_common::reward_params::RewardingParams;
use nym_mixnet_contract_common::{
    CurrentIntervalResponse, EpochStatus, ExecuteMsg, GatewayBond, IdentityKey, LayerAssignment,
    MixId, RewardedSetNodeStatus,
};
use nym_name_service_common::msg::QueryMsg as NameServiceQueryMsg;
use nym_service_provider_directory_common::msg::QueryMsg as SpQueryMsg;
use nym_validator_client::nyxd::contract_traits::{NameServiceQueryClient, PagedDkgQueryClient};
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::{
    contract_traits::{
        CoconutBandwidthQueryClient, DkgQueryClient, DkgSigningClient, EphemeraQueryClient,
        EphemeraSigningClient, GroupQueryClient, MixnetQueryClient, MixnetSigningClient,
        MultisigQueryClient, MultisigSigningClient, NymContractsProvider, PagedEphemeraQueryClient,
        PagedMixnetQueryClient, PagedMultisigQueryClient, PagedVestingQueryClient,
        SpDirectoryQueryClient,
    },
    cosmwasm_client::types::ExecuteResult,
    CosmWasmClient, Fee,
};
use nym_validator_client::nyxd::{
    hash::{Hash, SHA256_HASH_SIZE},
    AccountId, Coin, TendermintTime,
};
use nym_validator_client::{nyxd, DirectSigningHttpRpcNyxdClient};
use nym_vesting_contract_common::AccountVestingCoins;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard};

pub(crate) struct Client(pub(crate) Arc<RwLock<DirectSigningHttpRpcNyxdClient>>);

impl Clone for Client {
    fn clone(&self) -> Self {
        Client(Arc::clone(&self.0))
    }
}

impl Client {
    pub(crate) fn new(config: &Config) -> Self {
        let details = config.get_network_details();
        let nyxd_url = config.get_nyxd_url();

        let client_config = nyxd::Config::try_from_nym_network_details(&details)
            .expect("failed to construct valid validator client config with the provided network");

        let mnemonic = config.get_mnemonic();

        let inner = DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
            client_config,
            nyxd_url.as_str(),
            mnemonic,
        )
        .expect("Failed to connect to nyxd!");

        Client(Arc::new(RwLock::new(inner)))
    }

    pub(crate) async fn read(&self) -> RwLockReadGuard<'_, DirectSigningHttpRpcNyxdClient> {
        self.0.read().await
    }

    pub(crate) async fn client_address(&self) -> AccountId {
        self.0.read().await.address()
    }

    pub(crate) async fn chain_details(&self) -> ChainDetails {
        self.0.read().await.current_chain_details().clone()
    }

    pub(crate) async fn get_rewarding_validator_address(&self) -> Result<AccountId, NyxdError> {
        let cosmwasm_addr = self
            .0
            .read()
            .await
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
            .map_err(|_| NyxdError::MalformedAccountAddress(cosmwasm_addr))
    }

    // a helper function for the future to obtain the current block timestamp
    #[allow(dead_code)]
    pub(crate) async fn current_block_timestamp(&self) -> Result<TendermintTime, NyxdError> {
        let time = self.0.read().await.get_current_block_timestamp().await?;

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
    ) -> Result<Option<[u8; SHA256_HASH_SIZE]>, NyxdError> {
        let hash = match self.0.read().await.get_block_hash(height).await? {
            Hash::Sha256(hash) => Some(hash),
            Hash::None => None,
        };

        Ok(hash)
    }

    pub(crate) async fn get_mixnodes(&self) -> Result<Vec<MixNodeDetails>, NyxdError> {
        self.0.read().await.get_all_mixnodes_detailed().await
    }

    pub(crate) async fn get_gateways(&self) -> Result<Vec<GatewayBond>, NyxdError> {
        self.0.read().await.get_all_gateways().await
    }

    pub(crate) async fn get_current_interval(&self) -> Result<CurrentIntervalResponse, NyxdError> {
        self.0.read().await.get_current_interval_details().await
    }

    pub(crate) async fn get_current_epoch_status(&self) -> Result<EpochStatus, NyxdError> {
        self.0.read().await.get_current_epoch_status().await
    }

    pub(crate) async fn get_current_rewarding_parameters(
        &self,
    ) -> Result<RewardingParams, NyxdError> {
        self.0.read().await.get_rewarding_parameters().await
    }

    pub(crate) async fn get_rewarded_set_mixnodes(
        &self,
    ) -> Result<Vec<(MixId, RewardedSetNodeStatus)>, NyxdError> {
        self.0.read().await.get_all_rewarded_set_mixnodes().await
    }

    pub(crate) async fn get_current_vesting_account_storage_key(&self) -> Result<u32, NyxdError> {
        let guard = self.0.read().await;

        // the expect is fine as we always construct the client with the vesting contract explicitly set
        let vesting_contract = guard
            .vesting_contract_address()
            .expect("vesting contract address is not available");
        // TODO: I don't like the usage of the hardcoded value here
        let res = guard
            .query_contract_raw(vesting_contract, b"key".to_vec())
            .await?;
        if res.is_empty() {
            return Ok(0);
        }

        serde_json::from_slice(&res).map_err(NyxdError::from)
    }

    pub(crate) async fn get_all_vesting_coins(
        &self,
    ) -> Result<Vec<AccountVestingCoins>, NyxdError> {
        self.0.read().await.get_all_accounts_vesting_coins().await
    }

    pub(crate) async fn get_all_family_members(
        &self,
    ) -> Result<Vec<(IdentityKey, FamilyHead)>, NyxdError> {
        self.0.read().await.get_all_family_members().await
    }

    pub(crate) async fn get_pending_events_count(&self) -> Result<u32, NyxdError> {
        let pending = self.0.read().await.get_number_of_pending_events().await?;
        Ok(pending.epoch_events + pending.interval_events)
    }

    pub(crate) async fn begin_epoch_transition(&self) -> Result<(), NyxdError> {
        self.0.write().await.begin_epoch_transition(None).await?;
        Ok(())
    }

    pub(crate) async fn send_rewarding_messages(
        &self,
        nodes: &[MixnodeWithPerformance],
    ) -> Result<(), NyxdError> {
        // for some reason, compiler complains if this is explicitly inline in code ¯\_(ツ)_/¯
        #[inline]
        #[allow(unused_variables)]
        fn generate_reward_messages(
            eligible_mixnodes: &[MixnodeWithPerformance],
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

        // "technically" we don't need a write access to the client,
        // but we REALLY don't want to accidentally send any transactions while we're sending rewarding messages
        // as that would have messed up sequence numbers
        let guard = self.0.write().await;

        // the expect is fine as we always construct the client with the mixnet contract explicitly set
        let mixnet_contract = guard
            .mixnet_contract_address()
            .expect("mixnet contract address is not available");

        let msgs = generate_reward_messages(nodes);

        guard
            .execute_multiple(
                mixnet_contract,
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
    ) -> Result<(), NyxdError> {
        self.0
            .write()
            .await
            .advance_current_epoch(new_rewarded_set, expected_active_set_size, None)
            .await?;
        Ok(())
    }

    pub(crate) async fn reconcile_epoch_events(&self, limit: Option<u32>) -> Result<(), NyxdError> {
        self.0
            .write()
            .await
            .reconcile_epoch_events(limit, None)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl crate::coconut::client::Client for Client {
    async fn address(&self) -> AccountId {
        self.client_address().await
    }

    async fn get_tx(&self, tx_hash: &str) -> crate::coconut::error::Result<nyxd::TxResponse> {
        let tx_hash: Hash = tx_hash
            .parse()
            .map_err(|_| CoconutError::TxHashParseError)?;
        Ok(self.0.read().await.get_tx(tx_hash).await?)
    }

    async fn get_proposal(
        &self,
        proposal_id: u64,
    ) -> crate::coconut::error::Result<ProposalResponse> {
        Ok(self.0.read().await.query_proposal(proposal_id).await?)
    }

    async fn list_proposals(&self) -> crate::coconut::error::Result<Vec<ProposalResponse>> {
        Ok(self.0.read().await.get_all_proposals().await?)
    }

    async fn get_spent_credential(
        &self,
        blinded_serial_number: String,
    ) -> crate::coconut::error::Result<SpendCredentialResponse> {
        Ok(self
            .0
            .read()
            .await
            .get_spent_credential(blinded_serial_number)
            .await?)
    }

    async fn get_current_epoch(&self) -> crate::coconut::error::Result<Epoch> {
        Ok(self.0.read().await.get_current_epoch().await?)
    }

    async fn group_member(&self, addr: String) -> crate::coconut::error::Result<MemberResponse> {
        Ok(self.0.read().await.member(addr, None).await?)
    }

    async fn get_current_epoch_threshold(
        &self,
    ) -> crate::coconut::error::Result<Option<nym_dkg::Threshold>> {
        Ok(self.0.read().await.get_current_epoch_threshold().await?)
    }

    async fn get_initial_dealers(
        &self,
    ) -> crate::coconut::error::Result<Option<InitialReplacementData>> {
        Ok(self.0.read().await.get_initial_dealers().await?)
    }

    async fn get_self_registered_dealer_details(
        &self,
    ) -> crate::coconut::error::Result<DealerDetailsResponse> {
        let self_address = &self.address().await;
        Ok(self.0.read().await.get_dealer_details(self_address).await?)
    }

    async fn get_current_dealers(&self) -> crate::coconut::error::Result<Vec<DealerDetails>> {
        Ok(self.0.read().await.get_all_current_dealers().await?)
    }

    async fn get_dealings(
        &self,
        idx: usize,
    ) -> crate::coconut::error::Result<Vec<ContractDealing>> {
        Ok(self
            .0
            .read()
            .await
            .get_all_epoch_dealings(idx as u64)
            .await?)
    }

    async fn get_verification_key_shares(
        &self,
        epoch_id: EpochId,
    ) -> crate::coconut::error::Result<Vec<ContractVKShare>> {
        Ok(self
            .0
            .read()
            .await
            .get_all_verification_key_shares(epoch_id)
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
            .vote_proposal(proposal_id, vote_yes, fee)
            .await?;
        Ok(())
    }

    async fn execute_proposal(&self, proposal_id: u64) -> crate::coconut::error::Result<()> {
        self.0
            .read()
            .await
            .execute_proposal(proposal_id, None)
            .await?;
        Ok(())
    }

    async fn advance_epoch_state(&self) -> crate::coconut::error::Result<()> {
        self.0.write().await.advance_dkg_epoch_state(None).await?;
        Ok(())
    }

    async fn register_dealer(
        &self,
        bte_key: EncodedBTEPublicKeyWithProof,
        announce_address: String,
        resharing: bool,
    ) -> Result<ExecuteResult, CoconutError> {
        Ok(self
            .0
            .write()
            .await
            .register_dealer(bte_key, announce_address, resharing, None)
            .await?)
    }

    async fn submit_dealing(
        &self,
        dealing_bytes: ContractSafeBytes,
        resharing: bool,
    ) -> Result<ExecuteResult, CoconutError> {
        Ok(self
            .0
            .write()
            .await
            .submit_dealing_bytes(dealing_bytes, resharing, None)
            .await?)
    }

    async fn submit_verification_key_share(
        &self,
        share: VerificationKeyShare,
        resharing: bool,
    ) -> crate::coconut::error::Result<ExecuteResult> {
        Ok(self
            .0
            .write()
            .await
            .submit_verification_key_share(share, resharing, None)
            .await?)
    }
}

#[async_trait]
impl crate::ephemera::client::Client for Client {
    async fn get_ephemera_peers(&self) -> crate::ephemera::error::Result<Vec<JsonPeerInfo>> {
        Ok(self.0.read().await.get_all_ephemera_peers().await?)
    }

    async fn register_ephemera_peer(
        &self,
        peer_info: JsonPeerInfo,
    ) -> crate::ephemera::error::Result<ExecuteResult> {
        Ok(self
            .0
            .write()
            .await
            .register_as_peer(peer_info, None)
            .await?)
    }
}

#[async_trait]
impl DkgQueryClient for Client {
    async fn query_dkg_contract<T>(&self, query: DkgQueryMsg) -> std::result::Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        self.0.read().await.query_dkg_contract(query).await
    }
}

#[async_trait]
impl EphemeraQueryClient for Client {
    async fn query_ephemera_contract<T>(
        &self,
        query: EphemeraQueryMsg,
    ) -> std::result::Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        self.0.read().await.query_ephemera_contract(query).await
    }
}

#[async_trait]
impl SpDirectoryQueryClient for Client {
    async fn query_service_provider_contract<T>(
        &self,
        query: SpQueryMsg,
    ) -> std::result::Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        self.0
            .read()
            .await
            .query_service_provider_contract(query)
            .await
    }
}

#[async_trait]
impl NameServiceQueryClient for Client {
    async fn query_name_service_contract<T>(
        &self,
        query: NameServiceQueryMsg,
    ) -> std::result::Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        self.0.read().await.query_name_service_contract(query).await
    }
}
