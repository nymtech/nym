use cosmwasm_std::{Addr, StdError};
use cw_controllers::AdminError;
use nym_service_provider_directory_common::ServiceId;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    AdminError(#[from] AdminError),

    #[error("service not found: {service_id}")]
    NotFound { service_id: ServiceId },

    #[error("{sender} is not owner of service")]
    Unauthorized { sender: Addr },

    #[error("deposit required to announce service")]
    DepositRequired { source: cw_utils::PaymentError },

    #[error("insufficiant deposit: {funds}, required: {deposit_required}")]
    InsufficientDeposit {
        funds: cosmwasm_std::Uint128,
        deposit_required: cosmwasm_std::Uint128,
    },

    #[error("deposit too large: {funds}, required: {deposit_required}")]
    TooLargeDeposit {
        funds: cosmwasm_std::Uint128,
        deposit_required: cosmwasm_std::Uint128,
    },
}

pub(crate) type Result<T, E = ContractError> = std::result::Result<T, E>;
