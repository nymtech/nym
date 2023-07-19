// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::delegation::{self, OwnerProxySubKey};
use crate::error::MixnetContractError;
use crate::families::FamilyHead;
use crate::gateway::{Gateway, GatewayConfigUpdate};
use crate::helpers::IntoBaseDecimal;
use crate::mixnode::{Layer, MixNode, MixNodeConfigUpdate, MixNodeCostParams};
use crate::pending_events::{EpochEventId, IntervalEventId};
use crate::reward_params::{
    IntervalRewardParams, IntervalRewardingParamsUpdate, Performance, RewardingParams,
};

use crate::types::{ContractStateParams, LayerAssignment, MixId};
use contracts_common::{signing::MessageSignature, IdentityKey, Percent};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Decimal};
use std::time::Duration;

#[cfg(feature = "schema")]
use crate::{
    delegation::{
        MixNodeDelegationResponse, PagedAllDelegationsResponse, PagedDelegatorDelegationsResponse,
        PagedMixNodeDelegationsResponse,
    },
    families::{Family, PagedFamiliesResponse, PagedMembersResponse},
    gateway::{GatewayBondResponse, GatewayOwnershipResponse, PagedGatewayResponse},
    interval::{CurrentIntervalResponse, EpochStatus},
    mixnode::{
        MixNodeDetails, MixOwnershipResponse, MixnodeDetailsResponse,
        MixnodeRewardingDetailsResponse, PagedMixnodeBondsResponse, PagedMixnodesDetailsResponse,
        PagedUnbondedMixnodesResponse, StakeSaturationResponse, UnbondedMixnodeResponse,
    },
    pending_events::{
        NumberOfPendingEventsResponse, PendingEpochEventResponse, PendingEpochEventsResponse,
        PendingIntervalEventResponse, PendingIntervalEventsResponse,
    },
    rewarding::{
        EstimatedCurrentEpochRewardResponse, PagedRewardedSetResponse, PendingRewardResponse,
    },
    types::{ContractState, LayerDistribution},
};
#[cfg(feature = "schema")]
use contracts_common::{signing::Nonce, ContractBuildInformation};
#[cfg(feature = "schema")]
use cosmwasm_schema::QueryResponses;
#[cfg(feature = "schema")]
use std::collections::HashSet;

#[cw_serde]
pub struct InstantiateMsg {
    pub rewarding_validator_address: String,
    pub vesting_contract_address: String,

    pub rewarding_denom: String,
    pub epochs_in_interval: u32,
    pub epoch_duration: Duration,
    pub initial_rewarding_params: InitialRewardingParams,
}

#[cw_serde]
pub struct InitialRewardingParams {
    pub initial_reward_pool: Decimal,
    pub initial_staking_supply: Decimal,

    pub staking_supply_scale_factor: Percent,
    pub sybil_resistance: Percent,
    pub active_set_work_factor: Decimal,
    pub interval_pool_emission: Percent,

    pub rewarded_set_size: u32,
    pub active_set_size: u32,
}

impl InitialRewardingParams {
    pub fn into_rewarding_params(
        self,
        epochs_in_interval: u32,
    ) -> Result<RewardingParams, MixnetContractError> {
        let epoch_reward_budget = self.initial_reward_pool
            / epochs_in_interval.into_base_decimal()?
            * self.interval_pool_emission;
        let stake_saturation_point =
            self.initial_staking_supply / self.rewarded_set_size.into_base_decimal()?;

        Ok(RewardingParams {
            interval: IntervalRewardParams {
                reward_pool: self.initial_reward_pool,
                staking_supply: self.initial_staking_supply,
                staking_supply_scale_factor: self.staking_supply_scale_factor,
                epoch_reward_budget,
                stake_saturation_point,
                sybil_resistance: self.sybil_resistance,
                active_set_work_factor: self.active_set_work_factor,
                interval_pool_emission: self.interval_pool_emission,
            },
            rewarded_set_size: self.rewarded_set_size,
            active_set_size: self.active_set_size,
        })
    }
}

