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

    #[error("deposit required to announce service")]
    DepositRequired { source: cw_utils::PaymentError },

    #[error("insufficiant deposit: {funds}, required: {deposit_required}")]
    InsufficientDeposit {
        funds: cosmwasm_std::Uint128,
        deposit_required: cosmwasm_std::Coin,
    },

    #[error("deposit too large: {funds}, required: {deposit_required}")]
    TooLargeDeposit { funds: cosmwasm_std::Uint128, deposit_required: cosmwasm_std::Coin },
}
