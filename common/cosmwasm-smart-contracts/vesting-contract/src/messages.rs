use contracts_common::signing::MessageSignature;
use cosmwasm_std::{Coin, Timestamp};
use mixnet_contract_common::families::FamilyHead;
use mixnet_contract_common::{
    gateway::GatewayConfigUpdate,
    mixnode::{MixNodeConfigUpdate, MixNodeCostParams},
    Gateway, IdentityKey, MixId, MixNode,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::PledgeCap;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InitMsg {
    pub mixnet_contract_address: String,
    pub mix_denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema, Default)]
pub struct VestingSpecification {
    start_time: Option<u64>,
    period_seconds: Option<u64>,
    num_periods: Option<u64>,
}

impl VestingSpecification {
    pub fn new(
        start_time: Option<u64>,
        period_seconds: Option<u64>,
        num_periods: Option<u64>,
    ) -> Self {
        Self {
            start_time,
            period_seconds,
            num_periods,
        }
    }

    pub fn start_time(&self) -> Option<u64> {
        self.start_time
    }

    pub fn period_seconds(&self) -> u64 {
        self.period_seconds.unwrap_or(3 * 30 * 86400)
    }

    pub fn num_periods(&self) -> u64 {
        self.num_periods.unwrap_or(8)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetContractVersion {},
    #[serde(rename = "get_cw2_contract_version")]
    GetCW2ContractVersion {},
    GetAccountsPaged {
        start_next_after: Option<String>,
        limit: Option<u32>,
    },
    GetAccountsVestingCoinsPaged {
        start_next_after: Option<String>,
        limit: Option<u32>,
    },
    LockedCoins {
        vesting_account_address: String,
        block_time: Option<Timestamp>,
    },
    SpendableCoins {
        vesting_account_address: String,
        block_time: Option<Timestamp>,
    },
    GetVestedCoins {
        vesting_account_address: String,
        block_time: Option<Timestamp>,
    },
    GetVestingCoins {
        vesting_account_address: String,
        block_time: Option<Timestamp>,
    },
    GetStartTime {
        vesting_account_address: String,
    },
    GetEndTime {
        vesting_account_address: String,
    },
    GetOriginalVesting {
        vesting_account_address: String,
    },
    GetHistoricalVestingStakingReward {
        vesting_account_address: String,
    },
    GetSpendableVestedCoins {
        vesting_account_address: String,
    },
    GetSpendableRewardCoins {
        vesting_account_address: String,
    },
    GetDelegatedCoins {
        vesting_account_address: String,
    },
    GetPledgedCoins {
        vesting_account_address: String,
    },
    GetStakedCoins {
        vesting_account_address: String,
    },
    GetWithdrawnCoins {
        vesting_account_address: String,
    },
    GetAccount {
        address: String,
    },
    GetMixnode {
        address: String,
    },
    GetGateway {
        address: String,
    },
    GetCurrentVestingPeriod {
        address: String,
    },
    GetDelegation {
        address: String,
        mix_id: MixId,
        block_timestamp_secs: u64,
    },
    GetTotalDelegationAmount {
        address: String,
        mix_id: MixId,
    },
    GetDelegationTimes {
        address: String,
        mix_id: MixId,
    },
    GetAllDelegations {
        start_after: Option<(u32, MixId, u64)>,
        limit: Option<u32>,
    },
}
