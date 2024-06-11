// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::error::CoconutError;
use crate::epoch_operations::MixnodeWithPerformance;
use crate::support::config::Config;
use anyhow::Result;
use async_trait::async_trait;
use cw3::{ProposalResponse, VoteResponse};
use cw4::MemberResponse;
use nym_coconut_dkg_common::dealer::RegisteredDealerDetails;
use nym_coconut_dkg_common::dealing::{
    DealerDealingsStatusResponse, DealingChunkInfo, DealingMetadata, DealingStatusResponse,
    PartialContractDealing,
};
use nym_coconut_dkg_common::msg::QueryMsg as DkgQueryMsg;
use nym_coconut_dkg_common::types::{ChunkIndex, DealingIndex, PartialContractDealingData, State};
use nym_coconut_dkg_common::{
    dealer::{DealerDetails, DealerDetailsResponse},
    types::{EncodedBTEPublicKeyWithProof, Epoch, EpochId},
    verification_key::{ContractVKShare, VerificationKeyShare},
};
use nym_config::defaults::{ChainDetails, NymNetworkDetails};
use nym_ecash_contract_common::blacklist::BlacklistedAccountResponse;
use nym_ecash_contract_common::deposit::{DepositId, DepositResponse};
use nym_ecash_contract_common::spend_credential::EcashSpentCredentialResponse;
use nym_mixnet_contract_common::families::FamilyHead;
use nym_mixnet_contract_common::mixnode::MixNodeDetails;
use nym_mixnet_contract_common::reward_params::RewardingParams;
use nym_mixnet_contract_common::{
    CurrentIntervalResponse, EpochStatus, ExecuteMsg, GatewayBond, IdentityKey, LayerAssignment,
    MixId, RewardedSetNodeStatus,
};
use nym_validator_client::nyxd::contract_traits::PagedDkgQueryClient;
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::{
    contract_traits::{
        DkgQueryClient, DkgSigningClient, EcashQueryClient, EcashSigningClient, GroupQueryClient,
        MixnetQueryClient, MixnetSigningClient, MultisigQueryClient, MultisigSigningClient,
        NymContractsProvider, PagedMixnetQueryClient, PagedMultisigQueryClient,
        PagedVestingQueryClient,
    },
    cosmwasm_client::types::ExecuteResult,
    CosmWasmClient, Fee,
};
use nym_validator_client::nyxd::{
    hash::{Hash, SHA256_HASH_SIZE},
    AccountId, Coin, TendermintTime,
};
use nym_validator_client::{nyxd, DirectSigningHttpRpcNyxdClient, QueryHttpRpcNyxdClient};
use nym_vesting_contract_common::AccountVestingCoins;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::{RwLock, RwLockReadGuard};

#[macro_export]
macro_rules! query_guard {
  ($guard:expr, $($op:tt)*) => {{
        match &*$guard {
            $crate::support::nyxd::ClientInner::Signing(client) => client.$($op)*,
            $crate::support::nyxd::ClientInner::Query(client) => client.$($op)*,
        }
    }};
}

macro_rules! nyxd_query {
    ($self:expr, $($op:tt)*) => {{
        let guard = $self.inner.read().await;
        match &*guard {
            $crate::support::nyxd::ClientInner::Signing(client) => client.$($op)*,
            $crate::support::nyxd::ClientInner::Query(client) => client.$($op)*,
        }
    }};
}

macro_rules! nyxd_signing_shared {
    ($self:expr, $($op:tt)*) => {{
        let guard = $self.inner.read().await;
        match &*guard {
            $crate::support::nyxd::ClientInner::Signing(client) => client.$($op)*,
            $crate::support::nyxd::ClientInner::Query(_) => panic!("attempted to use a signing method on a query client"),
        }
    }};
}

macro_rules! nyxd_signing {
    ($self:expr, $($op:tt)*) => {{
        let guard = $self.inner.write().await;
        match &*guard {
            $crate::support::nyxd::ClientInner::Signing(client) => client.$($op)*,
            $crate::support::nyxd::ClientInner::Query(_) => panic!("attempted to use a signing method on a query client"),
        }
    }};
}

#[derive(Clone)]
pub(crate) struct Client {
    inner: Arc<RwLock<ClientInner>>,
}

pub enum ClientInner {
    Signing(DirectSigningHttpRpcNyxdClient),
    Query(QueryHttpRpcNyxdClient),
}

