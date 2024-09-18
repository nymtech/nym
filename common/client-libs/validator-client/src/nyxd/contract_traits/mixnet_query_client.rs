// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::collect_paged;
use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::error::NyxdError;
use crate::nyxd::CosmWasmClient;
use async_trait::async_trait;
use cosmrs::AccountId;
use nym_contracts_common::signing::Nonce;
use nym_mixnet_contract_common::{
    delegation,
    delegation::{MixNodeDelegationResponse, OwnerProxySubKey},
    families::{Family, FamilyHead},
    mixnode::{
        MixnodeRewardingDetailsResponse, PagedMixnodesDetailsResponse,
        PagedUnbondedMixnodesResponse, StakeSaturationResponse, UnbondedMixnodeResponse,
    },
    reward_params::{Performance, RewardingParams},
    rewarding::{EstimatedCurrentEpochRewardResponse, PendingRewardResponse},
    ContractBuildInformation, ContractState, ContractStateParams, CurrentIntervalResponse,
    Delegation, EpochEventId, EpochStatus, FamilyByHeadResponse, FamilyByLabelResponse,
    FamilyMembersByHeadResponse, FamilyMembersByLabelResponse, GatewayBond, GatewayBondResponse,
    GatewayOwnershipResponse, IdentityKey, IdentityKeyRef, IntervalEventId, LayerDistribution,
    MixId, MixNodeBond, MixNodeDetails, MixOwnershipResponse, MixnodeDetailsByIdentityResponse,
    MixnodeDetailsResponse, NumberOfPendingEventsResponse, PagedAllDelegationsResponse,
    PagedDelegatorDelegationsResponse, PagedFamiliesResponse, PagedGatewayResponse,
    PagedMembersResponse, PagedMixNodeDelegationsResponse, PagedMixnodeBondsResponse,
    PagedRewardedSetResponse, PendingEpochEvent, PendingEpochEventResponse,
    PendingEpochEventsResponse, PendingIntervalEvent, PendingIntervalEventResponse,
    PendingIntervalEventsResponse, QueryMsg as MixnetQueryMsg, RewardedSetNodeStatus,
    UnbondedMixnode,
};
use serde::Deserialize;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait MixnetQueryClient {
    async fn query_mixnet_contract<T>(&self, query: MixnetQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>;

    // state/sys-params-related

    async fn admin(&self) -> Result<cw_controllers::AdminResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::Admin {}).await
    }

    async fn get_mixnet_contract_version(&self) -> Result<ContractBuildInformation, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetContractVersion {})
            .await
    }

    async fn get_mixnet_contract_cw2_version(&self) -> Result<cw2::ContractVersion, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetCW2ContractVersion {})
            .await
    }

    async fn get_rewarding_validator_address(&self) -> Result<AccountId, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetRewardingValidatorAddress {})
            .await
    }

    async fn get_mixnet_contract_settings(&self) -> Result<ContractStateParams, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetStateParams {})
            .await
    }

    async fn get_mixnet_contract_state_params(&self) -> Result<ContractStateParams, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetStateParams {})
            .await
    }

    async fn get_mixnet_contract_state(&self) -> Result<ContractState, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetState {})
            .await
    }

    async fn get_rewarding_parameters(&self) -> Result<RewardingParams, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetRewardingParams {})
            .await
    }

    async fn get_current_epoch_status(&self) -> Result<EpochStatus, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetEpochStatus {})
            .await
    }

    async fn get_current_interval_details(&self) -> Result<CurrentIntervalResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetCurrentIntervalDetails {})
            .await
    }

    async fn get_rewarded_set_paged(
        &self,
        start_after: Option<MixId>,
        limit: Option<u32>,
    ) -> Result<PagedRewardedSetResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetRewardedSet { limit, start_after })
            .await
    }

    async fn get_all_node_families_paged(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<PagedFamiliesResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetAllFamiliesPaged { limit, start_after })
            .await
    }

    async fn get_all_family_members_paged(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<PagedMembersResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetAllMembersPaged { limit, start_after })
            .await
    }

    async fn get_family_members_by_head<S: Into<String> + Send>(
        &self,
        head: S,
    ) -> Result<FamilyMembersByHeadResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetFamilyMembersByHead { head: head.into() })
            .await
    }

    async fn get_family_members_by_label<S: Into<String> + Send>(
        &self,
        label: S,
    ) -> Result<FamilyMembersByLabelResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetFamilyMembersByLabel {
            label: label.into(),
        })
        .await
    }

    // mixnode-related:

    async fn get_mixnode_bonds_paged(
        &self,
        start_after: Option<MixId>,
        limit: Option<u32>,
    ) -> Result<PagedMixnodeBondsResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetMixNodeBonds { limit, start_after })
            .await
    }

    async fn get_mixnodes_detailed_paged(
        &self,
        start_after: Option<MixId>,
        limit: Option<u32>,
    ) -> Result<PagedMixnodesDetailsResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetMixNodesDetailed { limit, start_after })
            .await
    }

    async fn get_unbonded_paged(
        &self,
        start_after: Option<MixId>,
        limit: Option<u32>,
    ) -> Result<PagedUnbondedMixnodesResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetUnbondedMixNodes { limit, start_after })
            .await
    }

    async fn get_unbonded_by_owner_paged(
        &self,
        owner: &AccountId,
        start_after: Option<MixId>,
        limit: Option<u32>,
    ) -> Result<PagedUnbondedMixnodesResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetUnbondedMixNodesByOwner {
            owner: owner.to_string(),
            limit,
            start_after,
        })
        .await
    }

    async fn get_unbonded_by_identity_paged(
        &self,
        identity_key: IdentityKeyRef<'_>,
        start_after: Option<MixId>,
        limit: Option<u32>,
    ) -> Result<PagedUnbondedMixnodesResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetUnbondedMixNodesByIdentityKey {
            identity_key: identity_key.to_string(),
            limit,
            start_after,
        })
        .await
    }

    async fn get_owned_mixnode(
        &self,
        address: &AccountId,
    ) -> Result<MixOwnershipResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetOwnedMixnode {
            address: address.to_string(),
        })
        .await
    }

    async fn get_mixnode_details(
        &self,
        mix_id: MixId,
    ) -> Result<MixnodeDetailsResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetMixnodeDetails { mix_id })
            .await
    }

    async fn get_mixnode_details_by_identity(
        &self,
        mix_identity: IdentityKey,
    ) -> Result<MixnodeDetailsByIdentityResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetBondedMixnodeDetailsByIdentity {
            mix_identity,
        })
        .await
    }

    async fn get_mixnode_rewarding_details(
        &self,
        mix_id: MixId,
    ) -> Result<MixnodeRewardingDetailsResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetMixnodeRewardingDetails { mix_id })
            .await
    }

    async fn get_mixnode_stake_saturation(
        &self,
        mix_id: MixId,
    ) -> Result<StakeSaturationResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetStakeSaturation { mix_id })
            .await
    }

    async fn get_unbonded_mixnode_information(
        &self,
        mix_id: MixId,
    ) -> Result<UnbondedMixnodeResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetUnbondedMixNodeInformation { mix_id })
            .await
    }

    async fn get_layer_distribution(&self) -> Result<LayerDistribution, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetLayerDistribution {})
            .await
    }

    // gateway-related:

    async fn get_gateways_paged(
        &self,
        start_after: Option<IdentityKey>,
        limit: Option<u32>,
    ) -> Result<PagedGatewayResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetGateways { start_after, limit })
            .await
    }

    /// Checks whether there is a bonded gateway associated with the provided identity key
    async fn get_gateway_bond(
        &self,
        identity: IdentityKey,
    ) -> Result<GatewayBondResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetGatewayBond { identity })
            .await
    }

    /// Checks whether there is a bonded gateway associated with the provided client's address
    async fn get_owned_gateway(
        &self,
        address: &AccountId,
    ) -> Result<GatewayOwnershipResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetOwnedGateway {
            address: address.to_string(),
        })
        .await
    }

    // delegation-related:

    /// Gets list of all delegations towards particular mixnode on particular page.
    async fn get_mixnode_delegations_paged(
        &self,
        mix_id: MixId,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<PagedMixNodeDelegationsResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetMixnodeDelegations {
            mix_id,
            start_after,
            limit,
        })
        .await
    }

    /// Gets list of all the mixnodes to which a particular address delegated.
    async fn get_delegator_delegations_paged(
        &self,
        delegator: &AccountId,
        start_after: Option<(MixId, OwnerProxySubKey)>,
        limit: Option<u32>,
    ) -> Result<PagedDelegatorDelegationsResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetDelegatorDelegations {
            delegator: delegator.to_string(),
            start_after,
            limit,
        })
        .await
    }

    /// Checks value of delegation of given client towards particular mixnode.
    async fn get_delegation_details(
        &self,
        mix_id: MixId,
        delegator: &AccountId,
        proxy: Option<String>,
    ) -> Result<MixNodeDelegationResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetDelegationDetails {
            mix_id,
            delegator: delegator.to_string(),
            proxy,
        })
        .await
    }

    /// Gets all the delegations on the entire network
    async fn get_all_network_delegations_paged(
        &self,
        start_after: Option<delegation::StorageKey>,
        limit: Option<u32>,
    ) -> Result<PagedAllDelegationsResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetAllDelegations { start_after, limit })
            .await
    }

    // rewards related
    async fn get_pending_operator_reward(
        &self,
        operator: &AccountId,
    ) -> Result<PendingRewardResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetPendingOperatorReward {
            address: operator.to_string(),
        })
        .await
    }

    async fn get_pending_mixnode_operator_reward(
        &self,
        mix_id: MixId,
    ) -> Result<PendingRewardResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetPendingMixNodeOperatorReward { mix_id })
            .await
    }

    async fn get_pending_delegator_reward(
        &self,
        delegator: &AccountId,
        mix_id: MixId,
        proxy: Option<String>,
    ) -> Result<PendingRewardResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetPendingDelegatorReward {
            address: delegator.to_string(),
            mix_id,
            proxy,
        })
        .await
    }

    // given the provided performance, estimate the reward at the end of the current epoch
    async fn get_estimated_current_epoch_operator_reward(
        &self,
        mix_id: MixId,
        estimated_performance: Performance,
    ) -> Result<EstimatedCurrentEpochRewardResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetEstimatedCurrentEpochOperatorReward {
            mix_id,
            estimated_performance,
        })
        .await
    }

    // given the provided performance, estimate the reward at the end of the current epoch
    async fn get_estimated_current_epoch_delegator_reward(
        &self,
        delegator: &AccountId,
        mix_id: MixId,
        proxy: Option<String>,
        estimated_performance: Performance,
    ) -> Result<EstimatedCurrentEpochRewardResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetEstimatedCurrentEpochDelegatorReward {
            address: delegator.to_string(),
            mix_id,
            proxy,
            estimated_performance,
        })
        .await
    }

    // interval-related

    async fn get_pending_epoch_events_paged(
        &self,
        start_after: Option<EpochEventId>,
        limit: Option<u32>,
    ) -> Result<PendingEpochEventsResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetPendingEpochEvents { start_after, limit })
            .await
    }

    async fn get_pending_interval_events_paged(
        &self,
        start_after: Option<IntervalEventId>,
        limit: Option<u32>,
    ) -> Result<PendingIntervalEventsResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetPendingIntervalEvents { start_after, limit })
            .await
    }

    async fn get_pending_epoch_event(
        &self,
        event_id: EpochEventId,
    ) -> Result<PendingEpochEventResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetPendingEpochEvent { event_id })
            .await
    }

    async fn get_pending_interval_event(
        &self,
        event_id: IntervalEventId,
    ) -> Result<PendingIntervalEventResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetPendingIntervalEvent { event_id })
            .await
    }

    async fn get_number_of_pending_events(
        &self,
    ) -> Result<NumberOfPendingEventsResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetNumberOfPendingEvents {})
            .await
    }

    async fn get_signing_nonce(&self, address: &AccountId) -> Result<Nonce, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetSigningNonce {
            address: address.to_string(),
        })
        .await
    }

    async fn get_node_family_by_label(
        &self,
        label: String,
    ) -> Result<FamilyByLabelResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetFamilyByLabel { label })
            .await
    }

    async fn get_node_family_by_head(
        &self,
        head: String,
    ) -> Result<FamilyByHeadResponse, NyxdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetFamilyByHead { head })
            .await
    }
}

