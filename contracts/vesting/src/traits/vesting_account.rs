use cosmwasm_std::{Addr, Coin, Env, Storage, Timestamp, Uint128};
use vesting_contract_common::{OriginalVestingResponse, VestingContractError};

pub trait VestingAccount {
    fn total_staked(&self, storage: &dyn Storage) -> Result<Uint128, VestingContractError>;

    /// Returns the set of coins that are not spendable (can still be delegated tough) (i.e. locked),
    /// defined as vesting coins that are not delegated or pledged.
    ///
    /// To get spendable coins of a vesting account, first the total balance must
    /// be retrieved and the locked tokens can be subtracted from the total balance.
    /// Note, the spendable balance can be negative.
    /// See [/vesting-contract/struct.Account.html/method.locked_coins] for impl
    fn locked_coins(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, VestingContractError>;

    /// Calculated as current_balance minus [crate::traits::VestingAccount::locked_coins]
    /// See [/vesting-contract/struct.Account.html/method.spendable_coins] for impl
    fn spendable_coins(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, VestingContractError>;

    fn spendable_vested_coins(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, VestingContractError>;

    fn spendable_reward_coins(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, VestingContractError>;

    /// See [/vesting-contract/struct.Account.html/method.get_vested_coins] for impl
    fn get_vested_coins(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, VestingContractError>;

    /// See [/vesting-contract/struct.Account.html/method.get_vesting_coins] for impl
    fn get_vesting_coins(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, VestingContractError>;

    /// See [/vesting-contract/struct.Account.html/method.get_start_time] for impl
    fn get_start_time(&self) -> Timestamp;
    /// See [/vesting-contract/struct.Account.html/method.get_end_time] for impl
    fn get_end_time(&self) -> Timestamp;

    /// Returns amount of coins set at account creation
    /// See [/vesting-contract/struct.Account.html/method.get_original_vesting] for impl
    fn get_original_vesting(&self) -> Result<OriginalVestingResponse, VestingContractError>;

    /// See [/vesting-contract/struct.Account.html/method.transfer_ownership] for impl
    fn transfer_ownership(
        &mut self,
        to_address: &Addr,
        storage: &mut dyn Storage,
    ) -> Result<(), VestingContractError>;
    /// See [/vesting-contract/struct.Account.html/method.update_staking_address] for impl
    fn update_staking_address(
        &mut self,
        to_address: Option<Addr>,
        storage: &mut dyn Storage,
    ) -> Result<(), VestingContractError>;
    fn track_reward(
        &self,
        amount: Coin,
        storage: &mut dyn Storage,
    ) -> Result<(), VestingContractError>;
    fn get_historical_vested_staking_rewards(
        &self,
        storage: &dyn Storage,
    ) -> Result<Coin, VestingContractError>;
}