impl Client {
    pub(crate) fn new(config: &Config) -> Self {
        let details = NymNetworkDetails::new_from_env();
        let nyxd_url = config.get_nyxd_url();

        let client_config = nyxd::Config::try_from_nym_network_details(&details)
            .expect("failed to construct valid validator client config with the provided network");

        let inner = if let Some(mnemonic) = config.get_mnemonic() {
            ClientInner::Signing(
                DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
                    client_config,
                    nyxd_url.as_str(),
                    mnemonic.clone(),
                )
                .expect("Failed to connect to nyxd!"),
            )
        } else {
            ClientInner::Query(
                QueryHttpRpcNyxdClient::connect(client_config, nyxd_url.as_str())
                    .expect("Failed to connect to nyxd!"),
            )
        };

        Client {
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    pub(crate) async fn read(&self) -> RwLockReadGuard<'_, ClientInner> {
        self.inner.read().await
    }

    pub(crate) async fn client_address(&self) -> AccountId {
        nyxd_signing_shared!(self, address())
    }

    pub(crate) async fn balance<S: Into<String>>(&self, denom: S) -> Result<Coin, NyxdError> {
        let address = self.client_address().await;
        let denom = denom.into();
        let balance = nyxd_query!(self, get_balance(&address, denom.clone()).await?);

        match balance {
            None => Ok(Coin::new(0, denom)),
            Some(coin) => Ok(coin),
        }
    }

    pub(crate) async fn chain_details(&self) -> ChainDetails {
        nyxd_query!(self, current_chain_details().clone())
    }

    pub(crate) async fn get_rewarding_validator_address(&self) -> Result<AccountId, NyxdError> {
        let cosmwasm_addr = nyxd_query!(
            self,
            get_mixnet_contract_state()
                .await?
                .rewarding_validator_address
                .into_string()
        );

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
        let time = nyxd_query!(self, get_current_block_timestamp().await?);

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
        let hash = match nyxd_query!(self, get_block_hash(height).await?) {
            Hash::Sha256(hash) => Some(hash),
            Hash::None => None,
        };

        Ok(hash)
    }

    pub(crate) async fn get_mixnodes(&self) -> Result<Vec<MixNodeDetails>, NyxdError> {
        nyxd_query!(self, get_all_mixnodes_detailed().await)
    }

    pub(crate) async fn get_gateways(&self) -> Result<Vec<GatewayBond>, NyxdError> {
        nyxd_query!(self, get_all_gateways().await)
    }

    pub(crate) async fn get_current_interval(&self) -> Result<CurrentIntervalResponse, NyxdError> {
        nyxd_query!(self, get_current_interval_details().await)
    }

    pub(crate) async fn get_current_epoch_status(&self) -> Result<EpochStatus, NyxdError> {
        nyxd_query!(self, get_current_epoch_status().await)
    }

    pub(crate) async fn get_current_rewarding_parameters(
        &self,
    ) -> Result<RewardingParams, NyxdError> {
        nyxd_query!(self, get_rewarding_parameters().await)
    }

    pub(crate) async fn get_rewarded_set_mixnodes(
        &self,
    ) -> Result<Vec<(MixId, RewardedSetNodeStatus)>, NyxdError> {
        nyxd_query!(self, get_all_rewarded_set_mixnodes().await)
    }

    pub(crate) async fn get_current_vesting_account_storage_key(&self) -> Result<u32, NyxdError> {
        let guard = self.inner.read().await;

        // the expect is fine as we always construct the client with the vesting contract explicitly set
        let vesting_contract = query_guard!(
            guard,
            vesting_contract_address().expect("vesting contract address is not available")
        );
        // TODO: I don't like the usage of the hardcoded value here
        let res = query_guard!(
            guard,
            query_contract_raw(vesting_contract, b"key".to_vec()).await?
        );
        if res.is_empty() {
            return Ok(0);
        }

        serde_json::from_slice(&res).map_err(NyxdError::from)
    }

    pub(crate) async fn get_all_vesting_coins(
        &self,
    ) -> Result<Vec<AccountVestingCoins>, NyxdError> {
        nyxd_query!(self, get_all_accounts_vesting_coins().await)
    }

    pub(crate) async fn get_all_family_members(
        &self,
    ) -> Result<Vec<(IdentityKey, FamilyHead)>, NyxdError> {
        nyxd_query!(self, get_all_family_members().await)
    }

    pub(crate) async fn get_pending_events_count(&self) -> Result<u32, NyxdError> {
        let pending = nyxd_query!(self, get_number_of_pending_events().await?);
        Ok(pending.epoch_events + pending.interval_events)
    }

