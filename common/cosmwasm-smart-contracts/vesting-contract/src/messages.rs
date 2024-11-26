// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{PledgeCap, VestingSpecification};
use contracts_common::signing::MessageSignature;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Timestamp};
use mixnet_contract_common::{
    gateway::GatewayConfigUpdate,
    mixnode::{MixNodeConfigUpdate, NodeCostParams},
    Gateway, MixNode, NodeId,
};

#[cfg(feature = "schema")]
use contracts_common::ContractBuildInformation;
#[cfg(feature = "schema")]
use cosmwasm_schema::QueryResponses;

#[cfg(feature = "schema")]
use crate::{
    account::Account,
    types::{Period, PledgeData, VestingDelegation},
    AccountsResponse, AllDelegationsResponse, DelegationTimesResponse, OriginalVestingResponse,
    VestingCoinsResponse,
};

#[cw_serde]
pub struct InitMsg {
    pub mixnet_contract_address: String,
    pub mix_denom: String,
}

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    TrackReward {
        amount: Coin,
        address: String,
    },
    ClaimOperatorReward {},
    ClaimDelegatorReward {
        mix_id: NodeId,
    },
    UpdateMixnodeCostParams {
        new_costs: NodeCostParams,
    },
    UpdateMixnodeConfig {
        new_config: MixNodeConfigUpdate,
    },
    UpdateMixnetAddress {
        address: String,
    },
    DelegateToMixnode {
        mix_id: NodeId,
        amount: Coin,
        on_behalf_of: Option<String>,
    },
    UndelegateFromMixnode {
        mix_id: NodeId,
        on_behalf_of: Option<String>,
    },
    CreateAccount {
        owner_address: String,
        staking_address: Option<String>,
        vesting_spec: Option<VestingSpecification>,
        cap: Option<PledgeCap>,
    },
    WithdrawVestedCoins {
        amount: Coin,
    },
    TrackUndelegation {
        owner: String,
        mix_id: NodeId,
        amount: Coin,
    },
    BondMixnode {
        mix_node: MixNode,
        cost_params: NodeCostParams,
        owner_signature: MessageSignature,
        amount: Coin,
    },
    PledgeMore {
        amount: Coin,
    },
    DecreasePledge {
        amount: Coin,
    },
    UnbondMixnode {},
    TrackUnbondMixnode {
        owner: String,
        amount: Coin,
    },
    TrackDecreasePledge {
        owner: String,
        amount: Coin,
    },
    BondGateway {
        gateway: Gateway,
        owner_signature: MessageSignature,
        amount: Coin,
    },
    UnbondGateway {},
    TrackUnbondGateway {
        owner: String,
        amount: Coin,
    },
    UpdateGatewayConfig {
        new_config: GatewayConfigUpdate,
    },
    TransferOwnership {
        to_address: String,
    },
    UpdateStakingAddress {
        to_address: Option<String>,
    },
    UpdateLockedPledgeCap {
        address: String,
        cap: PledgeCap,
    },
    TrackMigratedMixnode {
        owner: String,
    },
    // no need to track migrated gateways as there are no vesting gateways on mainnet
    TrackMigratedDelegation {
        owner: String,
        mix_id: NodeId,
    },
}

