use crate::errors::ContractError;
use cosmwasm_std::{Coin, Env, Response, Storage, Timestamp, Uint128};
use mixnet_contract_common::IdentityKey;

pub trait DelegatingAccount {
    fn try_delegate_to_mixnode(
        &self,
        mix_identity: IdentityKey,
        amount: Coin,
        env: &Env,
        storage: &mut dyn Storage,
    ) -> Result<Response, ContractError>;

    fn try_undelegate_from_mixnode(
        &self,
        mix_identity: IdentityKey,
        storage: &dyn Storage,
    ) -> Result<Response, ContractError>;

    // track_delegation performs internal vesting accounting necessary when
    // delegating from a vesting account. It accepts the current block time, the
    // delegation amount and balance of all coins whose denomination exists in
    // the account's original vesting balance.
    fn track_delegation(
        &self,
        block_time: Timestamp,
        mix_identity: IdentityKey,
        // Save some gas by passing it in
        current_balance: Uint128,
        delegation: Coin,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError>;
    // track_undelegation performs internal vesting accounting necessary when a
    // vesting account performs an undelegation.
    fn track_undelegation(
        &self,
        mix_identity: IdentityKey,
        amount: Coin,
        storage: &mut dyn Storage,
    ) -> Result<(), ContractError>;
}