#[cw_serde]
pub enum ExecuteMsg {
    AssignNodeLayer {
        mix_id: MixId,
        layer: Layer,
    },
    // Families
    /// Only owner of the node can crate the family with node as head
    CreateFamily {
        label: String,
    },
    /// Family head needs to sign the joining node IdentityKey
    JoinFamily {
        join_permit: MessageSignature,
        family_head: FamilyHead,
    },
    LeaveFamily {
        family_head: FamilyHead,
    },
    KickFamilyMember {
        member: IdentityKey,
    },
    CreateFamilyOnBehalf {
        owner_address: String,
        label: String,
    },
    /// Family head needs to sign the joining node IdentityKey, MixNode needs to provide its signature proving that it wants to join the family
    JoinFamilyOnBehalf {
        member_address: String,
        join_permit: MessageSignature,
        family_head: FamilyHead,
    },
    LeaveFamilyOnBehalf {
        member_address: String,
        family_head: FamilyHead,
    },
    KickFamilyMemberOnBehalf {
        head_address: String,
        member: IdentityKey,
    },

    // state/sys-params-related
    UpdateRewardingValidatorAddress {
        address: String,
    },
    UpdateContractStateParams {
        updated_parameters: ContractStateParams,
    },
    UpdateActiveSetSize {
        active_set_size: u32,
        force_immediately: bool,
    },
    UpdateRewardingParams {
        updated_params: IntervalRewardingParamsUpdate,
        force_immediately: bool,
    },
    UpdateIntervalConfig {
        epochs_in_interval: u32,
        epoch_duration_secs: u64,
        force_immediately: bool,
    },
    BeginEpochTransition {},
    AdvanceCurrentEpoch {
        new_rewarded_set: Vec<LayerAssignment>,
        // families_in_layer: HashMap<String, Layer>,
        expected_active_set_size: u32,
    },
    ReconcileEpochEvents {
        limit: Option<u32>,
    },

    // mixnode-related:
    BondMixnode {
        mix_node: MixNode,
        cost_params: MixNodeCostParams,
        owner_signature: MessageSignature,
    },
    BondMixnodeOnBehalf {
        mix_node: MixNode,
        cost_params: MixNodeCostParams,
        owner_signature: MessageSignature,
        owner: String,
    },
    PledgeMore {},
    PledgeMoreOnBehalf {
        owner: String,
    },
    DecreasePledge {
        decrease_by: Coin,
    },
    DecreasePledgeOnBehalf {
        owner: String,
        decrease_by: Coin,
    },
    UnbondMixnode {},
    UnbondMixnodeOnBehalf {
        owner: String,
    },
    UpdateMixnodeCostParams {
        new_costs: MixNodeCostParams,
    },
    UpdateMixnodeCostParamsOnBehalf {
        new_costs: MixNodeCostParams,
        owner: String,
    },
    UpdateMixnodeConfig {
        new_config: MixNodeConfigUpdate,
    },
    UpdateMixnodeConfigOnBehalf {
        new_config: MixNodeConfigUpdate,
        owner: String,
    },

    // gateway-related:
    BondGateway {
        gateway: Gateway,
        owner_signature: MessageSignature,
    },
    BondGatewayOnBehalf {
        gateway: Gateway,
        owner: String,
        owner_signature: MessageSignature,
    },
    UnbondGateway {},
    UnbondGatewayOnBehalf {
        owner: String,
    },
    UpdateGatewayConfig {
        new_config: GatewayConfigUpdate,
    },
    UpdateGatewayConfigOnBehalf {
        new_config: GatewayConfigUpdate,
        owner: String,
    },

    // delegation-related:
    DelegateToMixnode {
        mix_id: MixId,
    },
    DelegateToMixnodeOnBehalf {
        mix_id: MixId,
        delegate: String,
    },
    UndelegateFromMixnode {
        mix_id: MixId,
    },
    UndelegateFromMixnodeOnBehalf {
        mix_id: MixId,
        delegate: String,
    },

    // reward-related
    RewardMixnode {
        mix_id: MixId,
        performance: Performance,
    },
    WithdrawOperatorReward {},
    WithdrawOperatorRewardOnBehalf {
        owner: String,
    },
    WithdrawDelegatorReward {
        mix_id: MixId,
    },
    WithdrawDelegatorRewardOnBehalf {
        mix_id: MixId,
        owner: String,
    },

    // testing-only
    #[cfg(feature = "contract-testing")]
    TestingResolveAllPendingEvents {
        limit: Option<u32>,
    },
}