impl ExecuteMsg {
    pub fn name(&self) -> &str {
        match self {
            ExecuteMsg::TrackReward { .. } => "VestingExecuteMsg::TrackReward",
            ExecuteMsg::ClaimOperatorReward { .. } => "VestingExecuteMsg::ClaimOperatorReward",
            ExecuteMsg::ClaimDelegatorReward { .. } => "VestingExecuteMsg::ClaimDelegatorReward",
            ExecuteMsg::UpdateMixnodeConfig { .. } => "VestingExecuteMsg::UpdateMixnodeConfig",
            ExecuteMsg::UpdateMixnodeCostParams { .. } => {
                "VestingExecuteMsg::UpdateMixnodeCostParams"
            }
            ExecuteMsg::UpdateMixnetAddress { .. } => "VestingExecuteMsg::UpdateMixnetAddress",
            ExecuteMsg::DelegateToMixnode { .. } => "VestingExecuteMsg::DelegateToMixnode",
            ExecuteMsg::UndelegateFromMixnode { .. } => "VestingExecuteMsg::UndelegateFromMixnode",
            ExecuteMsg::CreateAccount { .. } => "VestingExecuteMsg::CreateAccount",
            ExecuteMsg::WithdrawVestedCoins { .. } => "VestingExecuteMsg::WithdrawVestedCoins",
            ExecuteMsg::TrackUndelegation { .. } => "VestingExecuteMsg::TrackUndelegation",
            ExecuteMsg::BondMixnode { .. } => "VestingExecuteMsg::BondMixnode",
            ExecuteMsg::PledgeMore { .. } => "VestingExecuteMsg::PledgeMore",
            ExecuteMsg::DecreasePledge { .. } => "VestingExecuteMsg::DecreasePledge",
            ExecuteMsg::UnbondMixnode { .. } => "VestingExecuteMsg::UnbondMixnode",
            ExecuteMsg::TrackUnbondMixnode { .. } => "VestingExecuteMsg::TrackUnbondMixnode",
            ExecuteMsg::TrackDecreasePledge { .. } => "VestingExecuteMsg::TrackDecreasePledge",
            ExecuteMsg::BondGateway { .. } => "VestingExecuteMsg::BondGateway",
            ExecuteMsg::UnbondGateway { .. } => "VestingExecuteMsg::UnbondGateway",
            ExecuteMsg::TrackUnbondGateway { .. } => "VestingExecuteMsg::TrackUnbondGateway",
            ExecuteMsg::UpdateGatewayConfig { .. } => "VestingExecuteMsg::UpdateGatewayConfig",
            ExecuteMsg::TransferOwnership { .. } => "VestingExecuteMsg::TransferOwnership",
            ExecuteMsg::UpdateStakingAddress { .. } => "VestingExecuteMsg::UpdateStakingAddress",
            ExecuteMsg::UpdateLockedPledgeCap { .. } => "VestingExecuteMsg::UpdateLockedPledgeCap",
            ExecuteMsg::TrackMigratedMixnode { .. } => "VestingExecuteMsg::TrackMigratedMixnode",
            ExecuteMsg::TrackMigratedDelegation { .. } => {
                "VestingExecuteMsg::TrackMigratedDelegation"
            }
        }
    }
}

/// Queries exposed by this contract.
#[cw_serde]
#[cfg_attr(feature = "schema", derive(QueryResponses))]
pub enum QueryMsg {
    /// Gets build information of this contract, such as the commit hash used for the build or rustc version.
    #[cfg_attr(feature = "schema", returns(ContractBuildInformation))]
    GetContractVersion {},

    /// Gets the stored contract version information that's required by the CW2 spec interface for migrations.
    #[serde(rename = "get_cw2_contract_version")]
    #[cfg_attr(feature = "schema", returns(cw2::ContractVersion))]
    GetCW2ContractVersion {},

    /// Gets the list of vesting accounts held in this contract.
    #[cfg_attr(feature = "schema", returns(AccountsResponse))]
    GetAccountsPaged {
        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_next_after: Option<String>,

        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,
    },

    /// Gets the list of coins that are still vesting for each account held in this contract.
    #[cfg_attr(feature = "schema", returns(VestingCoinsResponse))]
    GetAccountsVestingCoinsPaged {
        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_next_after: Option<String>,

        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,
    },

    /// Returns the amount of locked coins for the provided vesting account,
    /// i.e. coins that are still vesting but have not been staked.
    /// `locked_coins = vesting_coins - staked_coins`
    #[cfg_attr(feature = "schema", returns(Coin))]
    LockedCoins {
        /// Address of the vesting account in question.
        vesting_account_address: String,

        /// (deprecated) Optional argument specifying that the query should be performed against non-current block.
        block_time: Option<Timestamp>,
    },

    /// Returns the amount of spendable coins for the provided vesting account,
    /// i.e. coins that could be withdrawn.
    /// `spendable_coins = account_balance - locked_coins`
    /// note: `account_balance` is the amount of coins still physically present in this contract, i.e. not withdrawn or staked.
    #[cfg_attr(feature = "schema", returns(Coin))]
    SpendableCoins {
        /// Address of the vesting account in question.
        vesting_account_address: String,

        /// (deprecated) Optional argument specifying that the query should be performed against non-current block.
        block_time: Option<Timestamp>,
    },

    /// Returns the amount of coins that have already vested for the provided vesting account address.
    #[cfg_attr(feature = "schema", returns(Coin))]
    GetVestedCoins {
        /// Address of the vesting account in question.
        vesting_account_address: String,

        /// (deprecated) Optional argument specifying that the query should be performed against non-current block.
        block_time: Option<Timestamp>,
    },

    /// Returns the amount of coins that are still vesting for the provided vesting account address.
    #[cfg_attr(feature = "schema", returns(Coin))]
    GetVestingCoins {
        /// Address of the vesting account in question.
        vesting_account_address: String,

        /// (deprecated) Optional argument specifying that the query should be performed against non-current block.
        block_time: Option<Timestamp>,
    },

    /// Returns the starting vesting time for the provided vesting account,
    /// i.e. the beginning of the first vesting period.
    #[cfg_attr(feature = "schema", returns(Timestamp))]
    GetStartTime {
        /// Address of the vesting account in question.
        vesting_account_address: String,
    },