    pub(crate) async fn begin_epoch_transition(&self) -> Result<(), NyxdError> {
        nyxd_signing!(self, begin_epoch_transition(None).await?);
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

        // the expect is fine as we always construct the client with the mixnet contract explicitly set
        let mixnet_contract = nyxd_query!(
            self,
            mixnet_contract_address()
                .expect("mixnet contract address is not available")
                .clone()
        );

        let msgs = generate_reward_messages(nodes);

        // "technically" we don't need a write access to the client,
        // but we REALLY don't want to accidentally send any transactions while we're sending rewarding messages
        // as that would have messed up sequence numbers
        nyxd_signing!(
            self,
            execute_multiple(
                &mixnet_contract,
                msgs,
                Default::default(),
                format!("rewarding {} mixnodes", nodes.len()),
            )
            .await?
        );
        Ok(())
    }

    pub(crate) async fn advance_current_epoch(
        &self,
        new_rewarded_set: Vec<LayerAssignment>,
        expected_active_set_size: u32,
    ) -> Result<(), NyxdError> {
        nyxd_signing!(
            self,
            advance_current_epoch(new_rewarded_set, expected_active_set_size, None).await?
        );
        Ok(())
    }

    pub(crate) async fn reconcile_epoch_events(&self, limit: Option<u32>) -> Result<(), NyxdError> {
        nyxd_signing!(self, reconcile_epoch_events(limit, None).await?);
        Ok(())
    }
}

#[async_trait]
impl crate::ecash::client::Client for Client {
    async fn address(&self) -> AccountId {
        self.client_address().await
    }

    async fn dkg_contract_address(&self) -> Result<AccountId, CoconutError> {
        nyxd_query!(
            self,
            dkg_contract_address()
                .cloned()
                .ok_or_else(|| NyxdError::unavailable_contract_address("dkg contract").into())
        )
    }

    async fn get_deposit(
        &self,
        deposit_id: DepositId,
    ) -> crate::ecash::error::Result<DepositResponse> {
        Ok(nyxd_query!(self, get_deposit(deposit_id).await?))
    }

    async fn get_proposal(
        &self,
        proposal_id: u64,
    ) -> crate::ecash::error::Result<ProposalResponse> {
        Ok(nyxd_query!(self, query_proposal(proposal_id).await?))
    }

    async fn list_proposals(&self) -> crate::ecash::error::Result<Vec<ProposalResponse>> {
        Ok(nyxd_query!(self, get_all_proposals().await?))
    }

    async fn get_vote(
        &self,
        proposal_id: u64,
        voter: String,
    ) -> crate::ecash::error::Result<VoteResponse> {
        Ok(nyxd_query!(self, query_vote(proposal_id, voter).await?))
    }

    async fn get_spent_credential(
        &self,
        blinded_serial_number: String,
    ) -> crate::ecash::error::Result<EcashSpentCredentialResponse> {
        Ok(nyxd_query!(
            self,
            get_spent_credential(blinded_serial_number).await?
        ))
    }

    async fn propose_for_blacklist(
        &self,
        public_key: String,
    ) -> crate::ecash::error::Result<ExecuteResult> {
        Ok(nyxd_signing!(
            self,
            propose_for_blacklist(public_key, None).await?
        ))
    }

    async fn get_blacklisted_account(
        &self,
        public_key: String,
    ) -> crate::ecash::error::Result<BlacklistedAccountResponse> {
        Ok(nyxd_query!(
            self,
            get_blacklisted_account(public_key).await?
        ))
    }

    async fn contract_state(&self) -> crate::ecash::error::Result<State> {
        Ok(nyxd_query!(self, get_state().await?))
    }

    async fn get_current_epoch(&self) -> crate::ecash::error::Result<Epoch> {
        Ok(nyxd_query!(self, get_current_epoch().await?))
    }

    async fn group_member(&self, addr: String) -> crate::ecash::error::Result<MemberResponse> {
        Ok(nyxd_query!(self, member(addr, None).await?))
    }

    async fn get_current_epoch_threshold(
        &self,
    ) -> crate::ecash::error::Result<Option<nym_dkg::Threshold>> {
        Ok(nyxd_query!(self, get_current_epoch_threshold().await?))
    }

    async fn get_self_registered_dealer_details(
        &self,
    ) -> crate::ecash::error::Result<DealerDetailsResponse> {
        let self_address = &self.address().await;
        Ok(nyxd_query!(self, get_dealer_details(self_address).await?))
    }

    async fn get_registered_dealer_details(
        &self,
        epoch_id: EpochId,
        dealer: String,
    ) -> crate::ecash::error::Result<RegisteredDealerDetails> {
        let dealer = dealer
            .as_str()
            .parse()
            .map_err(|_| NyxdError::MalformedAccountAddress(dealer))?;
        Ok(nyxd_query!(
            self,
            get_registered_dealer_details(&dealer, Some(epoch_id)).await?
        ))
    }

