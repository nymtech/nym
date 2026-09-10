// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash::error::EcashError;
use crate::epoch_operations::RewardedNodeWithParams;
use crate::support::config::Config;
use anyhow::{Context, Result};
use async_trait::async_trait;
use cw3::{ProposalResponse, VoteResponse};
use cw4::MemberResponse;
use nym_bin_common::bin_info;
use nym_coconut_dkg_common::dealer::RegisteredDealerDetails;
use nym_coconut_dkg_common::dealing::{
    DealerDealingsStatusResponse, DealingChunkInfo, DealingMetadata, DealingStatusResponse,
    PartialContractDealing,
};
use nym_coconut_dkg_common::msg::QueryMsg as DkgQueryMsg;
use nym_coconut_dkg_common::types::{ChunkIndex, DealingIndex, PartialContractDealingData, State};
use nym_coconut_dkg_common::{
    dealer::{DealerDetails, DealerDetailsResponse},
    types::{EncodedBTEPublicKeyWithProof, Epoch},
    verification_key::{ContractVKShare, VerificationKeyShare},
};
use nym_compact_ecash::{Base58, VerificationKeyAuth};
use nym_config::defaults::{ChainDetails, NymNetworkDetails};
use nym_dkg::Threshold;
use nym_ecash_contract_common::blacklist::BlacklistedAccountResponse;
use nym_ecash_contract_common::deposit::{DepositId, DepositResponse};
use nym_http_api_client::UserAgent;
use nym_mixnet_contract_common::gateway::PreassignedId;
use nym_mixnet_contract_common::mixnode::MixNodeDetails;
use nym_mixnet_contract_common::nym_node::Role;
use nym_mixnet_contract_common::reward_params::RewardingParams;
use nym_mixnet_contract_common::{
    ConfigScoreParams, CurrentIntervalResponse, Delegation, EpochRewardedSet, EpochStatus,
    ExecuteMsg, GatewayBond, HistoricalNymNodeVersionEntry, IdentityKey, KeyRotationState,
    NymNodeDetails, RewardedSet, RoleAssignment,
};
use nym_node_families_contract_common::msg::QueryMsg as NodeFamiliesQueryMsg;
use nym_validator_client::coconut::EcashApiError;
use nym_validator_client::nyxd::contract_traits::mixnet_query_client::MixnetQueryClientExt;
use nym_validator_client::nyxd::contract_traits::performance_query_client::{
    LastSubmission, NodePerformance,
};
use nym_validator_client::nyxd::contract_traits::{
    NetworkMonitorsQueryClient, NodeFamiliesQueryClient, PagedDkgQueryClient,
    PagedPerformanceQueryClient, PerformanceQueryClient, TypedNymContracts,
};
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::nyxd::nym_network_monitors_contract_common::AuthorisedNetworkMonitorOrchestrator;
use nym_validator_client::nyxd::Coin;
use nym_validator_client::nyxd::{
    contract_traits::{
        DkgQueryClient, DkgSigningClient, EcashQueryClient, GroupQueryClient, MixnetQueryClient,
        MixnetSigningClient, MultisigQueryClient, MultisigSigningClient, NymContractsProvider,
        PagedMixnetQueryClient, PagedMultisigQueryClient,
    },
    cosmwasm_client::types::ExecuteResult,
    BlockResponse, Fee, TendermintRpcClient,
};
use nym_validator_client::nyxd::{
    hash::{Hash, SHA256_HASH_SIZE},
    AccountId, TendermintTime,
};
use nym_validator_client::rpc::TendermintRpcClientExt;
use nym_validator_client::{
    nyxd, DirectSigningHttpRpcNyxdClient, EcashApiClient, QueryHttpRpcNyxdClient,
};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use tendermint::abci::response::Info;
use tokio::sync::RwLock;
use tracing::warn;
use url::Url;

macro_rules! nyxd_signing {
    ($self:expr, $($op:tt)*) => {{
        let Some(lock) = &*$self.signing else {
            panic!("attempted to use a signing method on a query-only client");
        };
        let guard = lock.write().await;
        guard.$($op)*
    }};
}

