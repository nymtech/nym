use crate::vesting::VestingPeriod;
use cosmwasm_std::{Addr, Coin, Timestamp};
use mixnet_contract::IdentityKey;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ExecuteMsg {
    DelegateToMixnode {
        mix_identity: IdentityKey,
    },
    UndelegateFromMixnode {
        mix_identity: IdentityKey,
    },
    CreatePeriodicVestingAccount {
        address: Addr,
        coins: Vec<Coin>,
        start_time: Option<u64>,
        periods: Option<Vec<VestingPeriod>>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum QueryMsg {
    LockedCoins { block_time: Option<Timestamp> },
    SpendableCoins { block_time: Option<Timestamp> },
    GetVestedCoins { block_time: Option<Timestamp> },
    GetVestingCoins { block_time: Option<Timestamp> },
    GetStartTime,
    GetEndTime,
    GetOriginalVesting,
    GetDelegatedFree,
    GetDelegatedVesting,
}