    async fn get_dealer_dealings_status(
        &self,
        epoch_id: EpochId,
        dealer: String,
    ) -> crate::ecash::error::Result<DealerDealingsStatusResponse> {
        Ok(nyxd_query!(
            self,
            get_dealer_dealings_status(epoch_id, dealer).await?
        ))
    }

    async fn get_dealing_status(
        &self,
        epoch_id: EpochId,
        dealer: String,
        dealing_index: DealingIndex,
    ) -> crate::ecash::error::Result<DealingStatusResponse> {
        Ok(nyxd_query!(
            self,
            get_dealing_status(epoch_id, dealer, dealing_index).await?
        ))
    }

    async fn get_current_dealers(&self) -> crate::ecash::error::Result<Vec<DealerDetails>> {
        Ok(nyxd_query!(self, get_all_current_dealers().await?))
    }

    async fn get_dealing_metadata(
        &self,
        epoch_id: EpochId,
        dealer: String,
        dealing_index: DealingIndex,
    ) -> crate::ecash::error::Result<Option<DealingMetadata>> {
        Ok(nyxd_query!(
            self,
            get_dealings_metadata(epoch_id, dealer, dealing_index)
                .await?
                .metadata
        ))
    }

    async fn get_dealing_chunk(
        &self,
        epoch_id: EpochId,
        dealer: &str,
        dealing_index: DealingIndex,
        chunk_index: ChunkIndex,
    ) -> crate::ecash::error::Result<Option<PartialContractDealingData>> {
        Ok(nyxd_query!(
            self,
            get_dealing_chunk(epoch_id, dealer.to_string(), dealing_index, chunk_index)
                .await?
                .chunk
        ))
    }

    async fn get_verification_key_share(
        &self,
        epoch_id: EpochId,
        dealer: String,
    ) -> Result<Option<ContractVKShare>, CoconutError> {
        Ok(nyxd_query!(self, get_vk_share(epoch_id, dealer).await?).share)
    }

    async fn get_verification_key_shares(
        &self,
        epoch_id: EpochId,
    ) -> Result<Vec<ContractVKShare>, CoconutError> {
        Ok(nyxd_query!(
            self,
            get_all_verification_key_shares(epoch_id).await?
        ))
    }

    async fn vote_proposal(
        &self,
        proposal_id: u64,
        vote_yes: bool,
        fee: Option<Fee>,
    ) -> Result<(), CoconutError> {
        nyxd_signing!(self, vote_proposal(proposal_id, vote_yes, fee).await?);
        Ok(())
    }

    async fn execute_proposal(&self, proposal_id: u64) -> crate::ecash::error::Result<()> {
        nyxd_signing!(self, execute_proposal(proposal_id, None).await?);
        Ok(())
    }

    async fn can_advance_epoch_state(&self) -> crate::ecash::error::Result<bool> {
        Ok(nyxd_query!(self, can_advance_state().await?.can_advance()))
    }

    async fn advance_epoch_state(&self) -> crate::ecash::error::Result<()> {
        nyxd_signing!(self, advance_dkg_epoch_state(None).await?);
        Ok(())
    }

    async fn register_dealer(
        &self,
        bte_key: EncodedBTEPublicKeyWithProof,
        identity_key: IdentityKey,
        announce_address: String,
        resharing: bool,
    ) -> Result<ExecuteResult, CoconutError> {
        Ok(nyxd_signing!(
            self,
            register_dealer(bte_key, identity_key, announce_address, resharing, None).await?
        ))
    }

    async fn submit_dealing_metadata(
        &self,
        dealing_index: DealingIndex,
        chunks: Vec<DealingChunkInfo>,
        resharing: bool,
    ) -> crate::ecash::error::Result<ExecuteResult> {
        Ok(nyxd_signing!(
            self,
            submit_dealing_metadata(dealing_index, chunks, resharing, None).await?
        ))
    }

    async fn submit_dealing_chunk(
        &self,
        chunk: PartialContractDealing,
    ) -> Result<ExecuteResult, CoconutError> {
        Ok(nyxd_signing!(
            self,
            submit_dealing_chunk(chunk, None).await?
        ))
    }

    async fn submit_verification_key_share(
        &self,
        share: VerificationKeyShare,
        resharing: bool,
    ) -> crate::ecash::error::Result<ExecuteResult> {
        Ok(nyxd_signing!(
            self,
            submit_verification_key_share(share, resharing, None).await?
        ))
    }
}

#[async_trait]
impl DkgQueryClient for Client {
    async fn query_dkg_contract<T>(&self, query: DkgQueryMsg) -> std::result::Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        nyxd_query!(self, query_dkg_contract(query).await)
    }
}
