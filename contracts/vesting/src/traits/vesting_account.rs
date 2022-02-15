use crate::errors::ContractError;
use cosmwasm_std::{Addr, Coin, Env, Storage, Timestamp};
use vesting_contract_common::OriginalVestingResponse;

pub trait VestingAccount {
    // locked_coins returns the set of coins that are not spendable (can still be delegated tough) (i.e. locked),
    // defined as the vesting coins that are not delegated or pledged.
    //
    // To get spendable coins of a vesting account, first the total balance must
    // be retrieved and the locked tokens can be subtracted from the total balance.
    // Note, the spendable balance can be negative.
    fn locked_coins(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, ContractError>;

    // Calculates the total spendable balance that can be sent to other accounts.
    fn spendable_coins(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, ContractError>;

    fn get_vested_coins(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
    ) -> Result<Coin, ContractError>;
    fn get_vesting_coins(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
    ) -> Result<Coin, ContractError>;

    fn get_start_time(&self) -> Timestamp;
    fn get_end_time(&self) -> Timestamp;

    fn get_original_vesting(&self) -> OriginalVestingResponse;
    fn get_delegated_free(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, ContractError>;
    fn get_delegated_vesting(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, ContractError>;
    fn get_pledged_free(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, ContractError>;
    fn get_pledged_vesting(
        &self,
        block_time: Option<Timestamp>,
        env: &Env,
        storage: &dyn Storage,
    ) -> Result<Coin, ContractError>;
    fn transfer_ownership(
        &mut self,
        to_address: &Addr,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError>;
    fn update_staking_address(
        &mut self,
        to_address: Option<Addr>,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError>;
}
