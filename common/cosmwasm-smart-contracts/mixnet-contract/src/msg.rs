// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::delegation::{self, OwnerProxySubKey};
use crate::error::MixnetContractError;
use crate::gateway::{Gateway, GatewayConfigUpdate};
use crate::helpers::IntoBaseDecimal;
use crate::mixnode::{MixNode, MixNodeConfigUpdate, NodeCostParams};
use crate::nym_node::{NodeConfigUpdate, Role};
use crate::pending_events::{EpochEventId, IntervalEventId};
use crate::reward_params::{
    ActiveSetUpdate, IntervalRewardParams, IntervalRewardingParamsUpdate, NodeRewardingParameters,
    Performance, RewardedSetParams, RewardingParams, WorkFactor,
};
use crate::types::NodeId;
use crate::{
    ContractStateParamsUpdate, NymNode, OutdatedVersionWeights, RoleAssignment,
    VersionScoreFormulaParams,
};
use crate::{OperatingCostRange, ProfitMarginRange};
use contracts_common::{signing::MessageSignature, IdentityKey, Percent};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Decimal};
use std::time::Duration;

#[cfg(feature = "schema")]
use crate::{
    delegation::{
        NodeDelegationResponse, PagedAllDelegationsResponse, PagedDelegatorDelegationsResponse,
        PagedNodeDelegationsResponse,
    },
    gateway::{
        GatewayBondResponse, GatewayOwnershipResponse, PagedGatewayResponse,
        PreassignedGatewayIdsResponse,
    },
    interval::{CurrentIntervalResponse, EpochStatus},
    mixnode::{
        MixOwnershipResponse, MixStakeSaturationResponse, MixnodeDetailsByIdentityResponse,
        MixnodeDetailsResponse, MixnodeRewardingDetailsResponse, PagedMixnodeBondsResponse,
        PagedMixnodesDetailsResponse, PagedUnbondedMixnodesResponse, UnbondedMixnodeResponse,
    },
    nym_node::{
        EpochAssignmentResponse, NodeDetailsByIdentityResponse, NodeDetailsResponse,
        NodeOwnershipResponse, NodeRewardingDetailsResponse, PagedNymNodeBondsResponse,
        PagedNymNodeDetailsResponse, PagedUnbondedNymNodesResponse, RolesMetadataResponse,
        StakeSaturationResponse, UnbondedNodeResponse,
    },
    pending_events::{
        NumberOfPendingEventsResponse, PendingEpochEventResponse, PendingEpochEventsResponse,
        PendingIntervalEventResponse, PendingIntervalEventsResponse,
    },
    rewarding::{EstimatedCurrentEpochRewardResponse, PendingRewardResponse},
    types::{ContractState, ContractStateParams},
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

    pub current_nym_node_version: String,

    #[serde(default)]
    pub version_score_weights: OutdatedVersionWeights,

    #[serde(default)]
    pub version_score_params: VersionScoreFormulaParams,

    #[serde(default)]
    pub profit_margin: ProfitMarginRange,

    #[serde(default)]
    pub interval_operating_cost: OperatingCostRange,
}

#[cw_serde]
pub struct InitialRewardingParams {
    pub initial_reward_pool: Decimal,
    pub initial_staking_supply: Decimal,

    pub staking_supply_scale_factor: Percent,
    pub sybil_resistance: Percent,
    pub active_set_work_factor: Decimal,
    pub interval_pool_emission: Percent,

    pub rewarded_set_params: RewardedSetParams,
}

