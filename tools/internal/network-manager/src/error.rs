// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only
use nym_validator_client::nyxd::error::NyxdError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NetworkManagerError {
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("failed to parse mnemonic: {0}")]
    Bip39Error(#[from] bip39::Error),

    #[error("failed to parse the url: {0}")]
    MalformedUrl(#[from] url::ParseError),

    #[error("one of the account addresses was malformed - the developer was too lazy to propagate the actual error message with the address")]
    MalformedAccountAddress,

    #[error(transparent)]
    Nyxd(#[from] NyxdError),

    #[error("you need to set the master mnemonic on initial run")]
    MnemonicNotSet,

    #[error("you need to set the rpc endpoint on initial run")]
    RpcEndpointNotSet,

    #[error("experienced internal database error: {0}")]
    InternalDatabaseError(#[from] sqlx::Error),

    #[error("failed to perform startup SQL migration - {0}")]
    StartupMigrationFailure(#[from] sqlx::migrate::MigrateError),

    #[error("could not find .wasm file for {name} contract under the provided directory")]
    ContractWasmNotFound { name: String },

    #[error("could not find code_id for {name} contract")]
    ContractNotUploaded { name: String },

    #[error("could not find contract admin for {name} contract")]
    ContractAdminNotSet { name: String },

    #[error("could not find address for {name} contract")]
    ContractNotInitialised { name: String },

    #[error("could not find build information for {name} contract")]
    ContractNotQueried { name: String },

    #[error("contract {name} has been build before build information got standarised. this is not supported")]
    MissingBuildInfo { name: String },
}
