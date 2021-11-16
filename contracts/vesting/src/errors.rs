use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("Account does not exist - {0}")]
    NoSuchAccount(String),
    #[error("Only admin can perform this action, {0} is not admin")]
    NotAdmin(String),
}