impl ExecuteMsg {
    pub fn default_memo(&self) -> String {
        match self {
            ExecuteMsg::AssignNodeLayer { mix_id, layer } => {
                format!("assigning mix {mix_id} for layer {layer:?}")
            }
            ExecuteMsg::CreateFamily { .. } => "crating node family with".to_string(),
            ExecuteMsg::JoinFamily { family_head, .. } => {
                format!("joining family {family_head}")
            }
            ExecuteMsg::LeaveFamily { family_head, .. } => {
                format!("leaving family {family_head}")
            }
            ExecuteMsg::KickFamilyMember { member, .. } => {
                format!("kicking {member} from family")
            }
            ExecuteMsg::CreateFamilyOnBehalf { .. } => "crating node family with".to_string(),
            ExecuteMsg::JoinFamilyOnBehalf { family_head, .. } => {
                format!("joining family {family_head}")
            }
            ExecuteMsg::LeaveFamilyOnBehalf { family_head, .. } => {
                format!("leaving family {family_head}")
            }
            ExecuteMsg::KickFamilyMemberOnBehalf { member, .. } => {
                format!("kicking {member} from family")
            }
            ExecuteMsg::UpdateRewardingValidatorAddress { address } => {
                format!("updating rewarding validator to {address}")
            }
            ExecuteMsg::UpdateContractStateParams { .. } => {
                "updating mixnet state parameters".into()
            }
            ExecuteMsg::UpdateActiveSetSize {
                active_set_size,
                force_immediately,
            } => format!(
                "updating active set size to {active_set_size}. forced: {force_immediately}"
            ),
            ExecuteMsg::UpdateRewardingParams {
                force_immediately, ..
            } => format!("updating mixnet rewarding parameters. forced: {force_immediately}"),
            ExecuteMsg::UpdateIntervalConfig {
                force_immediately, ..
            } => format!("updating mixnet interval configuration. forced: {force_immediately}"),
            ExecuteMsg::BeginEpochTransition {} => "beginning epoch transition".into(),
            ExecuteMsg::AdvanceCurrentEpoch { .. } => "advancing current epoch".into(),
            ExecuteMsg::ReconcileEpochEvents { .. } => "reconciling epoch events".into(),
            ExecuteMsg::BondMixnode { mix_node, .. } => {
                format!("bonding mixnode {}", mix_node.identity_key)
            }
            ExecuteMsg::BondMixnodeOnBehalf { mix_node, .. } => {
                format!("bonding mixnode {} on behalf", mix_node.identity_key)
            }
            ExecuteMsg::PledgeMore {} => "pledging additional tokens".into(),
            ExecuteMsg::PledgeMoreOnBehalf { .. } => "pledging additional tokens on behalf".into(),
            ExecuteMsg::DecreasePledge { .. } => "decreasing mixnode pledge".into(),
            ExecuteMsg::DecreasePledgeOnBehalf { .. } => {
                "decreasing mixnode pledge on behalf".into()
            }
            ExecuteMsg::UnbondMixnode { .. } => "unbonding mixnode".into(),
            ExecuteMsg::UnbondMixnodeOnBehalf { .. } => "unbonding mixnode on behalf".into(),
            ExecuteMsg::UpdateMixnodeCostParams { .. } => "updating mixnode cost parameters".into(),
            ExecuteMsg::UpdateMixnodeCostParamsOnBehalf { .. } => {
                "updating mixnode cost parameters on behalf".into()
            }
            ExecuteMsg::UpdateMixnodeConfig { .. } => "updating mixnode configuration".into(),
            ExecuteMsg::UpdateMixnodeConfigOnBehalf { .. } => {
                "updating mixnode configuration on behalf".into()
            }
            ExecuteMsg::BondGateway { gateway, .. } => {
                format!("bonding gateway {}", gateway.identity_key)
            }
            ExecuteMsg::BondGatewayOnBehalf { gateway, .. } => {
                format!("bonding gateway {} on behalf", gateway.identity_key)
            }
            ExecuteMsg::UnbondGateway { .. } => "unbonding gateway".into(),
            ExecuteMsg::UnbondGatewayOnBehalf { .. } => "unbonding gateway on behalf".into(),
            ExecuteMsg::UpdateGatewayConfig { .. } => "updating gateway configuration".into(),
            ExecuteMsg::UpdateGatewayConfigOnBehalf { .. } => {
                "updating gateway configuration on behalf".into()
            }
            ExecuteMsg::DelegateToMixnode { mix_id } => format!("delegating to mixnode {mix_id}"),
            ExecuteMsg::DelegateToMixnodeOnBehalf { mix_id, .. } => {
                format!("delegating to mixnode {mix_id} on behalf")
            }
            ExecuteMsg::UndelegateFromMixnode { mix_id } => {
                format!("removing delegation from mixnode {mix_id}")
            }
            ExecuteMsg::UndelegateFromMixnodeOnBehalf { mix_id, .. } => {
                format!("removing delegation from mixnode {mix_id} on behalf")
            }
            ExecuteMsg::RewardMixnode {
                mix_id,
                performance,
            } => format!("rewarding mixnode {mix_id} for performance {performance}"),
            ExecuteMsg::WithdrawOperatorReward { .. } => "withdrawing operator reward".into(),
            ExecuteMsg::WithdrawOperatorRewardOnBehalf { .. } => {
                "withdrawing operator reward on behalf".into()
            }
            ExecuteMsg::WithdrawDelegatorReward { mix_id } => {
                format!("withdrawing delegator reward from mixnode {mix_id}")
            }
            ExecuteMsg::WithdrawDelegatorRewardOnBehalf { mix_id, .. } => {
                format!("withdrawing delegator reward from mixnode {mix_id} on behalf")
            }
            #[cfg(feature = "contract-testing")]
            ExecuteMsg::TestingResolveAllPendingEvents { .. } => {
                "resolving all pending events".into()
            }
        }
    }
}