    /// Returns the ending vesting time for the provided vesting account,
    /// i.e. the end of the last vesting period.
    #[cfg_attr(feature = "schema", returns(Timestamp))]
    GetEndTime {
        /// Address of the vesting account in question.
        vesting_account_address: String,
    },

    /// Returns the initial vesting specification used for the provided vesting account address.
    #[cfg_attr(feature = "schema", returns(OriginalVestingResponse))]
    GetOriginalVesting {
        /// Address of the vesting account in question.
        vesting_account_address: String,
    },

    /// Returns the total amount of coins accrued through claimed staking rewards by the provided vesting account.
    #[cfg_attr(feature = "schema", returns(Coin))]
    GetHistoricalVestingStakingReward {
        /// Address of the vesting account in question.
        vesting_account_address: String,
    },

    /// Returns the amount of spendable vesting coins for the provided vesting account,
    /// i.e. coins that could be withdrawn that originated from the vesting specification.
    #[cfg_attr(feature = "schema", returns(Coin))]
    GetSpendableVestedCoins {
        /// Address of the vesting account in question.
        vesting_account_address: String,
    },

    /// Returns the amount of spendable reward coins for the provided vesting account,
    /// i.e. coins that could be withdrawn that originated from the claimed staking rewards.
    #[cfg_attr(feature = "schema", returns(Coin))]
    GetSpendableRewardCoins {
        /// Address of the vesting account in question.
        vesting_account_address: String,
    },

    /// Returns the amount of coins that are currently delegated for the provided vesting account address.
    #[cfg_attr(feature = "schema", returns(Coin))]
    GetDelegatedCoins {
        /// Address of the vesting account in question.
        vesting_account_address: String,
    },

    /// Returns the amount of coins that are currently pledged for the provided vesting account address.
    #[cfg_attr(feature = "schema", returns(Coin))]
    GetPledgedCoins {
        /// Address of the vesting account in question.
        vesting_account_address: String,
    },

    /// Returns the amount of coins that are currently staked (i.e. delegations + pledges) for the provided vesting account address.
    #[cfg_attr(feature = "schema", returns(Coin))]
    GetStakedCoins {
        /// Address of the vesting account in question.
        vesting_account_address: String,
    },

    /// Returns the amount of coins that got withdrawn for the provided vesting account address.
    #[cfg_attr(feature = "schema", returns(Coin))]
    GetWithdrawnCoins {
        /// Address of the vesting account in question.
        vesting_account_address: String,
    },

    /// Returns detailed information associated with the account for the provided vesting account address.
    #[cfg_attr(feature = "schema", returns(Account))]
    GetAccount {
        /// Address of the vesting account in question.
        address: String,
    },

    /// Returns pledge information (if applicable) for bonded mixnode for the provided vesting account address.
    #[cfg_attr(feature = "schema", returns(Option<PledgeData>))]
    GetMixnode {
        /// Address of the vesting account in question.
        address: String,
    },

    /// Returns pledge information (if applicable) for bonded gateway for the provided vesting account address.
    #[cfg_attr(feature = "schema", returns(Option<PledgeData>))]
    GetGateway {
        /// Address of the vesting account in question.
        address: String,
    },

    /// Returns the current vesting period for the provided vesting account address.
    #[cfg_attr(feature = "schema", returns(Period))]
    GetCurrentVestingPeriod {
        /// Address of the vesting account in question.
        address: String,
    },

    /// Returns the information about particular vesting delegation.
    #[cfg_attr(feature = "schema", returns(VestingDelegation))]
    GetDelegation {
        /// Address of the vesting account in question.
        address: String,

        /// Id of the mixnode towards which the delegation has been made.
        mix_id: NodeId,

        /// Block timestamp of the delegation.
        block_timestamp_secs: u64,
    },

    /// Returns the total amount of coins delegated towards particular mixnode by the provided vesting account address.
    #[cfg_attr(feature = "schema", returns(Coin))]
    GetTotalDelegationAmount {
        /// Address of the vesting account in question.
        address: String,

        /// Id of the mixnode towards which the delegations have been made.
        mix_id: NodeId,
    },

    /// Returns timestamps of delegations made towards particular mixnode by the provided vesting account address.
    #[cfg_attr(feature = "schema", returns(DelegationTimesResponse))]
    GetDelegationTimes {
        /// Address of the vesting account in question.
        address: String,

        /// Id of the mixnode towards which the delegations have been made.
        mix_id: NodeId,
    },

    /// Returns all active delegations made with vesting tokens stored in this contract.
    #[cfg_attr(feature = "schema", returns(AllDelegationsResponse))]
    GetAllDelegations {
        /// Pagination control for the values returned by the query. Note that the provided value itself will **not** be used for the response.
        start_after: Option<(u32, NodeId, u64)>,

        /// Controls the maximum number of entries returned by the query. Note that too large values will be overwritten by a saner default.
        limit: Option<u32>,
    },
}
