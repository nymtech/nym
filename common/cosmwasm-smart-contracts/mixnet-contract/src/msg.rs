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
    families::{
        FamilyByHeadResponse, FamilyByLabelResponse, FamilyMembersByHeadResponse,
        FamilyMembersByLabelResponse, PagedFamiliesResponse, PagedMembersResponse,
    },
    gateway::{GatewayBondResponse, GatewayOwnershipResponse, PagedGatewayResponse},
    interval::{CurrentIntervalResponse, EpochStatus},
    mixnode::{
        MixOwnershipResponse, MixnodeDetailsByIdentityResponse, MixnodeDetailsResponse,
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
                format!("bonding mixnode {}, accepted https://nymtech.net/terms-and-conditions/operators/v1.0.0", mix_node.identity_key)
            }
            ExecuteMsg::BondMixnodeOnBehalf { mix_node, .. } => {
                format!("bonding mixnode {} on behalf, accepted https://nymtech.net/terms-and-conditions/operators/v1.0.0", mix_node.identity_key)
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
                format!("bonding gateway {}, accepted https://nymtech.net/terms-and-conditions/operators/v1.0.0", gateway.identity_key)
            }
            ExecuteMsg::BondGatewayOnBehalf { gateway, .. } => {
                format!("bonding gateway {} on behalf, accepted https://nymtech.net/terms-and-conditions/operators/v1.0.0", gateway.identity_key)
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
    /// Gets the list of families registered in this contract.
    #[cfg_attr(feature = "schema", returns(PagedFamiliesResponse))]
    GetAllFamiliesPaged {
        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,

        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<String>,
    },

    /// Gets the list of all family members registered in this contract.
    #[cfg_attr(feature = "schema", returns(PagedMembersResponse))]
    GetAllMembersPaged {
        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,

        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<String>,
    },

    /// Attempts to lookup family information given the family head.
    #[cfg_attr(feature = "schema", returns(FamilyByHeadResponse))]
    GetFamilyByHead { head: String },

    /// Attempts to lookup family information given the family label.
    #[cfg_attr(feature = "schema", returns(FamilyByLabelResponse))]
    GetFamilyByLabel { label: String },

    /// Attempts to retrieve family members given the family head.
    #[cfg_attr(feature = "schema", returns(FamilyMembersByHeadResponse))]
    GetFamilyMembersByHead { head: String },

    /// Attempts to retrieve family members given the family label.
    #[cfg_attr(feature = "schema", returns(FamilyMembersByLabelResponse))]
    GetFamilyMembersByLabel { label: String },

    // state/sys-params-related
    /// Gets build information of this contract, such as the commit hash used for the build or rustc version.
    #[cfg_attr(feature = "schema", returns(ContractBuildInformation))]
    GetContractVersion {},

    /// Gets the stored contract version information that's required by the CW2 spec interface for migrations.
    #[serde(rename = "get_cw2_contract_version")]
    #[cfg_attr(feature = "schema", returns(cw2::ContractVersion))]
    GetCW2ContractVersion {},

    /// Gets the address of the validator that's allowed to send rewarding transactions and transition the epoch.
    #[cfg_attr(feature = "schema", returns(String))]
    GetRewardingValidatorAddress {},

    /// Gets the contract parameters that could be adjusted in a transaction by the contract admin.
    #[cfg_attr(feature = "schema", returns(ContractStateParams))]
    GetStateParams {},

    /// Gets the current state of the contract.
    #[cfg_attr(feature = "schema", returns(ContractState))]
    GetState {},

    /// Gets the current parameters used for reward calculation.
    #[cfg_attr(feature = "schema", returns(RewardingParams))]
    GetRewardingParams {},

    /// Gets the status of the current rewarding epoch.
    #[cfg_attr(feature = "schema", returns(EpochStatus))]
    GetEpochStatus {},

    /// Get the details of the current rewarding interval.
    #[cfg_attr(feature = "schema", returns(CurrentIntervalResponse))]
    GetCurrentIntervalDetails {},

    /// Gets the current list of mixnodes in the rewarded set.
    #[cfg_attr(feature = "schema", returns(PagedRewardedSetResponse))]
    GetRewardedSet {
        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,

        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<MixId>,
    },

    // mixnode-related:
    /// Gets the basic list of all currently bonded mixnodes.
    #[cfg_attr(feature = "schema", returns(PagedMixnodeBondsResponse))]
    GetMixNodeBonds {
        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,

        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<MixId>,
    },

    /// Gets the detailed list of all currently bonded mixnodes.
    #[cfg_attr(feature = "schema", returns(PagedMixnodesDetailsResponse))]
    GetMixNodesDetailed {
        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,

        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<MixId>,
    },

    /// Gets the basic list of all unbonded mixnodes.
    #[cfg_attr(feature = "schema", returns(PagedUnbondedMixnodesResponse))]
    GetUnbondedMixNodes {
        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,

        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<MixId>,
    },

    /// Gets the basic list of all unbonded mixnodes that belonged to a particular owner.
    #[cfg_attr(feature = "schema", returns(PagedUnbondedMixnodesResponse))]
    GetUnbondedMixNodesByOwner {
        /// The address of the owner of the the mixnodes used for the query.
        owner: String,

        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,

        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<MixId>,
    },

    /// Gets the basic list of all unbonded mixnodes that used the particular identity key.
    #[cfg_attr(feature = "schema", returns(PagedUnbondedMixnodesResponse))]
    GetUnbondedMixNodesByIdentityKey {
        /// The identity key (base58-encoded ed25519 public key) of the mixnode used for the query.
        identity_key: String,

        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,

        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<MixId>,
    },

    /// Gets the detailed mixnode information belonging to the particular owner.
    #[cfg_attr(feature = "schema", returns(MixOwnershipResponse))]
    GetOwnedMixnode {
        /// Address of the mixnode owner to use for the query.
        address: String,
    },

    /// Gets the detailed mixnode information of a node with the provided id.
    #[cfg_attr(feature = "schema", returns(MixnodeDetailsResponse))]
    GetMixnodeDetails {
        /// Id of the node to query.
        mix_id: MixId,
    },

    /// Gets the rewarding information of a mixnode with the provided id.
    #[cfg_attr(feature = "schema", returns(MixnodeRewardingDetailsResponse))]
    GetMixnodeRewardingDetails {
        /// Id of the node to query.
        mix_id: MixId,
    },

    /// Gets the stake saturation of a mixnode with the provided id.
    #[cfg_attr(feature = "schema", returns(StakeSaturationResponse))]
    GetStakeSaturation {
        /// Id of the node to query.
        mix_id: MixId,
    },

    /// Gets the basic information of an unbonded mixnode with the provided id.
    #[cfg_attr(feature = "schema", returns(UnbondedMixnodeResponse))]
    GetUnbondedMixNodeInformation {
        /// Id of the node to query.
        mix_id: MixId,
    },

    /// Gets the detailed mixnode information of a node given its current identity key.
    #[cfg_attr(feature = "schema", returns(MixnodeDetailsByIdentityResponse))]
    GetBondedMixnodeDetailsByIdentity {
        /// The identity key (base58-encoded ed25519 public key) of the mixnode used for the query.
        mix_identity: IdentityKey,
    },

    /// Gets the current layer configuration of the mix network.
    #[cfg_attr(feature = "schema", returns(LayerDistribution))]
    GetLayerDistribution {},

    // gateway-related:
    /// Gets the basic list of all currently bonded gateways.
    #[cfg_attr(feature = "schema", returns(PagedGatewayResponse))]
    GetGateways {
        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<IdentityKey>,

        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,
    },

    /// Gets the gateway details of a node given its identity key.
    #[cfg_attr(feature = "schema", returns(GatewayBondResponse))]
    GetGatewayBond {
        /// The identity key (base58-encoded ed25519 public key) of the gateway used for the query.
        identity: IdentityKey,
    },

    /// Gets the detailed gateway information belonging to the particular owner.
    #[cfg_attr(feature = "schema", returns(GatewayOwnershipResponse))]
    GetOwnedGateway {
        /// Address of the gateway owner to use for the query.
        address: String,
    },

    // delegation-related:
    /// Gets all delegations associated with particular mixnode
    #[cfg_attr(feature = "schema", returns(PagedMixNodeDelegationsResponse))]
    GetMixnodeDelegations {
        /// Id of the node to query.
        mix_id: MixId,

        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<OwnerProxySubKey>,

        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,
    },

    /// Gets all delegations associated with particular delegator
    #[cfg_attr(feature = "schema", returns(PagedDelegatorDelegationsResponse))]
    GetDelegatorDelegations {
        // since `delegator` is user-provided input, we can't use `Addr` as we
        // can't guarantee it's validated.
        /// The address of the owner of the delegations.
        delegator: String,

        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<(MixId, OwnerProxySubKey)>,

        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,
    },

    /// Gets delegation information associated with particular mixnode - delegator pair
    #[cfg_attr(feature = "schema", returns(MixNodeDelegationResponse))]
    GetDelegationDetails {
        /// Id of the node to query.
        mix_id: MixId,

        /// The address of the owner of the delegation.
        delegator: String,

        /// Entity who made the delegation on behalf of the owner.
        /// If present, it's most likely the address of the vesting contract.
        proxy: Option<String>,
    },

    /// Gets all delegations in the system
    #[cfg_attr(feature = "schema", returns(PagedAllDelegationsResponse))]
    GetAllDelegations {
        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<delegation::StorageKey>,

        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,
    },

    // rewards related
    /// Gets the reward amount accrued by the node operator that has not yet been claimed.
    #[cfg_attr(feature = "schema", returns(PendingRewardResponse))]
    GetPendingOperatorReward {
        /// Address of the operator to use for the query.
        address: String,
    },

    /// Gets the reward amount accrued by the particular mixnode that has not yet been claimed.
    #[cfg_attr(feature = "schema", returns(PendingRewardResponse))]
    GetPendingMixNodeOperatorReward {
        /// Id of the node to query.
        mix_id: MixId,
    },

    /// Gets the reward amount accrued by the particular delegator that has not yet been claimed.
    #[cfg_attr(feature = "schema", returns(PendingRewardResponse))]
    GetPendingDelegatorReward {
        /// Address of the delegator to use for the query.
        address: String,

        /// Id of the node to query.
        mix_id: MixId,

        /// Entity who made the delegation on behalf of the owner.
        /// If present, it's most likely the address of the vesting contract.
        proxy: Option<String>,
    },

    /// Given the provided node performance, attempt to estimate the operator reward for the current epoch.
    #[cfg_attr(feature = "schema", returns(EstimatedCurrentEpochRewardResponse))]
    GetEstimatedCurrentEpochOperatorReward {
        /// Id of the node to query.
        mix_id: MixId,

        /// The estimated performance for the current epoch of the given node.
        estimated_performance: Performance,
    },

    /// Given the provided node performance, attempt to estimate the delegator reward for the current epoch.
    #[cfg_attr(feature = "schema", returns(EstimatedCurrentEpochRewardResponse))]
    GetEstimatedCurrentEpochDelegatorReward {
        /// Address of the delegator to use for the query.
        address: String,

        /// Id of the node to query.
        mix_id: MixId,

        /// Entity who made the delegation on behalf of the owner.
        /// If present, it's most likely the address of the vesting contract.
        proxy: Option<String>,

        /// The estimated performance for the current epoch of the given node.
        estimated_performance: Performance,
    },

    // interval-related
    /// Gets the list of all currently pending epoch events that will be resolved once the current epoch finishes.
    #[cfg_attr(feature = "schema", returns(PendingEpochEventsResponse))]
    GetPendingEpochEvents {
        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,

        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<u32>,
    },

    /// Gets the list of all currently pending interval events that will be resolved once the current interval finishes.
    #[cfg_attr(feature = "schema", returns(PendingIntervalEventsResponse))]
    GetPendingIntervalEvents {
        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,

        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<u32>,
    },

    /// Gets detailed information about a pending epoch event given its id.
    #[cfg_attr(feature = "schema", returns(PendingEpochEventResponse))]
    GetPendingEpochEvent {
        /// The unique id associated with the event.
        event_id: EpochEventId,
    },

    /// Gets detailed information about a pending interval event given its id.
    #[cfg_attr(feature = "schema", returns(PendingIntervalEventResponse))]
    GetPendingIntervalEvent {
        /// The unique id associated with the event.
        event_id: IntervalEventId,
    },

    /// Gets the information about the number of currently pending epoch and interval events.
    #[cfg_attr(feature = "schema", returns(NumberOfPendingEventsResponse))]
    GetNumberOfPendingEvents {},

    // signing-related
    /// Gets the signing nonce associated with the particular cosmos address.
    #[cfg_attr(feature = "schema", returns(Nonce))]
    GetSigningNonce {
        /// Cosmos address used for the query of the signing nonce.
        address: String,
    },
}

#[cw_serde]
pub struct MigrateMsg {
    pub vesting_contract_address: Option<String>,
}
