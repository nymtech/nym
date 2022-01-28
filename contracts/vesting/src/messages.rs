use cosmwasm_std::{Coin, Timestamp};
use mixnet_contract_common::{Gateway, IdentityKey, MixNode};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InitMsg {
    pub(crate) mixnet_contract_address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
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
    UpdateMixnetAddress {
        address: String,
    },
    DelegateToMixnode {
        mix_identity: IdentityKey,
        amount: Coin,
    },
    UndelegateFromMixnode {
        mix_identity: IdentityKey,
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
        mix_identity: IdentityKey,
        amount: Coin,
    },
    BondMixnode {
        mix_node: MixNode,
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
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
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
}
