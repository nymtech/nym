use cosmwasm_std::{Addr, StdError};
use cw_controllers::AdminError;
use nym_name_service_common::{NameId, NymAddress, NymName};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum NameServiceError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    AdminError(#[from] AdminError),

    #[error("name id entry not found: {name_id}")]
    NotFound { name_id: NameId },

    #[error("name entry not found: {name}")]
    NameNotFound { name: NymName },

    #[error("{sender} is not registrator of name")]
    Unauthorized { sender: Addr },

    #[error("deposit required to register a name")]
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

    #[error("reached the max number of names ({max_names}) for owner {owner}")]
    ReachedMaxNamesForOwner { max_names: u32, owner: Addr },

    #[error("reached the max number of names ({max_names}) for nym address {0}", nym_address.to_string())]
    ReachedMaxNamesForNymAddress {
        max_names: u32,
        nym_address: NymAddress,
    },

    #[error("failed to parse {value} into a valid SemVer version: {error_message}")]
    SemVerFailure {
        value: String,
        error_message: String,
    },

    #[error("duplicate entries detected for name: {name}")]
    DuplicateNames { name: NymName },

    #[error("name already registered: {name}")]
    NameAlreadyRegistered { name: NymName },
}

pub(crate) type Result<T, E = NameServiceError> = std::result::Result<T, E>;