impl InitialRewardingParams {
    pub fn into_rewarding_params(
        self,
        epochs_in_interval: u32,
    ) -> Result<RewardingParams, MixnetContractError> {
        let epoch_reward_budget = self.initial_reward_pool
            / epochs_in_interval.into_base_decimal()?
            * self.interval_pool_emission;
        let stake_saturation_point = self.initial_staking_supply
            / self
                .rewarded_set_params
                .rewarded_set_size()
                .into_base_decimal()?;

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
            rewarded_set: self.rewarded_set_params,
        })
    }
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Change the admin
    UpdateAdmin {
        admin: String,
    },

    // state/sys-params-related
    UpdateRewardingValidatorAddress {
        address: String,
    },
    UpdateContractStateParams {
        update: ContractStateParamsUpdate,
    },
    UpdateCurrentNymNodeSemver {
        current_version: String,
    },
    UpdateActiveSetDistribution {
        update: ActiveSetUpdate,
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
    ReconcileEpochEvents {
        limit: Option<u32>,
    },
    AssignRoles {
        assignment: RoleAssignment,
    },

    // mixnode-related:
    BondMixnode {
        mix_node: MixNode,
        cost_params: NodeCostParams,
        owner_signature: MessageSignature,
    },
    BondMixnodeOnBehalf {
        mix_node: MixNode,
        cost_params: NodeCostParams,
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
    #[serde(
        alias = "UpdateMixnodeCostParams",
        alias = "update_mixnode_cost_params"
    )]
    UpdateCostParams {
        new_costs: NodeCostParams,
    },
    UpdateMixnodeCostParamsOnBehalf {
        new_costs: NodeCostParams,
        owner: String,
    },
    UpdateMixnodeConfig {
        new_config: MixNodeConfigUpdate,
    },
    UpdateMixnodeConfigOnBehalf {
        new_config: MixNodeConfigUpdate,
        owner: String,
    },
    MigrateMixnode {},

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
    MigrateGateway {
        cost_params: Option<NodeCostParams>,
    },

    // nym-node related:
    BondNymNode {
        node: NymNode,
        cost_params: NodeCostParams,
        owner_signature: MessageSignature,
    },
    UnbondNymNode {},
    UpdateNodeConfig {
        update: NodeConfigUpdate,
    },

    // delegation-related:
    #[serde(alias = "DelegateToMixnode", alias = "delegate_to_mixnode")]
    Delegate {
        #[serde(alias = "mix_id")]
        node_id: NodeId,
    },
    DelegateToMixnodeOnBehalf {
        mix_id: NodeId,
        delegate: String,
    },
    #[serde(alias = "UndelegateFromMixnode", alias = "undelegate_from_mixnode")]
    Undelegate {
        #[serde(alias = "mix_id")]
        node_id: NodeId,
    },
    UndelegateFromMixnodeOnBehalf {
        mix_id: NodeId,
        delegate: String,
    },

    // reward-related
    RewardNode {
        #[serde(alias = "mix_id")]
        node_id: NodeId,
        params: NodeRewardingParameters,
    },
    WithdrawOperatorReward {},
    WithdrawOperatorRewardOnBehalf {
        owner: String,
    },
    WithdrawDelegatorReward {
        #[serde(alias = "mix_id")]
        node_id: NodeId,
    },
    WithdrawDelegatorRewardOnBehalf {
        mix_id: NodeId,
        owner: String,
    },

    // vesting migration:
    MigrateVestedMixNode {},
    MigrateVestedDelegation {
        mix_id: NodeId,
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
            ExecuteMsg::UpdateAdmin { admin } => format!("updating contract admin to {admin}"),
            ExecuteMsg::UpdateRewardingValidatorAddress { address } => {
                format!("updating rewarding validator to {address}")
            }
            ExecuteMsg::UpdateContractStateParams { .. } => {
                "updating mixnet state parameters".into()
            }
            ExecuteMsg::UpdateCurrentNymNodeSemver { current_version } => {
                format!("updating current nym-node semver to {current_version}")
            }
            ExecuteMsg::UpdateActiveSetDistribution {
                force_immediately, ..
            } => format!("updating active set distribution. forced: {force_immediately}"),
            ExecuteMsg::UpdateRewardingParams {
                force_immediately, ..
            } => format!("updating mixnet rewarding parameters. forced: {force_immediately}"),
            ExecuteMsg::UpdateIntervalConfig {
                force_immediately, ..
            } => format!("updating mixnet interval configuration. forced: {force_immediately}"),
            ExecuteMsg::BeginEpochTransition {} => "beginning epoch transition".into(),
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
            ExecuteMsg::UpdateCostParams { .. } => "updating mixnode cost parameters".into(),
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
            ExecuteMsg::Delegate { node_id: mix_id } => format!("delegating to mixnode {mix_id}"),
            ExecuteMsg::DelegateToMixnodeOnBehalf { mix_id, .. } => {
                format!("delegating to mixnode {mix_id} on behalf")
            }
            ExecuteMsg::Undelegate { node_id: mix_id } => {
                format!("removing delegation from mixnode {mix_id}")
            }
            ExecuteMsg::UndelegateFromMixnodeOnBehalf { mix_id, .. } => {
                format!("removing delegation from mixnode {mix_id} on behalf")
            }
            ExecuteMsg::RewardNode { node_id, .. } => format!("rewarding node {node_id}"),
            ExecuteMsg::WithdrawOperatorReward { .. } => "withdrawing operator reward".into(),
            ExecuteMsg::WithdrawOperatorRewardOnBehalf { .. } => {
                "withdrawing operator reward on behalf".into()
            }
            ExecuteMsg::WithdrawDelegatorReward { node_id: mix_id } => {
                format!("withdrawing delegator reward from mixnode {mix_id}")
            }
            ExecuteMsg::WithdrawDelegatorRewardOnBehalf { mix_id, .. } => {
                format!("withdrawing delegator reward from mixnode {mix_id} on behalf")
            }
            ExecuteMsg::MigrateVestedMixNode { .. } => "migrate vested mixnode".into(),
            ExecuteMsg::MigrateVestedDelegation { .. } => "migrate vested delegation".to_string(),
            ExecuteMsg::AssignRoles { .. } => "assigning epoch roles".into(),
            ExecuteMsg::MigrateMixnode { .. } => "migrating legacy mixnode".into(),
            ExecuteMsg::MigrateGateway { .. } => "migrating legacy gateway".into(),
            ExecuteMsg::BondNymNode { .. } => "bonding nym-node".into(),
            ExecuteMsg::UnbondNymNode { .. } => "unbonding nym-node".into(),
            ExecuteMsg::UpdateNodeConfig { .. } => "updating node config".into(),

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
    #[cfg_attr(feature = "schema", returns(cw_controllers::AdminResponse))]
    Admin {},

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

    // mixnode-related:
    /// Gets the basic list of all currently bonded mixnodes.
    #[cfg_attr(feature = "schema", returns(PagedMixnodeBondsResponse))]
    GetMixNodeBonds {
        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,

        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<NodeId>,
    },

    /// Gets the detailed list of all currently bonded mixnodes.
    #[cfg_attr(feature = "schema", returns(PagedMixnodesDetailsResponse))]
    GetMixNodesDetailed {
        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,

        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<NodeId>,
    },

    /// Gets the basic list of all unbonded mixnodes.
    #[cfg_attr(feature = "schema", returns(PagedUnbondedMixnodesResponse))]
    GetUnbondedMixNodes {
        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,

        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<NodeId>,
    },

    /// Gets the basic list of all unbonded mixnodes that belonged to a particular owner.
    #[cfg_attr(feature = "schema", returns(PagedUnbondedMixnodesResponse))]
    GetUnbondedMixNodesByOwner {
        /// The address of the owner of the mixnodes used for the query.
        owner: String,

        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,

        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<NodeId>,
    },

    /// Gets the basic list of all unbonded mixnodes that used the particular identity key.
    #[cfg_attr(feature = "schema", returns(PagedUnbondedMixnodesResponse))]
    GetUnbondedMixNodesByIdentityKey {
        /// The identity key (base58-encoded ed25519 public key) of the mixnode used for the query.
        identity_key: String,

        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,

        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<NodeId>,
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
        mix_id: NodeId,
    },

    /// Gets the rewarding information of a mixnode with the provided id.
    #[cfg_attr(feature = "schema", returns(MixnodeRewardingDetailsResponse))]
    GetMixnodeRewardingDetails {
        /// Id of the node to query.
        mix_id: NodeId,
    },

    /// Gets the stake saturation of a mixnode with the provided id.
    #[cfg_attr(feature = "schema", returns(MixStakeSaturationResponse))]
    GetStakeSaturation {
        /// Id of the node to query.
        mix_id: NodeId,
    },

    /// Gets the basic information of an unbonded mixnode with the provided id.
    #[cfg_attr(feature = "schema", returns(UnbondedMixnodeResponse))]
    GetUnbondedMixNodeInformation {
        /// Id of the node to query.
        mix_id: NodeId,
    },

    /// Gets the detailed mixnode information of a node given its current identity key.
    #[cfg_attr(feature = "schema", returns(MixnodeDetailsByIdentityResponse))]
    GetBondedMixnodeDetailsByIdentity {
        /// The identity key (base58-encoded ed25519 public key) of the mixnode used for the query.
        mix_identity: IdentityKey,
    },

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

    /// Get the `NodeId`s of all the legacy gateways that they will get assigned once migrated into NymNodes
    #[cfg_attr(feature = "schema", returns(PreassignedGatewayIdsResponse))]
    GetPreassignedGatewayIds {
        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<IdentityKey>,

        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,
    },

    // nym-node-related:
    /// Gets the basic list of all currently bonded nymnodes.
    #[cfg_attr(feature = "schema", returns(PagedNymNodeBondsResponse))]
    GetNymNodeBondsPaged {
        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,

        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<NodeId>,
    },

    /// Gets the detailed list of all currently bonded nymnodes.
    #[cfg_attr(feature = "schema", returns(PagedNymNodeDetailsResponse))]
    GetNymNodesDetailedPaged {
        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,

        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<NodeId>,
    },

    /// Gets the basic information of an unbonded nym-node with the provided id.
    #[cfg_attr(feature = "schema", returns(UnbondedNodeResponse))]
    GetUnbondedNymNode {
        /// Id of the node to query.
        node_id: NodeId,
    },

    /// Gets the basic list of all unbonded nymnodes.
    #[cfg_attr(feature = "schema", returns(PagedUnbondedNymNodesResponse))]
    GetUnbondedNymNodesPaged {
        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,

        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<NodeId>,
    },

    /// Gets the basic list of all unbonded nymnodes that belonged to a particular owner.
    #[cfg_attr(feature = "schema", returns(PagedUnbondedNymNodesResponse))]
    GetUnbondedNymNodesByOwnerPaged {
        /// The address of the owner of the nym-node used for the query
        owner: String,

        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,

        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<NodeId>,
    },

    /// Gets the basic list of all unbonded nymnodes that used the particular identity key.
    #[cfg_attr(feature = "schema", returns(PagedUnbondedNymNodesResponse))]
    GetUnbondedNymNodesByIdentityKeyPaged {
        /// The identity key (base58-encoded ed25519 public key) of the node used for the query.
        identity_key: IdentityKey,

        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,

        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<NodeId>,
    },

    /// Gets the detailed nymnode information belonging to the particular owner.
    #[cfg_attr(feature = "schema", returns(NodeOwnershipResponse))]
    GetOwnedNymNode {
        /// Address of the node owner to use for the query.
        address: String,
    },

    /// Gets the detailed nymnode information of a node with the provided id.
    #[cfg_attr(feature = "schema", returns(NodeDetailsResponse))]
    GetNymNodeDetails {
        /// Id of the node to query.
        node_id: NodeId,
    },

    /// Gets the detailed nym-node information given its current identity key.
    #[cfg_attr(feature = "schema", returns(NodeDetailsByIdentityResponse))]
    GetNymNodeDetailsByIdentityKey {
        /// The identity key (base58-encoded ed25519 public key) of the nym-node used for the query.
        node_identity: IdentityKey,
    },

    /// Gets the rewarding information of a nym-node with the provided id.
    #[cfg_attr(feature = "schema", returns(NodeRewardingDetailsResponse))]
    GetNodeRewardingDetails {
        /// Id of the node to query.
        node_id: NodeId,
    },

    /// Gets the stake saturation of a nym-node with the provided id.
    #[cfg_attr(feature = "schema", returns(StakeSaturationResponse))]
    GetNodeStakeSaturation {
        /// Id of the node to query.
        node_id: NodeId,
    },

    #[cfg_attr(feature = "schema", returns(EpochAssignmentResponse))]
    GetRoleAssignment { role: Role },

    #[cfg_attr(feature = "schema", returns(RolesMetadataResponse))]
    GetRewardedSetMetadata {},

    // delegation-related:
    /// Gets all delegations associated with particular node
    #[cfg_attr(feature = "schema", returns(PagedNodeDelegationsResponse))]
    #[serde(alias = "GetMixnodeDelegations", alias = "get_mixnode_delegations")]
    GetNodeDelegations {
        /// Id of the node to query.
        #[serde(alias = "mix_id")]
        node_id: NodeId,

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
        start_after: Option<(NodeId, OwnerProxySubKey)>,

        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,
    },

    /// Gets delegation information associated with particular mixnode - delegator pair
    #[cfg_attr(feature = "schema", returns(NodeDelegationResponse))]
    GetDelegationDetails {
        /// Id of the node to query.
        #[serde(alias = "mix_id")]
        node_id: NodeId,

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
    #[serde(
        alias = "GetPendingMixNodeOperatorReward",
        alias = "get_pending_mix_node_operator_reward"
    )]
    GetPendingNodeOperatorReward {
        /// Id of the node to query.
        #[serde(alias = "mix_id")]
        node_id: NodeId,
    },

    /// Gets the reward amount accrued by the particular delegator that has not yet been claimed.
    #[cfg_attr(feature = "schema", returns(PendingRewardResponse))]
    GetPendingDelegatorReward {
        /// Address of the delegator to use for the query.
        address: String,

        /// Id of the node to query.
        #[serde(alias = "mix_id")]
        node_id: NodeId,

        /// Entity who made the delegation on behalf of the owner.
        /// If present, it's most likely the address of the vesting contract.
        proxy: Option<String>,
    },

    /// Given the provided node performance, attempt to estimate the operator reward for the current epoch.
    #[cfg_attr(feature = "schema", returns(EstimatedCurrentEpochRewardResponse))]
    GetEstimatedCurrentEpochOperatorReward {
        /// Id of the node to query.
        #[serde(alias = "mix_id")]
        node_id: NodeId,

        /// The estimated performance for the current epoch of the given node.
        estimated_performance: Performance,

        /// The estimated work for the current epoch of the given node.
        estimated_work: Option<WorkFactor>,
    },

    /// Given the provided node performance, attempt to estimate the delegator reward for the current epoch.
    #[cfg_attr(feature = "schema", returns(EstimatedCurrentEpochRewardResponse))]
    GetEstimatedCurrentEpochDelegatorReward {
        /// Address of the delegator to use for the query.
        address: String,

        /// Id of the node to query.
        #[serde(alias = "mix_id")]
        node_id: NodeId,

        /// The estimated performance for the current epoch of the given node.
        estimated_performance: Performance,

        /// The estimated work for the current epoch of the given node.
        estimated_work: Option<WorkFactor>,
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
    pub unsafe_skip_state_updates: Option<bool>,
    pub vesting_contract_address: Option<String>,
    pub current_nym_node_semver: String,

    #[serde(default)]
    pub version_score_weights: OutdatedVersionWeights,

    #[serde(default)]
    pub version_score_params: VersionScoreFormulaParams,
}
