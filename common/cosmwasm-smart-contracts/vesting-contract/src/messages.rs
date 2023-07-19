// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{PledgeCap, VestingSpecification};
use contracts_common::signing::MessageSignature;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Timestamp};
use mixnet_contract_common::families::FamilyHead;
use mixnet_contract_common::{
    gateway::GatewayConfigUpdate,
    mixnode::{MixNodeConfigUpdate, MixNodeCostParams},
    Gateway, IdentityKey, MixId, MixNode,
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
    // Families
    /// Only owner of the node can crate the family with node as head
    CreateFamily {
        label: String,
    },
    /// Family head needs to sign the joining node IdentityKey, the Node provides its signature signaling consent to join the family
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
    TrackReward {
        amount: Coin,
        address: String,
    },
    ClaimOperatorReward {},
    ClaimDelegatorReward {
        mix_id: MixId,
    },
    UpdateMixnodeCostParams {
        new_costs: MixNodeCostParams,
    },
    UpdateMixnodeConfig {
        new_config: MixNodeConfigUpdate,
    },
    UpdateMixnetAddress {
        address: String,
    },
    DelegateToMixnode {
        mix_id: MixId,
        amount: Coin,
        on_behalf_of: Option<String>,
    },
    UndelegateFromMixnode {
        mix_id: MixId,
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
        mix_id: MixId,
        amount: Coin,
    },
    BondMixnode {
        mix_node: MixNode,
        cost_params: MixNodeCostParams,
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
}

impl ExecuteMsg {
    pub fn name(&self) -> &str {
        match self {
            ExecuteMsg::CreateFamily { .. } => "VestingExecuteMsg::CreateFamily",
            ExecuteMsg::JoinFamily { .. } => "VestingExecuteMsg::JoinFamily",
            ExecuteMsg::LeaveFamily { .. } => "VestingExecuteMsg::LeaveFamily",
            ExecuteMsg::KickFamilyMember { .. } => "VestingExecuteMsg::KickFamilyMember",
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
        }
    }
}

#[cw_serde]
#[cfg_attr(feature = "schema", derive(QueryResponses))]
pub enum QueryMsg {
    #[cfg_attr(feature = "schema", returns(ContractBuildInformation))]
    GetContractVersion {},

    #[serde(rename = "get_cw2_contract_version")]
    #[cfg_attr(feature = "schema", returns(cw2::ContractVersion))]
    GetCW2ContractVersion {},

    #[cfg_attr(feature = "schema", returns(AccountsResponse))]
    GetAccountsPaged {
        start_next_after: Option<String>,
        limit: Option<u32>,
    },

    #[cfg_attr(feature = "schema", returns(VestingCoinsResponse))]
    GetAccountsVestingCoinsPaged {
        start_next_after: Option<String>,
        limit: Option<u32>,
    },

    #[cfg_attr(feature = "schema", returns(Coin))]
    LockedCoins {
        vesting_account_address: String,
        block_time: Option<Timestamp>,
    },

    #[cfg_attr(feature = "schema", returns(Coin))]
    SpendableCoins {
        vesting_account_address: String,
        block_time: Option<Timestamp>,
    },

    #[cfg_attr(feature = "schema", returns(Coin))]
    GetVestedCoins {
        vesting_account_address: String,
        block_time: Option<Timestamp>,
    },

    #[cfg_attr(feature = "schema", returns(Coin))]
    GetVestingCoins {
        vesting_account_address: String,
        block_time: Option<Timestamp>,
    },

    #[cfg_attr(feature = "schema", returns(Timestamp))]
    GetStartTime { vesting_account_address: String },

    #[cfg_attr(feature = "schema", returns(Timestamp))]
    GetEndTime { vesting_account_address: String },

    #[cfg_attr(feature = "schema", returns(OriginalVestingResponse))]
    GetOriginalVesting { vesting_account_address: String },

    #[cfg_attr(feature = "schema", returns(Coin))]
    GetHistoricalVestingStakingReward { vesting_account_address: String },

    #[cfg_attr(feature = "schema", returns(Coin))]
    GetSpendableVestedCoins { vesting_account_address: String },

    #[cfg_attr(feature = "schema", returns(Coin))]
    GetSpendableRewardCoins { vesting_account_address: String },

    #[cfg_attr(feature = "schema", returns(Coin))]
    GetDelegatedCoins { vesting_account_address: String },

    #[cfg_attr(feature = "schema", returns(Coin))]
    GetPledgedCoins { vesting_account_address: String },

    #[cfg_attr(feature = "schema", returns(Coin))]
    GetStakedCoins { vesting_account_address: String },

    #[cfg_attr(feature = "schema", returns(Coin))]
    GetWithdrawnCoins { vesting_account_address: String },

    #[cfg_attr(feature = "schema", returns(Account))]
    GetAccount { address: String },

    #[cfg_attr(feature = "schema", returns(Option<PledgeData>))]
    GetMixnode { address: String },

    #[cfg_attr(feature = "schema", returns(Option<PledgeData>))]
    GetGateway { address: String },

    #[cfg_attr(feature = "schema", returns(Period))]
    GetCurrentVestingPeriod { address: String },

    #[cfg_attr(feature = "schema", returns(VestingDelegation))]
    GetDelegation {
        address: String,
        mix_id: MixId,
        block_timestamp_secs: u64,
    },

    #[cfg_attr(feature = "schema", returns(Coin))]
    GetTotalDelegationAmount { address: String, mix_id: MixId },

    #[cfg_attr(feature = "schema", returns(DelegationTimesResponse))]
    GetDelegationTimes { address: String, mix_id: MixId },

    #[cfg_attr(feature = "schema", returns(AllDelegationsResponse))]
    GetAllDelegations {
        start_after: Option<(u32, MixId, u64)>,
        limit: Option<u32>,
    },
}
