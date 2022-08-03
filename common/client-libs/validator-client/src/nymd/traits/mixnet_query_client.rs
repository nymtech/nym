// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use crate::nymd::cosmwasm_client::client::CosmWasmClient;
use crate::nymd::error::NymdError;
use crate::nymd::NymdClient;
use async_trait::async_trait;
use cosmrs::AccountId;
use mixnet_contract_common::delegation::{MixNodeDelegationResponse, OwnerProxySubKey};
use mixnet_contract_common::mixnode::{
    MixNodeDetails, MixnodeRewardingDetailsResponse, PagedMixnodesDetailsResponse,
    PagedUnbondedMixnodesResponse, StakeSaturationResponse, UnbondedMixnodeResponse,
};
use mixnet_contract_common::reward_params::{Performance, RewardingParams};
use mixnet_contract_common::rewarding::{
    EstimatedCurrentEpochRewardResponse, PendingRewardResponse,
};
use mixnet_contract_common::{
    delegation, ContractBuildInformation, ContractState, ContractStateParams,
    CurrentIntervalResponse, EpochEventId, GatewayBondResponse, GatewayOwnershipResponse,
    IdentityKey, IntervalEventId, LayerDistribution, MixOwnershipResponse, MixnodeDetailsResponse,
    NodeId, PagedAllDelegationsResponse, PagedDelegatorDelegationsResponse, PagedGatewayResponse,
    PagedMixNodeDelegationsResponse, PagedMixnodeBondsResponse, PagedRewardedSetResponse,
    PendingEpochEventsResponse, PendingIntervalEventsResponse, QueryMsg as MixnetQueryMsg,
};
use serde::Deserialize;

#[async_trait]
pub trait MixnetQueryClient {
    async fn query_mixnet_contract<T>(&self, query: MixnetQueryMsg) -> Result<T, NymdError>
    where
        for<'a> T: Deserialize<'a>;

    // state/sys-params-related