pub(crate) struct Client {
    query: QueryHttpRpcNyxdClient,
    signing: Arc<Option<RwLock<DirectSigningHttpRpcNyxdClient>>>,
}

impl Clone for Client {
    fn clone(&self) -> Self {
        Client {
            query: self.query.clone_query_client(),
            signing: Arc::clone(&self.signing),
        }
    }
}

impl Client {
    pub(crate) fn new(
        config: &Config,
        network_details: &NymNetworkDetails,
    ) -> anyhow::Result<Self> {
        let nyxd_url = config.get_nyxd_url();

        let client_config = nyxd::Config::try_from_nym_network_details(network_details).context(
            "failed to construct valid validator client config with the provided network",
        )?;

        let (query, signing) = if let Some(mnemonic) = config.get_mnemonic() {
            let signing_client = DirectSigningHttpRpcNyxdClient::connect_with_mnemonic(
                client_config,
                nyxd_url.as_str(),
                mnemonic.clone(),
            )
            .context("Failed to connect to nyxd!")?;
            let query = signing_client.clone_query_client();
            (query, Some(RwLock::new(signing_client)))
        } else {
            let query = QueryHttpRpcNyxdClient::connect(client_config, nyxd_url.as_str())
                .context("Failed to connect to nyxd!")?;
            (query, None)
        };

        Ok(Client {
            query,
            signing: Arc::new(signing),
        })
    }

    pub(crate) fn query_client(&self) -> QueryHttpRpcNyxdClient {
        self.query.clone_query_client()
    }

    pub(crate) async fn abci_info(&self) -> Result<Info, NyxdError> {
        Ok(self.query.abci_info().await?)
    }

    pub(crate) async fn block_info(&self, height: u32) -> Result<BlockResponse, NyxdError> {
        Ok(self.query.block(height).await?)
    }

    pub(crate) async fn client_address(&self) -> Option<AccountId> {
        Some((*self.signing).as_ref()?.read().await.address())
    }

    pub(crate) async fn balance<S: Into<String>>(&self, denom: S) -> Result<Coin, NyxdError> {
        let denom = denom.into();
        let Some(address) = self.client_address().await else {
            return Ok(Coin::new(0, denom));
        };
        let balance = self.query.get_balance(&address, denom.clone()).await?;

        match balance {
            None => Ok(Coin::new(0, denom)),
            Some(coin) => Ok(coin),
        }
    }

    /// Return the full set of Nym contract addresses currently configured on this client.
    #[allow(dead_code)]
    pub(crate) async fn known_contracts(&self) -> TypedNymContracts {
        self.query.get_nym_contracts()
    }

    pub(crate) async fn chain_details(&self) -> ChainDetails {
        self.query.get_chain_details()
    }

    pub(crate) async fn get_ecash_contract_address(&self) -> Result<AccountId, EcashError> {
        self.query
            .ecash_contract_address()
            .cloned()
            .ok_or_else(|| NyxdError::unavailable_contract_address("ecash contract").into())
    }

    /// Return the configured network-monitors contract address.
    ///
    /// Returns [`NyxdError::UnavailableContractAddress`] if the address is not configured - this
    /// is the signal used upstream (e.g. in `NetworkMonitorsCache`) to warn that stress-test
    /// submissions cannot be accepted.
    pub(crate) async fn get_network_monitors_contract_address(
        &self,
    ) -> Result<AccountId, NyxdError> {
        self.query
            .network_monitors_contract_address()
            .cloned()
            .ok_or_else(|| NyxdError::unavailable_contract_address("network monitors contract"))
    }

    pub(crate) async fn get_rewarding_validator_address(&self) -> Result<AccountId, NyxdError> {
        let cosmwasm_addr = self
            .query
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
        let time = self.query.get_current_block_timestamp().await?;

        Ok(time)
    }

    /// Tendermint block timestamp at the given height.
    pub(crate) async fn block_timestamp(&self, height: u32) -> Result<TendermintTime, NyxdError> {
        let time = self.query.get_block_timestamp(Some(height)).await?;

        Ok(time)
    }