#[cw_serde]
#[cfg_attr(feature = "schema", derive(QueryResponses))]
pub enum QueryMsg {
    // families
    #[cfg_attr(feature = "schema", returns(PagedFamiliesResponse))]
    GetAllFamiliesPaged {
        limit: Option<u32>,
        start_after: Option<String>,
    },

    #[cfg_attr(feature = "schema", returns(PagedMembersResponse))]
    GetAllMembersPaged {
        limit: Option<u32>,
        start_after: Option<String>,
    },

    #[cfg_attr(feature = "schema", returns(Option<Family>))]
    GetFamilyByHead { head: String },

    #[cfg_attr(feature = "schema", returns(Option<Family>))]
    GetFamilyByLabel { label: String },

    #[cfg_attr(feature = "schema", returns(HashSet<String>))]
    GetFamilyMembersByHead { head: String },

    #[cfg_attr(feature = "schema", returns(Option<HashSet<String>>))]
    GetFamilyMembersByLabel { label: String },

    // state/sys-params-related
    #[cfg_attr(feature = "schema", returns(ContractBuildInformation))]
    GetContractVersion {},

    #[serde(rename = "get_cw2_contract_version")]
    #[cfg_attr(feature = "schema", returns(cw2::ContractVersion))]
    GetCW2ContractVersion {},

    #[cfg_attr(feature = "schema", returns(String))]
    GetRewardingValidatorAddress {},

    #[cfg_attr(feature = "schema", returns(ContractStateParams))]
    GetStateParams {},

    #[cfg_attr(feature = "schema", returns(ContractState))]
    GetState {},

    #[cfg_attr(feature = "schema", returns(RewardingParams))]
    GetRewardingParams {},

    #[cfg_attr(feature = "schema", returns(EpochStatus))]
    GetEpochStatus {},

    #[cfg_attr(feature = "schema", returns(CurrentIntervalResponse))]
    GetCurrentIntervalDetails {},

    #[cfg_attr(feature = "schema", returns(PagedRewardedSetResponse))]
    GetRewardedSet {
        limit: Option<u32>,
        start_after: Option<MixId>,
    },

    // mixnode-related:
    #[cfg_attr(feature = "schema", returns(PagedMixnodeBondsResponse))]
    GetMixNodeBonds {
        limit: Option<u32>,
        start_after: Option<MixId>,
    },

    #[cfg_attr(feature = "schema", returns(PagedMixnodesDetailsResponse))]
    GetMixNodesDetailed {
        limit: Option<u32>,
        start_after: Option<MixId>,
    },

    #[cfg_attr(feature = "schema", returns(PagedUnbondedMixnodesResponse))]
    GetUnbondedMixNodes {
        limit: Option<u32>,
        start_after: Option<MixId>,
    },

    #[cfg_attr(feature = "schema", returns(PagedUnbondedMixnodesResponse))]
    GetUnbondedMixNodesByOwner {
        owner: String,
        limit: Option<u32>,
        start_after: Option<MixId>,
    },

    #[cfg_attr(feature = "schema", returns(PagedUnbondedMixnodesResponse))]
    GetUnbondedMixNodesByIdentityKey {
        identity_key: String,
        limit: Option<u32>,
        start_after: Option<MixId>,
    },

    #[cfg_attr(feature = "schema", returns(MixOwnershipResponse))]
    GetOwnedMixnode { address: String },

    #[cfg_attr(feature = "schema", returns(MixnodeDetailsResponse))]
    GetMixnodeDetails { mix_id: MixId },

    #[cfg_attr(feature = "schema", returns(MixnodeRewardingDetailsResponse))]
    GetMixnodeRewardingDetails { mix_id: MixId },