    async fn get_mixnet_contract_version(&self) -> Result<ContractBuildInformation, NymdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetContractVersion {})
            .await
    }

    async fn get_rewarding_validator_address(&self) -> Result<AccountId, NymdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetRewardingValidatorAddress {})
            .await
    }

    async fn get_mixnet_contract_settings(&self) -> Result<ContractStateParams, NymdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetStateParams {})
            .await
    }

    async fn get_mixnet_contract_state(&self) -> Result<ContractState, NymdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetState {})
            .await
    }

    async fn get_rewarding_parameters(&self) -> Result<RewardingParams, NymdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetRewardingParams {})
            .await
    }

    async fn get_current_interval_details(&self) -> Result<CurrentIntervalResponse, NymdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetCurrentIntervalDetails {})
            .await
    }

    async fn get_rewarded_set_paged(
        &self,
        start_after: Option<NodeId>,
        limit: Option<u32>,
    ) -> Result<PagedRewardedSetResponse, NymdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetRewardedSet { limit, start_after })
            .await
    }

    // mixnode-related:

    async fn get_mixnode_bonds_paged(
        &self,
        limit: Option<u32>,
        start_after: Option<NodeId>,
    ) -> Result<PagedMixnodeBondsResponse, NymdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetMixNodeBonds { limit, start_after })
            .await
    }

    async fn get_mixnodes_detailed_paged(
        &self,
        limit: Option<u32>,
        start_after: Option<NodeId>,
    ) -> Result<PagedMixnodesDetailsResponse, NymdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetMixNodesDetailed { limit, start_after })
            .await
    }

    async fn get_unbonded_paged(
        &self,
        limit: Option<u32>,
        start_after: Option<NodeId>,
    ) -> Result<PagedUnbondedMixnodesResponse, NymdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetUnbondedMixNodes { limit, start_after })
            .await
    }

    async fn get_owned_mixnode(
        &self,
        address: &AccountId,
    ) -> Result<MixOwnershipResponse, NymdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetOwnedMixnode {
            address: address.to_string(),
        })
        .await
    }

    async fn get_mixnode_details(
        &self,
        mix_id: NodeId,
    ) -> Result<MixnodeDetailsResponse, NymdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetMixnodeDetails { mix_id })
            .await
    }

    async fn get_mixnode_rewarding_details(
        &self,
        mix_id: NodeId,
    ) -> Result<MixnodeRewardingDetailsResponse, NymdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetMixnodeRewardingDetails { mix_id })
            .await
    }

    async fn get_mixnode_stake_saturation(
        &self,
        mix_id: NodeId,
    ) -> Result<StakeSaturationResponse, NymdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetStakeSaturation { mix_id })
            .await
    }

    async fn get_unbonded_mixnode_information(
        &self,
        mix_id: NodeId,
    ) -> Result<UnbondedMixnodeResponse, NymdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetUnbondedMixNodeInformation { mix_id })
            .await
    }

    async fn get_layer_distribution(&self) -> Result<LayerDistribution, NymdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetLayerDistribution {})
            .await
    }

    // gateway-related:

    async fn get_gateways_paged(
        &self,
        start_after: Option<IdentityKey>,
        limit: Option<u32>,
    ) -> Result<PagedGatewayResponse, NymdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetGateways { start_after, limit })
            .await
    }

    /// Checks whether there is a bonded gateway associated with the provided identity key
    async fn get_gateway_bond(
        &self,
        identity: IdentityKey,
    ) -> Result<GatewayBondResponse, NymdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetGatewayBond { identity })
            .await
    }

    /// Checks whether there is a bonded gateway associated with the provided client's address
    async fn get_owned_gateway(
        &self,
        address: &AccountId,
    ) -> Result<GatewayOwnershipResponse, NymdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetOwnedGateway {
            address: address.to_string(),
        })
        .await
    }

    // delegation-related:

    /// Gets list of all delegations towards particular mixnode on particular page.
    async fn get_mixnode_delegations_paged(
        &self,
        mix_id: NodeId,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<PagedMixNodeDelegationsResponse, NymdError> {
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
        delegator: String,
        start_after: Option<(NodeId, OwnerProxySubKey)>,
        limit: Option<u32>,
    ) -> Result<PagedDelegatorDelegationsResponse, NymdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetDelegatorDelegations {
            delegator,
            start_after,
            limit,
        })
        .await
    }

    /// Checks value of delegation of given client towards particular mixnode.
    async fn get_delegation_details(
        &self,
        mix_id: NodeId,
        delegator: &AccountId,
        proxy: Option<String>,
    ) -> Result<MixNodeDelegationResponse, NymdError> {
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
    ) -> Result<PagedAllDelegationsResponse, NymdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetAllDelegations { start_after, limit })
            .await
    }

    // rewards related
    async fn get_pending_operator_reward(
        &self,
        operator: &AccountId,
    ) -> Result<PendingRewardResponse, NymdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetPendingOperatorReward {
            address: operator.to_string(),
        })
        .await
    }

    async fn get_pending_mixnode_operator_reward(
        &self,
        mix_id: NodeId,
    ) -> Result<PendingRewardResponse, NymdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetPendingMixNodeOperatorReward { mix_id })
            .await
    }

    async fn get_pending_delegator_reward(
        &self,
        delegator: &AccountId,
        mix_id: NodeId,
        proxy: Option<String>,
    ) -> Result<PendingRewardResponse, NymdError> {
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
        mix_id: NodeId,
        estimated_performance: Performance,
    ) -> Result<EstimatedCurrentEpochRewardResponse, NymdError> {
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
        mix_id: NodeId,
        proxy: Option<String>,
        estimated_performance: Performance,
    ) -> Result<EstimatedCurrentEpochRewardResponse, NymdError> {
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
    ) -> Result<PendingEpochEventsResponse, NymdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetPendingEpochEvents { start_after, limit })
            .await
    }

    async fn get_pending_interval_events_paged(
        &self,
        start_after: Option<IntervalEventId>,
        limit: Option<u32>,
    ) -> Result<PendingIntervalEventsResponse, NymdError> {
        self.query_mixnet_contract(MixnetQueryMsg::GetPendingIntervalEvents { start_after, limit })
            .await
    }

    // DEPRECATED AND ONLY HERE FOR THE BACKWARDS COMPATIBILITY:

    #[deprecated(
        note = "deprecated since mixnet v2; please query for mixnodes by their NodeId instead. This method will be removed soon."
    )]
    async fn get_mixnode_details_by_identity(
        &self,
        mix_identity: IdentityKey,
    ) -> Result<Option<MixNodeDetails>, NymdError> {
        self.query_mixnet_contract(MixnetQueryMsg::DeprecatedGetMixnodeDetailsByIdentity {
            mix_identity,
        })
        .await
    }
}

#[async_trait]
impl<C> MixnetQueryClient for NymdClient<C>
where
    C: CosmWasmClient + Sync + Send,
{
    async fn query_mixnet_contract<T>(&self, query: MixnetQueryMsg) -> Result<T, NymdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        self.client
            .query_contract_smart(self.mixnet_contract_address(), &query)
            .await
    }
}

#[async_trait]
impl<C> MixnetQueryClient for crate::Client<C>
where
    C: CosmWasmClient + Sync + Send,
{
    async fn query_mixnet_contract<T>(&self, query: MixnetQueryMsg) -> Result<T, NymdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        self.nymd.query_mixnet_contract(query).await
    }
}
