use cosmwasm_std::{Addr, StdError};
use thiserror::Error;

use crate::state::ServiceId;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("service not found: {service_id}")]
    NotFound { service_id: ServiceId },

    #[error("{sender} is not owner of service")]
    Unauthorized { sender: Addr },
}
