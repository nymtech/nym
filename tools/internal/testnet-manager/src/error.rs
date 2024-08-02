// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only
use nym_compact_ecash::CompactEcashError;
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

    #[error("there aren't any initialised networks in the storage")]
    NoNetworksInitialised,

    #[error("you must specify at least a single api endpoint for the DKG")]
    NoApiEndpoints,

    #[error("the DKG process has already been started on the target network")]
    DkgAlreadyStarted,

    #[error("the target network is already in non-zero DKG epoch")]
    NonZeroEpoch,

    #[error("the target already has registered cw4 members")]
    ExistingCW4Members,

    #[error("failed to compute ecash keys: {source}")]
    EcashCryptoFailure {
        #[from]
        source: CompactEcashError,
    },

    #[error("the provided contract path does not point to a valid .wasm file")]
    MalformedDkgBypassContractPath,

    #[error("nym api initialisation returned non-zero return code")]
    NymApiExecutionFailure,

    #[error("nym node initialisation returned non-zero return code")]
    NymNodeExecutionFailure,

    #[error("nym client initialisation returned non-zero return code")]
    NymClientExecutionFailure,

    #[error("failed to deserialise nym-api config: {0}")]
    TomlDeserialisationFailure(#[from] toml::de::Error),

    #[error("failed to deserialise nym-node output: {0}")]
    JsonDeserialisationFailure(#[from] serde_json::Error),

    #[error(
        "the corresponding env file hasn't been generated. you need to setup local apis first."
    )]
    EnvFileNotGenerated,

    #[error("the default, pre-generated, .env file does not have the nym-api endpoint set!")]
    NymApiEndpointMissing,

    #[error("timed out while waiting for some gateway to appear in the directory (you don't need to run it)")]
    ApiGatewayWaitTimeout,

    #[error("timed out while waiting for the gateway to start receiving traffic (you need to actually run it!)")]
    GatewayWaitTimeout,
}