// extension trait to the query client to deal with the paged queries
// (it didn't feel appropriate to combine it with the existing trait
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait PagedMixnetQueryClient: MixnetQueryClient {
    async fn get_all_node_families(&self) -> Result<Vec<Family>, NyxdError> {
        collect_paged!(self, get_all_node_families_paged, families)
    }

    async fn get_all_family_members(&self) -> Result<Vec<(IdentityKey, FamilyHead)>, NyxdError> {
        collect_paged!(self, get_all_family_members_paged, members)
    }

    async fn get_all_rewarded_set_mixnodes(
        &self,
    ) -> Result<Vec<(MixId, RewardedSetNodeStatus)>, NyxdError> {
        collect_paged!(self, get_rewarded_set_paged, nodes)
    }

    async fn get_all_mixnode_bonds(&self) -> Result<Vec<MixNodeBond>, NyxdError> {
        collect_paged!(self, get_mixnode_bonds_paged, nodes)
    }

    async fn get_all_mixnodes_detailed(&self) -> Result<Vec<MixNodeDetails>, NyxdError> {
        collect_paged!(self, get_mixnodes_detailed_paged, nodes)
    }

    async fn get_all_unbonded_mixnodes(&self) -> Result<Vec<(MixId, UnbondedMixnode)>, NyxdError> {
        collect_paged!(self, get_unbonded_paged, nodes)
    }

    async fn get_all_unbonded_mixnodes_by_owner(
        &self,
        owner: &AccountId,
    ) -> Result<Vec<(MixId, UnbondedMixnode)>, NyxdError> {
        collect_paged!(self, get_unbonded_by_owner_paged, nodes, owner)
    }

    async fn get_all_unbonded_mixnodes_by_identity(
        &self,
        identity_key: IdentityKeyRef<'_>,
    ) -> Result<Vec<(MixId, UnbondedMixnode)>, NyxdError> {
        collect_paged!(self, get_unbonded_by_identity_paged, nodes, identity_key)
    }

    async fn get_all_gateways(&self) -> Result<Vec<GatewayBond>, NyxdError> {
        collect_paged!(self, get_gateways_paged, nodes)
    }

    async fn get_all_single_mixnode_delegations(
        &self,
        mix_id: MixId,
    ) -> Result<Vec<Delegation>, NyxdError> {
        collect_paged!(self, get_mixnode_delegations_paged, delegations, mix_id)
    }

    async fn get_all_delegator_delegations(
        &self,
        delegation_owner: &AccountId,
    ) -> Result<Vec<Delegation>, NyxdError> {
        collect_paged!(
            self,
            get_delegator_delegations_paged,
            delegations,
            delegation_owner
        )
    }

    async fn get_all_network_delegations(&self) -> Result<Vec<Delegation>, NyxdError> {
        collect_paged!(self, get_all_network_delegations_paged, delegations)
    }

    async fn get_all_pending_epoch_events(&self) -> Result<Vec<PendingEpochEvent>, NyxdError> {
        collect_paged!(self, get_pending_epoch_events_paged, events)
    }

    async fn get_all_pending_interval_events(
        &self,
    ) -> Result<Vec<PendingIntervalEvent>, NyxdError> {
        collect_paged!(self, get_pending_interval_events_paged, events)
    }
}

