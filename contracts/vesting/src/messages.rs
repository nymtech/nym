use cosmwasm_std::{Addr, Coin, Timestamp};
use mixnet_contract::IdentityKey;
use mixnet_contract::MixNode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InitMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    DelegateToMixnode {
        mix_identity: IdentityKey,
    },
    UndelegateFromMixnode {
        mix_identity: IdentityKey,
    },
    CreateAccount {
        address: String,
        start_time: Option<u64>,
    },
    WithdrawVestedCoins {
        amount: Coin,
    },
    TrackUndelegation {
        owner: Addr,
        mix_identity: IdentityKey,
        amount: Coin,
    },
    BondMixnode {
        mix_node: MixNode,
    },
    UnbondMixnode {},
    TrackUnbond {
        owner: Addr,
        amount: Coin,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    LockedCoins {
        vesting_account_address: Addr,
        block_time: Option<Timestamp>,
    },
    SpendableCoins {
        vesting_account_address: Addr,
        block_time: Option<Timestamp>,
    },
    GetVestedCoins {
        vesting_account_address: Addr,
        block_time: Option<Timestamp>,
    },
    GetVestingCoins {
        vesting_account_address: Addr,
        block_time: Option<Timestamp>,
    },
    GetStartTime {
        vesting_account_address: Addr,
    },
    GetEndTime {
        vesting_account_address: Addr,
    },
    GetOriginalVesting {
        vesting_account_address: Addr,
    },
    GetDelegatedFree {
        block_time: Option<Timestamp>,
        vesting_account_address: Addr,
    },
    GetDelegatedVesting {
        block_time: Option<Timestamp>,
        vesting_account_address: Addr,
    },
}
