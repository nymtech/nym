use cosmwasm_std::{Addr, StdError};
use cw_controllers::AdminError;
use nym_contracts_common::signing::verifier::ApiVerifierError;
use thiserror::Error;

use crate::{NymAddress, ServiceId};

#[derive(Error, Debug, PartialEq)]
pub enum SpContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    AdminError(#[from] AdminError),

    #[error("service not found: {service_id}")]
    NotFound { service_id: ServiceId },

    #[error("{sender} is not announcer of service")]
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

    #[error("reached the max number of providers ({max_providers}) for announcer {announcer}")]
    ReachedMaxProvidersForAdmin { max_providers: u32, announcer: Addr },

    #[error("reached the max number of aliases ({max_aliases}) for nym address {0}", nym_address.to_string())]
    ReachedMaxAliasesForNymAddress {
        max_aliases: u32,
        nym_address: NymAddress,
    },

    #[error("failed to parse {value} into a valid SemVer version: {error_message}")]
    SemVerFailure {
        value: String,
        error_message: String,
    },

    #[error("Failed to recover ed25519 public key from its base58 representation - {0}")]
    MalformedEd25519IdentityKey(String),

    #[error("Failed to recover ed25519 signature from its base58 representation - {0}")]
    MalformedEd25519Signature(String),

    #[error("Provided ed25519 signature did not verify correctly")]
    InvalidEd25519Signature,

    #[error("failed to verify message signature: {source}")]
    SignatureVerificationFailure {
        #[from]
        source: ApiVerifierError,
    },
}

pub type Result<T, E = SpContractError> = std::result::Result<T, E>;