    /// Latest committed block (height + timestamp) in a single RPC call.
    pub(crate) async fn current_block_info(&self) -> Result<BlockResponse, NyxdError> {
        Ok(self.query.latest_block().await?)
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
        let hash = match self.query.get_block_hash(height).await? {
            Hash::Sha256(hash) => Some(hash),
            Hash::None => None,
        };

        Ok(hash)
    }

    pub(crate) async fn get_nymnodes(&self) -> Result<Vec<NymNodeDetails>, NyxdError> {
        self.query.get_all_nymnodes_detailed().await
    }

    pub(crate) async fn get_mixnodes(&self) -> Result<Vec<MixNodeDetails>, NyxdError> {
        self.query.get_all_mixnodes_detailed().await
    }

    pub(crate) async fn get_gateways(&self) -> Result<Vec<GatewayBond>, NyxdError> {
        self.query.get_all_gateways().await
    }

    pub(crate) async fn get_gateway_ids(&self) -> Result<Vec<PreassignedId>, NyxdError> {
        self.query.get_all_preassigned_gateway_ids().await
    }

    pub(crate) async fn get_key_rotation_state(&self) -> Result<KeyRotationState, NyxdError> {
        self.query.get_key_rotation_state().await
    }

    pub(crate) async fn get_config_score_params(&self) -> Result<ConfigScoreParams, NyxdError> {
        self.query
            .get_mixnet_contract_state_params()
            .await
            .map(|state| state.config_score_params)
    }

    pub(crate) async fn get_nym_node_version_history(
        &self,
    ) -> Result<Vec<HistoricalNymNodeVersionEntry>, NyxdError> {
        self.query.get_full_nym_node_version_history().await
    }

    pub(crate) async fn get_current_interval(&self) -> Result<CurrentIntervalResponse, NyxdError> {
        self.query.get_current_interval_details().await
    }

    pub(crate) async fn get_mixnet_contract_state(
        &self,
    ) -> Result<nym_mixnet_contract_common::ContractState, NyxdError> {
        self.query.get_mixnet_contract_state().await
    }

    pub(crate) async fn get_current_epoch_status(&self) -> Result<EpochStatus, NyxdError> {
        self.query.get_current_epoch_status().await
    }

    pub(crate) async fn get_current_rewarding_parameters(
        &self,
    ) -> Result<RewardingParams, NyxdError> {
        self.query.get_rewarding_parameters().await
    }

    pub(crate) async fn get_rewarded_set_nodes(&self) -> Result<EpochRewardedSet, NyxdError> {
        self.query.get_rewarded_set().await
    }

    pub(crate) async fn get_pending_events_count(&self) -> Result<u32, NyxdError> {
        let pending = self.query.get_number_of_pending_events().await?;
        Ok(pending.epoch_events + pending.interval_events)
    }

    pub(crate) async fn begin_epoch_transition(&self) -> Result<(), NyxdError> {
        nyxd_signing!(self, begin_epoch_transition(None).await?);
        Ok(())
    }

    fn generate_reward_messages(
        &self,
        rewarded_set: &[RewardedNodeWithParams],
    ) -> Vec<(ExecuteMsg, Vec<Coin>)> {
        rewarded_set
            .iter()
            .map(|node| (*node).into())
            .zip(std::iter::repeat(Vec::new()))
            .collect()
    }

    pub(crate) async fn send_rewarding_messages(
        &self,
        rewarded_set: &[RewardedNodeWithParams],
    ) -> Result<(), NyxdError> {
        // the expect is fine as we always construct the client with the mixnet contract explicitly set
        let mixnet_contract = self
            .query
            .mixnet_contract_address()
            .expect("mixnet contract address is not available")
            .clone();

        let msgs = self.generate_reward_messages(rewarded_set);

        // "technically" we don't need a write access to the client,
        // but we REALLY don't want to accidentally send any transactions while we're sending rewarding messages
        // as that would have messed up sequence numbers
        nyxd_signing!(
            self,
            execute_multiple(
                &mixnet_contract,
                msgs,
                Default::default(),
                format!("rewarding {} nodes", rewarded_set.len()),
            )
            .await?
        );
        Ok(())
    }

