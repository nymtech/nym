use cosmwasm_std::{Addr, StdError};
use cw_controllers::AdminError;
use nym_contracts_common::signing::verifier::ApiVerifierError;
use thiserror::Error;

use crate::{Address, NameId, NymName};

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

    #[error("reached the max number of names ({max_names}) for address {0}", address.to_string())]
    ReachedMaxNamesForAddress { max_names: u32, address: Address },

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

    #[error("duplicate entries detected for name: {name}")]
    DuplicateNames { name: NymName },

    #[error("name already registered: {name}")]
    NameAlreadyRegistered { name: NymName },
}

pub type Result<T, E = NameServiceError> = std::result::Result<T, E>;