#[async_trait]
impl<T> PagedMixnetQueryClient for T where T: MixnetQueryClient {}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> MixnetQueryClient for C
where
    C: CosmWasmClient + NymContractsProvider + Send + Sync,
{
    async fn query_mixnet_contract<T>(&self, query: MixnetQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        let mixnet_contract_address = &self
            .mixnet_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("mixnet contract"))?;
        self.query_contract_smart(mixnet_contract_address, &query)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nyxd::contract_traits::tests::IgnoreValue;

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_query_variants_are_covered<C: MixnetQueryClient + Send + Sync>(
        client: C,
        msg: MixnetQueryMsg,
    ) -> u32 {
        match msg {
            MixnetQueryMsg::Admin {} => client.admin().ignore(),
            MixnetQueryMsg::GetAllFamiliesPaged { limit, start_after } => client
                .get_all_family_members_paged(start_after, limit)
                .ignore(),
            MixnetQueryMsg::GetAllMembersPaged { limit, start_after } => client
                .get_all_family_members_paged(start_after, limit)
                .ignore(),
            MixnetQueryMsg::GetFamilyByHead { head } => {
                client.get_node_family_by_head(head).ignore()
            }
            MixnetQueryMsg::GetFamilyByLabel { label } => {
                client.get_node_family_by_label(label).ignore()
            }
            MixnetQueryMsg::GetFamilyMembersByHead { head } => {
                client.get_family_members_by_head(head).ignore()
            }
            MixnetQueryMsg::GetFamilyMembersByLabel { label } => {
                client.get_family_members_by_label(label).ignore()
            }
            MixnetQueryMsg::GetContractVersion {} => client.get_mixnet_contract_version().ignore(),
            MixnetQueryMsg::GetCW2ContractVersion {} => {
                client.get_mixnet_contract_cw2_version().ignore()
            }
            MixnetQueryMsg::GetRewardingValidatorAddress {} => {
                client.get_rewarding_validator_address().ignore()
            }
            MixnetQueryMsg::GetStateParams {} => client.get_mixnet_contract_state_params().ignore(),
            MixnetQueryMsg::GetState {} => client.get_mixnet_contract_state().ignore(),
            MixnetQueryMsg::GetRewardingParams {} => client.get_rewarding_parameters().ignore(),
            MixnetQueryMsg::GetEpochStatus {} => client.get_current_epoch_status().ignore(),
            MixnetQueryMsg::GetCurrentIntervalDetails {} => {
                client.get_current_interval_details().ignore()
            }
            MixnetQueryMsg::GetRewardedSet { limit, start_after } => {
                client.get_rewarded_set_paged(start_after, limit).ignore()
            }
            MixnetQueryMsg::GetMixNodeBonds { limit, start_after } => {
                client.get_mixnode_bonds_paged(start_after, limit).ignore()
            }
            MixnetQueryMsg::GetMixNodesDetailed { limit, start_after } => client
                .get_mixnodes_detailed_paged(start_after, limit)
                .ignore(),
            MixnetQueryMsg::GetUnbondedMixNodes { limit, start_after } => {
                client.get_unbonded_paged(start_after, limit).ignore()
            }
            MixnetQueryMsg::GetUnbondedMixNodesByOwner {
                owner,
                limit,
                start_after,
            } => client
                .get_unbonded_by_owner_paged(&owner.parse().unwrap(), start_after, limit)
                .ignore(),
            MixnetQueryMsg::GetUnbondedMixNodesByIdentityKey {
                identity_key,
                limit,
                start_after,
            } => client
                .get_unbonded_by_identity_paged(&identity_key, start_after, limit)
                .ignore(),
            MixnetQueryMsg::GetOwnedMixnode { address } => {
                client.get_owned_mixnode(&address.parse().unwrap()).ignore()
            }
            MixnetQueryMsg::GetMixnodeDetails { mix_id } => {
                client.get_mixnode_details(mix_id).ignore()
            }
            MixnetQueryMsg::GetMixnodeRewardingDetails { mix_id } => {
                client.get_mixnode_rewarding_details(mix_id).ignore()
            }
            MixnetQueryMsg::GetStakeSaturation { mix_id } => {
                client.get_mixnode_stake_saturation(mix_id).ignore()
            }
            MixnetQueryMsg::GetUnbondedMixNodeInformation { mix_id } => {
                client.get_unbonded_mixnode_information(mix_id).ignore()
            }
            MixnetQueryMsg::GetBondedMixnodeDetailsByIdentity { mix_identity } => client
                .get_mixnode_details_by_identity(mix_identity)
                .ignore(),
            MixnetQueryMsg::GetLayerDistribution {} => client.get_layer_distribution().ignore(),
            MixnetQueryMsg::GetGateways { start_after, limit } => {
                client.get_gateways_paged(start_after, limit).ignore()
            }
            MixnetQueryMsg::GetGatewayBond { identity } => {
                client.get_gateway_bond(identity).ignore()
            }
            MixnetQueryMsg::GetOwnedGateway { address } => {
                client.get_owned_gateway(&address.parse().unwrap()).ignore()
            }
            MixnetQueryMsg::GetMixnodeDelegations {
                mix_id,
                start_after,
                limit,
            } => client
                .get_mixnode_delegations_paged(mix_id, start_after, limit)
                .ignore(),
            MixnetQueryMsg::GetDelegatorDelegations {
                delegator,
                start_after,
                limit,
            } => client
                .get_delegator_delegations_paged(&delegator.parse().unwrap(), start_after, limit)
                .ignore(),
            MixnetQueryMsg::GetDelegationDetails {
                mix_id,
                delegator,
                proxy,
            } => client
                .get_delegation_details(mix_id, &delegator.parse().unwrap(), proxy)
                .ignore(),
            MixnetQueryMsg::GetAllDelegations { start_after, limit } => client
                .get_all_network_delegations_paged(start_after, limit)
                .ignore(),
            MixnetQueryMsg::GetPendingOperatorReward { address } => client
                .get_pending_operator_reward(&address.parse().unwrap())
                .ignore(),
            MixnetQueryMsg::GetPendingMixNodeOperatorReward { mix_id } => {
                client.get_pending_mixnode_operator_reward(mix_id).ignore()
            }
            MixnetQueryMsg::GetPendingDelegatorReward {
                address,
                mix_id,
                proxy,
            } => client
                .get_pending_delegator_reward(&address.parse().unwrap(), mix_id, proxy)
                .ignore(),
            MixnetQueryMsg::GetEstimatedCurrentEpochOperatorReward {
                mix_id,
                estimated_performance,
            } => client
                .get_estimated_current_epoch_operator_reward(mix_id, estimated_performance)
                .ignore(),
            MixnetQueryMsg::GetEstimatedCurrentEpochDelegatorReward {
                address,
                mix_id,
                proxy,
                estimated_performance,
            } => client
                .get_estimated_current_epoch_delegator_reward(
                    &address.parse().unwrap(),
                    mix_id,
                    proxy,
                    estimated_performance,
                )
                .ignore(),
            MixnetQueryMsg::GetPendingEpochEvents { limit, start_after } => client
                .get_pending_epoch_events_paged(start_after, limit)
                .ignore(),
            MixnetQueryMsg::GetPendingIntervalEvents { limit, start_after } => client
                .get_pending_interval_events_paged(start_after, limit)
                .ignore(),
            MixnetQueryMsg::GetPendingEpochEvent { event_id } => {
                client.get_pending_epoch_event(event_id).ignore()
            }
            MixnetQueryMsg::GetPendingIntervalEvent { event_id } => {
                client.get_pending_interval_event(event_id).ignore()
            }
            MixnetQueryMsg::GetNumberOfPendingEvents {} => {
                client.get_number_of_pending_events().ignore()
            }
            MixnetQueryMsg::GetSigningNonce { address } => {
                client.get_signing_nonce(&address.parse().unwrap()).ignore()
            }
        }
    }
}