    fn generate_role_assignment_messages(
        &self,
        rewarded_set: RewardedSet,
    ) -> Vec<(ExecuteMsg, Vec<Coin>)> {
        // currently we just assign all of them together,
        // but the contract is ready to handle them separately should we need it
        // if the tx is too big
        let mut msgs = Vec::new();
        for (role, nodes) in [
            (Role::ExitGateway, rewarded_set.exit_gateways),
            (Role::EntryGateway, rewarded_set.entry_gateways),
            (Role::Layer1, rewarded_set.layer1),
            (Role::Layer2, rewarded_set.layer2),
            (Role::Layer3, rewarded_set.layer3),
            (Role::Standby, rewarded_set.standby),
        ] {
            msgs.push((
                ExecuteMsg::AssignRoles {
                    assignment: RoleAssignment { role, nodes },
                },
                Vec::new(),
            ));
        }
        msgs
    }

    pub(crate) async fn send_role_assignment_messages(
        &self,
        rewarded_set: RewardedSet,
    ) -> Result<(), NyxdError> {
        // the expect is fine as we always construct the client with the mixnet contract explicitly set
        let mixnet_contract = self
            .query
            .mixnet_contract_address()
            .expect("mixnet contract address is not available")
            .clone();

        let msgs = self.generate_role_assignment_messages(rewarded_set);

        // "technically" we don't need a write access to the client,
        // but we REALLY don't want to accidentally send any transactions while we're sending rewarding messages
        // as that would have messed up sequence numbers
        nyxd_signing!(
            self,
            execute_multiple(
                &mixnet_contract,
                msgs,
                Default::default(),
                "assigning all the rewarded set roles",
            )
            .await?
        );
        Ok(())
    }

    pub(crate) async fn reconcile_epoch_events(&self, limit: Option<u32>) -> Result<(), NyxdError> {
        nyxd_signing!(self, reconcile_epoch_events(limit, None).await?);
        Ok(())
    }

    pub(crate) async fn get_all_delegator_delegations(
        &self,
        delegation_owner: &AccountId,
    ) -> Result<Vec<Delegation>, NyxdError> {
        self.query
            .get_all_delegator_delegations(delegation_owner)
            .await
    }

    pub(crate) async fn get_address_balance(
        &self,
        address: &AccountId,
        denom: impl Into<String>,
    ) -> Result<Option<Coin>, NyxdError> {
        self.query.get_balance(&address, denom.into()).await
    }

    pub(crate) async fn get_last_performance_contract_submission(
        &self,
    ) -> Result<LastSubmission, NyxdError> {
        self.query.get_last_submission().await
    }

    pub(crate) async fn get_full_epoch_performance(
        &self,
        epoch_id: nym_mixnet_contract_common::EpochId,
    ) -> Result<Vec<NodePerformance>, NyxdError> {
        self.query.get_all_epoch_performance(epoch_id).await
    }

    /// Query the network-monitors contract for the full set of authorised orchestrators.
    ///
    /// This returns every orchestrator registered in the contract regardless of whether they
    /// have announced an identity key yet - callers are responsible for filtering entries with
    /// `identity_key == None`.
    pub(crate) async fn get_all_network_monitor_orchestrators(
        &self,
    ) -> Result<Vec<AuthorisedNetworkMonitorOrchestrator>, NyxdError> {
        Ok(self
            .query
            .get_network_monitor_orchestrators()
            .await?
            .authorised)
    }
}

fn construct_usable_ecash_api_clients(shares: Vec<ContractVKShare>) -> Vec<EcashApiClient> {
    let mut clients = Vec::with_capacity(shares.len());

    for share in shares {
        let owner = share.owner.clone();
        let epoch_id = share.epoch_id;

        match construct_ecash_api_client(share) {
            Ok(client) => clients.push(client),
            Err(err) => {
                warn!("ignoring the key share of {owner} for epoch {epoch_id}: {err}")
            }
        }
    }

    clients
}

