// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::http::error::NymNodeHttpError;
use crate::wireguard::error::WireguardError;
use nym_http_api_client::HttpClientError;
use nym_ip_packet_router::error::ClientCoreError;
use nym_validator_client::nyxd::error::NyxdError;
use nym_validator_client::ValidatorClientError;
use std::io;
use std::net::IpAddr;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
#[allow(clippy::enum_variant_names)]
pub enum KeyIOFailure {
    #[error("failed to load {keys} keys from {:?} (private key) and {:?} (public key): {err}", .paths.private_key_path, .paths.public_key_path)]
    KeyPairLoadFailure {
        keys: String,
        paths: nym_pemstore::KeyPairPath,
        #[source]
        err: io::Error,
    },

    #[error("failed to load {key} key from '{}': {err}", path.display())]
    KeyLoadFailure {
        key: String,
        path: PathBuf,
        #[source]
        err: io::Error,
    },

    #[error("failed to store {keys} keys to {:?} (private key) and {:?} (public key): {err}", .paths.private_key_path, .paths.public_key_path)]
    KeyPairStoreFailure {
        keys: String,
        paths: nym_pemstore::KeyPairPath,
        #[source]
        err: io::Error,
    },

    #[error("failed to store {key} key to '{}': {err}", path.display())]
    KeyStoreFailure {
        key: String,
        path: PathBuf,
        #[source]
        err: io::Error,
    },

    #[error("failed to move {key} key from '{}' to '{}': {err}", source.display(), destination.display())]
    KeyMoveFailure {
        key: String,
        source: PathBuf,
        destination: PathBuf,
        #[source]
        err: io::Error,
    },

    #[error("failed to copy {key} key from '{}' to '{}': {err}", source.display(), destination.display())]
    KeyCopyFailure {
        key: String,
        source: PathBuf,
        destination: PathBuf,
        #[source]
        err: io::Error,
    },

    #[error("failed to remove {key} key from '{}': {err}", path.display())]
    KeyRemovalFailure {
        key: String,
        path: PathBuf,
        #[source]
        err: io::Error,
    },
}

#[derive(Debug, Error)]
pub enum NymNodeError {
    #[error("this binary version no longer supports migration from legacy mixnodes and gateways")]
    UnsupportedMigration,

    #[error("failed to initialise shutdown signals: {source}")]
    ShutdownSignalFailure {
        #[source]
        source: io::Error,
    },

    #[error("could not find an existing config file at '{}' and fresh node initialisation has been disabled", config_path.display())]
    ForbiddenInitialisation { config_path: PathBuf },

    #[error("could not derive path to data directory of this nym node")]
    DataDirDerivationFailure,

