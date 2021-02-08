use cosmwasm_std::StdError;
use thiserror::Error;

/// Custom errors for contract failure conditions.
///
/// Add any other custom errors you like here.
/// Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Not enough funds sent for mixnode bond")]
    InsufficientBond {},

    #[error("Account does not own any mixnode bonds")]
    MixNodeBondNotFound {},

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Wrong coin denomination, you must send unym")]
    WrongDenom {},
}