pub(crate) fn construct_ecash_api_client(
    share: ContractVKShare,
) -> std::result::Result<EcashApiClient, EcashApiError> {
    if !share.verified {
        return Err(EcashApiError::UnverifiedShare);
    }

    let url_address = Url::parse(&share.announce_address)?;

    let api_client = nym_http_api_client::Client::builder(url_address)
        .map_err(|e| EcashApiError::ClientError(e.to_string()))?
        .with_timeout(Duration::from_secs(5))
        .with_user_agent(UserAgent::from(bin_info!()))
        .no_hickory_dns()
        .build()
        .map_err(|e| EcashApiError::ClientError(e.to_string()))?;

    Ok(EcashApiClient {
        api_client,
        verification_key: VerificationKeyAuth::try_from_bs58(&share.share)?,
        node_id: share.node_index,
        cosmos_address: share.owner.as_str().parse()?,
    })
}

#[async_trait]
impl crate::ecash::client::Client for Client {
    async fn address(&self) -> Result<AccountId, EcashError> {
        self.client_address()
            .await
            .ok_or(EcashError::ChainSignerNotEnabled)
    }

    async fn dkg_contract_address(&self) -> Result<AccountId, EcashError> {
        self.query
            .dkg_contract_address()
            .cloned()
            .ok_or_else(|| NyxdError::unavailable_contract_address("dkg contract").into())
    }

    async fn get_deposit(
        &self,
        deposit_id: DepositId,
    ) -> crate::ecash::error::Result<DepositResponse> {
        Ok(self.query.get_deposit(deposit_id).await?)
    }

    async fn get_proposal(
        &self,
        proposal_id: u64,
    ) -> crate::ecash::error::Result<ProposalResponse> {
        Ok(self.query.query_proposal(proposal_id).await?)
    }

    async fn list_proposals(&self) -> crate::ecash::error::Result<Vec<ProposalResponse>> {
        Ok(self.query.get_all_proposals().await?)
    }

    async fn get_vote(
        &self,
        proposal_id: u64,
        voter: String,
    ) -> crate::ecash::error::Result<VoteResponse> {
        Ok(self.query.query_vote(proposal_id, voter).await?)
    }

    // async fn propose_for_blacklist(
    //     &self,
    //     public_key: String,
    // ) -> crate::ecash::error::Result<ExecuteResult> {
    //     Ok(nyxd_signing!(
    //         self,
    //         propose_for_blacklist(public_key, None).await?
    //     ))
    // }

    async fn get_blacklisted_account(
        &self,
        public_key: String,
    ) -> crate::ecash::error::Result<BlacklistedAccountResponse> {
        Ok(self.query.get_blacklisted_account(public_key).await?)
    }

    async fn contract_state(&self) -> crate::ecash::error::Result<State> {
        Ok(self.query.get_state().await?)
    }

    async fn get_current_epoch(&self) -> crate::ecash::error::Result<Epoch> {
        Ok(self.query.get_current_epoch().await?)
    }

    async fn group_member(&self, addr: String) -> crate::ecash::error::Result<MemberResponse> {
        Ok(self.query.member(addr, None).await?)
    }

    async fn get_current_epoch_threshold(
        &self,
    ) -> crate::ecash::error::Result<Option<nym_dkg::Threshold>> {
        Ok(self.query.get_current_epoch_threshold().await?)
    }

    async fn get_epoch_threshold(
        &self,
        epoch_id: nym_coconut_dkg_common::types::EpochId,
    ) -> crate::ecash::error::Result<Option<Threshold>> {
        Ok(self.query.get_epoch_threshold(epoch_id).await?)
    }

    async fn get_self_registered_dealer_details(
        &self,
    ) -> crate::ecash::error::Result<DealerDetailsResponse> {
        let self_address = &self.address().await?;
        Ok(self.query.get_dealer_details(self_address).await?)
    }