    #[cfg_attr(feature = "schema", returns(StakeSaturationResponse))]
    GetStakeSaturation { mix_id: MixId },

    #[cfg_attr(feature = "schema", returns(UnbondedMixnodeResponse))]
    GetUnbondedMixNodeInformation { mix_id: MixId },

    #[cfg_attr(feature = "schema", returns(MixNodeDetails))]
    GetBondedMixnodeDetailsByIdentity { mix_identity: IdentityKey },

    #[cfg_attr(feature = "schema", returns(LayerDistribution))]
    GetLayerDistribution {},

    // gateway-related:
    #[cfg_attr(feature = "schema", returns(PagedGatewayResponse))]
    GetGateways {
        start_after: Option<IdentityKey>,
        limit: Option<u32>,
    },

    #[cfg_attr(feature = "schema", returns(GatewayBondResponse))]
    GetGatewayBond { identity: IdentityKey },

    #[cfg_attr(feature = "schema", returns(GatewayOwnershipResponse))]
    GetOwnedGateway { address: String },

    // delegation-related:
    // gets all [paged] delegations associated with particular mixnode
    #[cfg_attr(feature = "schema", returns(PagedMixNodeDelegationsResponse))]
    GetMixnodeDelegations {
        mix_id: MixId,
        // since `start_after` is user-provided input, we can't use `Addr` as we
        // can't guarantee it's validated.
        start_after: Option<String>,
        limit: Option<u32>,
    },

    // gets all [paged] delegations associated with particular delegator
    #[cfg_attr(feature = "schema", returns(PagedDelegatorDelegationsResponse))]
    GetDelegatorDelegations {
        // since `delegator` is user-provided input, we can't use `Addr` as we
        // can't guarantee it's validated.
        delegator: String,
        start_after: Option<(MixId, OwnerProxySubKey)>,
        limit: Option<u32>,
    },

    // gets delegation associated with particular mixnode, delegator pair
    #[cfg_attr(feature = "schema", returns(MixNodeDelegationResponse))]
    GetDelegationDetails {
        mix_id: MixId,
        delegator: String,
        proxy: Option<String>,
    },

    // gets all delegations in the system
    #[cfg_attr(feature = "schema", returns(PagedAllDelegationsResponse))]
    GetAllDelegations {
        start_after: Option<delegation::StorageKey>,
        limit: Option<u32>,
    },

    // rewards related
    #[cfg_attr(feature = "schema", returns(PendingRewardResponse))]
    GetPendingOperatorReward { address: String },

    #[cfg_attr(feature = "schema", returns(PendingRewardResponse))]
    GetPendingMixNodeOperatorReward { mix_id: MixId },

    #[cfg_attr(feature = "schema", returns(PendingRewardResponse))]
    GetPendingDelegatorReward {
        address: String,
        mix_id: MixId,
        proxy: Option<String>,
    },

    // given the provided performance, estimate the reward at the end of the current epoch
    #[cfg_attr(feature = "schema", returns(EstimatedCurrentEpochRewardResponse))]
    GetEstimatedCurrentEpochOperatorReward {
        mix_id: MixId,
        estimated_performance: Performance,
    },

    #[cfg_attr(feature = "schema", returns(EstimatedCurrentEpochRewardResponse))]
    GetEstimatedCurrentEpochDelegatorReward {
        address: String,
        mix_id: MixId,
        proxy: Option<String>,
        estimated_performance: Performance,
    },

    // interval-related
    #[cfg_attr(feature = "schema", returns(PendingEpochEventsResponse))]
    GetPendingEpochEvents {
        limit: Option<u32>,
        start_after: Option<u32>,
    },

    #[cfg_attr(feature = "schema", returns(PendingIntervalEventsResponse))]
    GetPendingIntervalEvents {
        limit: Option<u32>,
        start_after: Option<u32>,
    },

    #[cfg_attr(feature = "schema", returns(PendingEpochEventResponse))]
    GetPendingEpochEvent { event_id: EpochEventId },

    #[cfg_attr(feature = "schema", returns(PendingIntervalEventResponse))]
    GetPendingIntervalEvent { event_id: IntervalEventId },

    #[cfg_attr(feature = "schema", returns(NumberOfPendingEventsResponse))]
    GetNumberOfPendingEvents {},

    // signing-related
    #[cfg_attr(feature = "schema", returns(Nonce))]
    GetSigningNonce { address: String },
}

#[cw_serde]
pub struct MigrateMsg {
    pub vesting_contract_address: Option<String>,
}
