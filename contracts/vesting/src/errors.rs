use cosmwasm_std::{Addr, StdError};
use mixnet_contract_common::IdentityKey;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("VESTING ({}): {0}", line!())]
    Std(#[from] StdError),
    #[error("VESTING ({}): Account does not exist - {0}", line!())]
    NoAccountForAddress(String),
    #[error("VESTING ({}): Only admin can perform this action, {0} is not admin", line!())]
    NotAdmin(String),
    #[error("VESTING ({}): Balance not found for existing account ({0}), this is a bug", line!())]
    NoBalanceForAddress(String),
    #[error("VESTING ({}): Insufficient balance for address {0} -> {1}", line!())]
    InsufficientBalance(String, u128),
    #[error("VESTING ({}): Insufficient spendable balance for address {0} -> {1}", line!())]
    InsufficientSpendable(String, u128),
    #[error(
        "VESTING ({}):Only delegation owner can perform delegation actions, {0} is not the delegation owner"
    , line!())]
    NotDelegate(String),
    #[error("VESTING ({}): Total vesting amount is inprobably low -> {0}, this is likely an error", line!())]
    ImprobableVestingAmount(u128),
    #[error("VESTING ({}): Address {0} has already bonded a node", line!())]
    AlreadyBonded(String),
    #[error("VESTING ({}): Received empty funds vector", line!())]
    EmptyFunds,
    #[error("VESTING ({}): Received wrong denom: {0}, expected {1}", line!())]
    WrongDenom(String, String),
    #[error("VESTING ({}): Received multiple denoms, expected 1", line!())]
    MultipleDenoms,
    #[error("VESTING ({}): No delegations found for account {0}, mix_identity {1}", line!())]
    NoSuchDelegation(Addr, IdentityKey),
    #[error("VESTING ({}): Only mixnet contract can perform this operation, got {0}", line!())]
    NotMixnetContract(Addr),
    #[error("VESTING ({}): Calculation underflowed", line!())]
    Underflow,
    #[error("VESTING ({}): No bond found for account {0}", line!())]
    NoBondFound(String),
    #[error("VESTING ({}): Action can only be executed by account owner -> {0}", line!())]
    NotOwner(String),
    #[error("VESTING ({}): Invalid address: {0}", line!())]
    InvalidAddress(String),
}
