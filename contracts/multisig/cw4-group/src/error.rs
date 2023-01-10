use cosmwasm_std::StdError;
use thiserror::Error;

use cw_controllers::{AdminError, HookError};

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Hook(#[from] HookError),

    #[error(transparent)]
    Admin(#[from] AdminError),

    #[error("Unauthorized")]
    Unauthorized {},
}
