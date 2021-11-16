use cosmwasm_std::{Addr, Coin, DepsMut, Env, Timestamp};
use mixnet_contract::IdentityKey;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

trait VestingAccount {
    // locked_coins returns the set of coins that are not spendable (i.e. locked),
    // defined as the vesting coins that are not delegated.
    //
    // To get spendable coins of a vesting account, first the total balance must
    // be retrieved and the locked tokens can be subtracted from the total balance.
    // Note, the spendable balance can be negative.
    fn locked_coins(&self, block_time: Option<Timestamp>, env: Env, deps: DepsMut) -> Vec<Coin>;

    // Calculates the total spendable balance that can be sent to other accounts.
    fn spendable_coins(&self, block_time: Option<Timestamp>, env: Env, deps: DepsMut) -> Vec<Coin>;

    // track_delegation performs internal vesting accounting necessary when
    // delegating from a vesting account. It accepts the current block time, the
    // delegation amount and balance of all coins whose denomination exists in
    // the account's original vesting balance.
    fn track_delegation(
        &self,
        block_time: Timestamp,
        delegation_amount: Vec<Coin>,
        original_vesting_balance: Vec<Coin>,
        env: Env,
        deps: DepsMut,
    );
    // track_undelegation performs internal vesting accounting necessary when a
    // vesting account performs an undelegation.
    fn track_undelegation(&self, delegation_amount: Vec<Coin>, env: Env, deps: DepsMut);

    fn get_vested_coins(&self, block_time: Timestamp, env: Env, deps: DepsMut) -> Vec<Coin>;
    fn get_vesting_coins(&self, block_time: Timestamp, env: Env, deps: DepsMut) -> Vec<Coin>;

    fn get_start_time(&self, env: Env, deps: DepsMut) -> Timestamp;
    fn get_end_time(&self, env: Env, deps: DepsMut) -> Timestamp;

    fn get_original_vesting(&self, env: Env, deps: DepsMut) -> Vec<Coin>;
    fn get_delegated_free(&self, env: Env, deps: DepsMut) -> Vec<Coin>;
    fn get_delegated_vesting(&self, env: Env, deps: DepsMut) -> Vec<Coin>;
}

trait DelegationAccount {
    fn try_delegate_to_mixnode(mix_identity: IdentityKey, amount: Vec<Coin>);
    fn try_undelegate_from_mixnode(mix_identity: IdentityKey);
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct VestingPeriod {
    secs: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PeriodicVestingAccount {
    address: Addr,
    start_time: u64,
    periods: Vec<VestingPeriod>,
}

impl Default for VestingPeriod {
    fn default() -> Self {
        // 90 days
        VestingPeriod { secs: 90 * 86400 }
    }
}

impl PeriodicVestingAccount {
    pub fn new(
        address: Addr,
        coins: Vec<Coin>,
        start_time: u64,
        periods: Vec<VestingPeriod>,
    ) -> Self {
        unimplemented!()
    }
}
