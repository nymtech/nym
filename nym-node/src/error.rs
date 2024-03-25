// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::config::helpers::UnsupportedGatewayAddresses;
use crate::wireguard::error::WireguardError;
use nym_ip_packet_router::error::ClientCoreError;
use nym_node_http_api::NymNodeHttpError;
use std::io;
use std::net::IpAddr;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
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
}

#[derive(Debug, Error)]
pub enum NymNodeError {
    #[error("could not find an existing config file at '{}' and fresh node initialisation has been disabled", config_path.display())]
    ForbiddenInitialisation { config_path: PathBuf },

    #[error("could not derive path to data directory of this nym node")]
    DataDirDerivationFailure,

    #[error("could not derive path to config directory of this nym node")]
    ConfigDirDerivationFailure,

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
    
    #[error("the node description file is malformed: {source}")]
    MalformedDescriptionFile {
        #[source]
        source: toml::de::Error
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

    #[error("this node attempted to announce an invalid public address: {address}. Please modify [host.public_ips] section of your config. Alternatively, if you wanted to use it in the local setting, run the node with the '--local' flag.")]
    InvalidPublicIp { address: IpAddr },

    #[error(transparent)]
    WireguardError {
        #[from]
        source: WireguardError,
    },

    #[deprecated]
    #[error(transparent)]
    KeyRecoveryError {
        #[from]
        source: nym_crypto::asymmetric::encryption::KeyRecoveryError,
    },

    #[error(transparent)]
    KeyFailure(#[from] KeyIOFailure),

    #[error("could not initialise nym-node as '--{name}' has not been specified which is required for a first time setup. (config section: {section})")]
    MissingInitArg { section: String, name: String },

    #[error("failed to migrate {node_type}: {message}")]
    MigrationFailure { node_type: String, message: String },

    #[error(transparent)]
    MixnodeFailure(#[from] MixnodeError),

    #[error(transparent)]
    EntryGatewayFailure(#[from] EntryGatewayError),

    #[error(transparent)]
    ExitGatewayFailure(#[from] ExitGatewayError),
}

impl From<nym_mixnode::error::MixnodeError> for NymNodeError {
    fn from(value: nym_mixnode::error::MixnodeError) -> Self {
        MixnodeError::from(value).into()
    }
}

#[derive(Debug, Error)]
pub enum MixnodeError {
    #[error("failed to load mixnode description from {}: {source}", path.display())]
    DescriptionLoadFailure {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    
    #[error("currently it's not supported to have different ip addresses for verloc and mixnet ({verloc_bind_ip} and {mix_bind_ip} were used)")]
    UnsupportedAddresses {
        verloc_bind_ip: IpAddr,
        mix_bind_ip: IpAddr,
    },

    #[error("mixnode failure: {0}")]
    External(#[from] nym_mixnode::error::MixnodeError),
}

#[derive(Debug, Error)]
pub enum EntryGatewayError {
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

    #[error(transparent)]
    UnsupportedAddresses(#[from] UnsupportedGatewayAddresses),

    #[error("entry gateway failure: {0}")]
    External(#[from] nym_gateway::GatewayError),
}

#[derive(Debug, Error)]
pub enum ExitGatewayError {
    #[error(transparent)]
    KeyFailure(#[from] KeyIOFailure),

    #[error(transparent)]
    UnsupportedAddresses(#[from] UnsupportedGatewayAddresses),

    // TODO: more granular errors
    #[error(transparent)]
    ExternalClientCore(#[from] ClientCoreError),

    #[error("exit gateway failure: {0}")]
    External(#[from] nym_gateway::GatewayError),
}