    #[error(transparent)]
    HttpFailure(#[from] NymNodeHttpError),

    #[error("failed to load config file using path '{}'. detailed message: {source}", path.display())]
    ConfigLoadFailure {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to save config file for id {id} using path '{}'. detailed message: {source}", path.display())]
    ConfigSaveFailure {
        id: String,
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to validate loaded config: {error}")]
    ConfigValidationFailure { error: String },

    #[error("the node description file is malformed: {source}")]
    MalformedDescriptionFile {
        #[source]
        source: toml::de::Error,
    },

    #[error("failed to load description file using path '{}'. detailed message: {source}", path.display())]
    DescriptionLoadFailure {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to save description file using path '{}'. detailed message: {source}", path.display())]
    DescriptionSaveFailure {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to write bonding information to '{}': {source}", path.display())]
    BondingInfoWriteFailure {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("this node hasn't set any valid public addresses to announce. Please modify [host.public_ips] section of your config")]
    NoPublicIps,

    #[error("there are no available nym api endpoints")]
    NoNymApiUrls,

    #[error("failed to resolve nym-api query - no nodes returned a valid response")]
    NymApisExhausted,

    #[error("failed to resolve chain query: {0}")]
    NyxdFailure(#[from] NyxdError),

    #[error("this node attempted to announce an invalid public address: {address}. Please modify [host.public_ips] section of your config. Alternatively, if you wanted to use it in the local setting, run the node with the '--local' flag.")]
    InvalidPublicIp { address: IpAddr },

    #[error(transparent)]
    WireguardError {
        #[from]
        source: WireguardError,
    },

    #[error("wireguard data is no longer available - has it been reused?")]
    WireguardDataUnavailable,

    #[deprecated]
    #[error(transparent)]
    KeyRecoveryError {
        #[from]
        source: nym_crypto::asymmetric::x25519::KeyRecoveryError,
    },

    #[error(transparent)]
    KeyFailure(#[from] KeyIOFailure),

    #[error("could not initialise nym-node as '--{name}' has not been specified which is required for a first time setup. (config section: {section})")]
    MissingInitArg { section: String, name: String },

    #[error("there was an issue with wireguard IP network: {source}")]
    IpNetworkError {
        #[from]
        source: ipnetwork::IpNetworkError,
    },

    #[error(
        "failed to retrieve initial network topology - can't start the node without it: {source}"
    )]
    InitialTopologyQueryFailure { source: ValidatorClientError },

    #[error("experienced critical failure with the replay detection bloomfilter: {message}")]
    BloomfilterFailure { message: &'static str },

    #[error("failed to save/load the bloomfilter: {source} using path: {}", path.display())]
    BloomfilterIoFailure { source: io::Error, path: PathBuf },

    #[error(transparent)]
    GatewayFailure(Box<nym_gateway::GatewayError>),

    #[error(transparent)]
    GatewayTasksStartupFailure(Box<dyn std::error::Error + Send + Sync>),

    #[error(transparent)]
    EntryGatewayFailure(Box<EntryGatewayError>),

    #[error(transparent)]
    ServiceProvidersFailure(#[from] ServiceProvidersError),

    // TODO: more granular errors
    #[error(transparent)]
    ExternalClientCore(#[from] ClientCoreError),

    #[error("failed upgrade")]
    FailedUpgrade,
}

impl From<EntryGatewayError> for NymNodeError {
    fn from(error: EntryGatewayError) -> Self {
        NymNodeError::EntryGatewayFailure(Box::new(error))
    }
}

impl From<nym_gateway::GatewayError> for NymNodeError {
    fn from(error: nym_gateway::GatewayError) -> Self {
        NymNodeError::GatewayFailure(Box::new(error))
    }
}

impl NymNodeError {
    pub fn config_validation_failure<S: Into<String>>(error: S) -> Self {
        NymNodeError::ConfigValidationFailure {
            error: error.into(),
        }
    }

    pub fn bloomfilter_failure(message: &'static str) -> Self {
        NymNodeError::BloomfilterFailure { message }
    }
}

#[derive(Debug, Error)]
pub enum EntryGatewayError {
    #[error(transparent)]
    KeyFailure(#[from] KeyIOFailure),

    // TODO: more granular errors
    #[error(transparent)]
    ExternalClientCore(#[from] ClientCoreError),

    #[error("failed to load entry gateway account mnemonic from {}: {source}", path.display())]
    MnemonicLoadFailure {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to save entry gateway account mnemonic from {}: {source}", path.display())]
    MnemonicSaveFailure {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("the stored mnemonic is malformed: {source}")]
    MalformedBip39Mnemonic {
        #[from]
        source: bip39::Error,
    },

    #[error("entry gateway failure: {0}")]
    External(Box<nym_gateway::GatewayError>),
}

impl From<nym_gateway::GatewayError> for EntryGatewayError {
    fn from(error: nym_gateway::GatewayError) -> Self {
        EntryGatewayError::External(Box::new(error))
    }
}

#[derive(Debug, Error)]
pub enum ServiceProvidersError {
    #[error(transparent)]
    KeyFailure(#[from] KeyIOFailure),

    // TODO: more granular errors
    #[error(transparent)]
    ExternalClientCore(#[from] ClientCoreError),
}

impl From<HttpClientError> for NymNodeError {
    fn from(value: HttpClientError) -> Self {
        Self::HttpFailure(NymNodeHttpError::ClientError { source: value })
    }
}
