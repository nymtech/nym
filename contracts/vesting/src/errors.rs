use cosmwasm_std::{Addr, StdError};
use mixnet_contract::IdentityKey;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("Account does not exist - {0}")]
    NoAccountForAddress(String),
    #[error("Only admin can perform this action, {0} is not admin")]
    NotAdmin(String),
    #[error("Balance not found for existing account ({0}), this is a bug")]
    NoBalanceForAddress(String),
    #[error("Insufficient balance for address {0} -> {1}")]
    InsufficientBalance(String, u128),
    #[error("Insufficient spendable balance for address {0} -> {1}")]
    InsufficientSpendable(String, u128),
    #[error(
        "Only delegation owner can perform delegation actions, {0} is not the delegation owner"
    )]
    NotDelegate(String),
    #[error("Total vesting amount is inprobably low -> {0}, this is likely an error")]
    ImprobableVestingAmount(u128),
    #[error("Address {0} has already bonded a node")]
    AlreadyBonded(String),
    #[error("Recieved empty funds vector")]
    EmptyFunds,
    #[error("Recieved wrong denom: {0}, expected {1}")]
    WrongDenom(String, String),
    #[error("Recieved multiple denoms, expected 1")]
    MultipleDenoms,
    #[error("No delegations found for account {0}, mix_identity {1}")]
    NoSuchDelegation(Addr, IdentityKey),
    #[error("Only mixnet contract can perform this operation, got {0}")]
    NotMixnetContract(Addr),
    #[error("Calculation underflowed")]
    Underflow,
    #[error("No bond found for account {0}")]
    NoBondFound(String),
    #[error("Action can only be executed by account owner -> {0}")]
    NotOwner(String),
    #[error("Invalid address: {0}")]
    InvalidAddress(String),
}
