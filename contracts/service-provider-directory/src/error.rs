use cosmwasm_std::{Addr, StdError};
use cw_controllers::AdminError;
use nym_service_provider_directory_common::{NymAddress, ServiceId};
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

    #[error("reached the max number of providers ({max_providers}) for owner {owner}")]
    ReachedMaxProvidersForAdmin { max_providers: u32, owner: Addr },

    #[error("reached the max number of aliases ({max_aliases}) for nym address {0}", nym_address.to_string())]
    ReachedMaxAliasesForNymAddress {
        max_aliases: u32,
        nym_address: NymAddress,
    },
}

pub(crate) type Result<T, E = ContractError> = std::result::Result<T, E>;
