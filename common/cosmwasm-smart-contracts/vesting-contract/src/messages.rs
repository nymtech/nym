use cosmwasm_std::{Coin, Timestamp, Uint128};
use mixnet_contract_common::{
    mixnode::{MixNodeConfigUpdate, MixNodeCostParams},
    Gateway, MixNode, NodeId,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InitMsg {
    pub mixnet_contract_address: String,
    pub mix_denom: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {
    pub mix_denom: String,
}

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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
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
        new_costs: MixNodeCostParams,
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
    },
    UndelegateFromMixnode {
        mix_id: NodeId,
    },
    CreateAccount {
        owner_address: String,
        staking_address: Option<String>,
        vesting_spec: Option<VestingSpecification>,
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
        cost_params: MixNodeCostParams,
        owner_signature: String,
        amount: Coin,
    },
    UnbondMixnode {},
    TrackUnbondMixnode {
        owner: String,
        amount: Coin,
    },
    BondGateway {
        gateway: Gateway,
        owner_signature: String,
        amount: Coin,
    },
    UnbondGateway {},
    TrackUnbondGateway {
        owner: String,
        amount: Coin,
    },
    TransferOwnership {
        to_address: String,
    },
    UpdateStakingAddress {
        to_address: Option<String>,
    },
    UpdateLockedPledgeCap {
        amount: Uint128,
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
            ExecuteMsg::UnbondMixnode { .. } => "VestingExecuteMsg::UnbondMixnode",
            ExecuteMsg::TrackUnbondMixnode { .. } => "VestingExecuteMsg::TrackUnbondMixnode",
            ExecuteMsg::BondGateway { .. } => "VestingExecuteMsg::BondGateway",
            ExecuteMsg::UnbondGateway { .. } => "VestingExecuteMsg::UnbondGateway",
            ExecuteMsg::TrackUnbondGateway { .. } => "VestingExecuteMsg::TrackUnbondGateway",
            ExecuteMsg::TransferOwnership { .. } => "VestingExecuteMsg::TransferOwnership",
            ExecuteMsg::UpdateStakingAddress { .. } => "VestingExecuteMsg::UpdateStakingAddress",
            ExecuteMsg::UpdateLockedPledgeCap { .. } => "VestingExecuteMsg::UpdateLockedPledgeCap",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
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
    GetDelegatedFree {
        block_time: Option<Timestamp>,
        vesting_account_address: String,
    },
    GetDelegatedVesting {
        block_time: Option<Timestamp>,
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
    GetLockedPledgeCap {},
}
