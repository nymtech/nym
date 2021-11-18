use cosmwasm_std::StdError;
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
    #[error("Insufficient balance")]
    InsufficientBalance(String, u128),
    #[error("Insufficient spendable balance")]
    InsufficientSpendable(String, u128),
    #[error("Only delegation owner can perform delegation actions, {0} is not the delegation owner")]
    NotDelegate(String),
}