    async fn get_registered_dealer_details(
        &self,
        epoch_id: nym_coconut_dkg_common::types::EpochId,
        dealer: String,
    ) -> crate::ecash::error::Result<RegisteredDealerDetails> {
        let dealer = dealer
            .as_str()
            .parse()
            .map_err(|_| NyxdError::MalformedAccountAddress(dealer))?;
        Ok(self
            .query
            .get_registered_dealer_details(&dealer, Some(epoch_id))
            .await?)
    }

    async fn get_dealer_dealings_status(
        &self,
        epoch_id: nym_coconut_dkg_common::types::EpochId,
        dealer: String,
    ) -> crate::ecash::error::Result<DealerDealingsStatusResponse> {
        Ok(self
            .query
            .get_dealer_dealings_status(epoch_id, dealer)
            .await?)
    }

    async fn get_dealing_status(
        &self,
        epoch_id: nym_coconut_dkg_common::types::EpochId,
        dealer: String,
        dealing_index: DealingIndex,
    ) -> crate::ecash::error::Result<DealingStatusResponse> {
        Ok(self
            .query
            .get_dealing_status(epoch_id, dealer, dealing_index)
            .await?)
    }

    async fn get_current_dealers(&self) -> crate::ecash::error::Result<Vec<DealerDetails>> {
        Ok(self.query.get_all_current_dealers().await?)
    }

    async fn get_dealing_metadata(
        &self,
        epoch_id: nym_coconut_dkg_common::types::EpochId,
        dealer: String,
        dealing_index: DealingIndex,
    ) -> crate::ecash::error::Result<Option<DealingMetadata>> {
        Ok(self
            .query
            .get_dealings_metadata(epoch_id, dealer, dealing_index)
            .await?
            .metadata)
    }

    async fn get_dealing_chunk(
        &self,
        epoch_id: nym_coconut_dkg_common::types::EpochId,
        dealer: &str,
        dealing_index: DealingIndex,
        chunk_index: ChunkIndex,
    ) -> crate::ecash::error::Result<Option<PartialContractDealingData>> {
        Ok(self
            .query
            .get_dealing_chunk(epoch_id, dealer.to_string(), dealing_index, chunk_index)
            .await?
            .chunk)
    }

    async fn get_verification_key_share(
        &self,
        epoch_id: nym_coconut_dkg_common::types::EpochId,
        dealer: String,
    ) -> Result<Option<ContractVKShare>, EcashError> {
        Ok(self.query.get_vk_share(epoch_id, dealer).await?.share)
    }

    async fn get_verification_key_shares(
        &self,
        epoch_id: nym_coconut_dkg_common::types::EpochId,
    ) -> Result<Vec<ContractVKShare>, EcashError> {
        Ok(self.query.get_all_verification_key_shares(epoch_id).await?)
    }

    async fn get_registered_ecash_clients(
        &self,
        epoch_id: nym_coconut_dkg_common::types::EpochId,
    ) -> Result<Vec<EcashApiClient>, EcashError> {
        Ok(construct_usable_ecash_api_clients(
            self.get_verification_key_shares(epoch_id).await?,
        ))
    }

    async fn vote_proposal(
        &self,
        proposal_id: u64,
        vote_yes: bool,
        fee: Option<Fee>,
    ) -> Result<(), EcashError> {
        nyxd_signing!(self, vote_proposal(proposal_id, vote_yes, fee).await?);
        Ok(())
    }

    async fn execute_proposal(&self, proposal_id: u64) -> crate::ecash::error::Result<()> {
        nyxd_signing!(self, execute_proposal(proposal_id, None).await?);
        Ok(())
    }

    async fn can_advance_epoch_state(&self) -> crate::ecash::error::Result<bool> {
        Ok(self.query.can_advance_state().await?.can_advance())
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
    ) -> Result<ExecuteResult, EcashError> {
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
    ) -> Result<ExecuteResult, EcashError> {
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
        self.query.query_dkg_contract(query).await
    }
}

#[async_trait]
impl NodeFamiliesQueryClient for Client {
    async fn query_node_families_contract<T>(
        &self,
        query: NodeFamiliesQueryMsg,
    ) -> std::result::Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        self.query.query_node_families_contract(query).await
    }
}
