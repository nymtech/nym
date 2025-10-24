use nym_validator_client::nyxd::error::NyxdError;
use std::path::PathBuf;

use nym_ip_packet_requests::v8::response::{ConnectFailureReason, IpPacketResponseData};
use nym_validator_client::nym_api::error::NymAPIError;

/// Top-level Error enum for the mixnet client and its relevant types.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("i/o error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("toml serialization error: {0}")]
    TomlSerializationError(#[from] toml::ser::Error),

    #[error("toml deserialization error: {0}")]
    TomlDeserializationError(#[from] toml::de::Error),

    #[error("Ed25519 error: {0}")]
    Ed25519RecoveryError(#[from] nym_crypto::asymmetric::ed25519::Ed25519RecoveryError),

    #[error(transparent)]
    ClientCoreError(#[from] nym_client_core::error::ClientCoreError),

    #[error("key file encountered that we don't want to overwrite: {0}")]
    DontOverwrite(PathBuf),

    #[error("shared gateway key file encountered that we don't want to overwrite: {0}")]
    DontOverwriteGatewayKey(PathBuf),

    #[error("no gateway config available for writing")]
    GatewayNotAvailableForWriting,

    #[error("expected to received a directory, received: {0}")]
    ExpectedDirectory(PathBuf),

    #[error("failed to transition to registered state before connection to mixnet")]
    FailedToTransitionToRegisteredState,

    #[error(
        "registering with gateway when the client is already in a registered state is not \
         supported, and likely and user mistake"
    )]
    ReregisteringGatewayNotSupported,

    #[error("no gateway key set")]
    NoGatewayKeySet,

    #[error("credentials mode not enabled")]
    DisabledCredentialsMode,

    #[error("bad validator details: {0}")]
    BadValidatorDetails(#[from] NyxdError),

    #[error("socks5 configuration set: {}, but expected to be {}", set, !set)]
    Socks5Config { set: bool },

    #[error("socks5 channel could not be started")]
    Socks5NotStarted,

    #[error("bandwidth controller error: {0}")]
    BandwidthControllerError(#[from] nym_bandwidth_controller::error::BandwidthControllerError),

    #[error("invalid voucher blob")]
    InvalidVoucherBlob,

    #[error("invalid mnemonic: {0}")]
    InvalidMnemonic(#[from] bip39::Error),

    #[error("failed to use reply storage backend: {source}")]
    ReplyStorageError {
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("failed to use key storage backend: {source}")]
    KeyStorageError {
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("failed to use credential storage backend: {source}")]
    CredentialStorageError {
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error(transparent)]
    CredentialIssuanceError {
        #[from]
        source: nym_credential_utils::Error,
    },

    #[error("loaded shared gateway key without providing information about what gateway it corresponds to")]
    GatewayWithUnknownEndpoint,

    #[error("failed to send the provided message")]
    MessageSendingFailure,

    #[error("this operation is currently unsupported: {details}")]
    Unsupported { details: String },

    #[error(transparent)]
    Bincode(#[from] bincode::Error),

    #[error("Failed to get shutdown tracker from the task runtime registry: {0}")]
    RegistryAccess(#[from] nym_task::RegistryAccessError),
    #[error("nymsphinx receiver error: {0}")]
    MessageRecovery(#[from] nym_sphinx::receiver::MessageRecoveryError),

    #[error("client not connected")]
    IprStreamClientNotConnected,

    #[error("client already connected or connecting")]
    IprStreamClientAlreadyConnectedOrConnecting,

    #[error("listening for connection response timed out")]
    IPRConnectResponseTimeout,

    #[error("no next frame: assuming stream is closed")]
    IPRClientStreamClosed,

    #[error("expected control response, got {0:?}")]
    UnexpectedResponseType(IpPacketResponseData),

    #[error("connect denied: {0:?}")]
    ConnectDenied(ConnectFailureReason),

    #[error("api directory error: {0}")]
    GatewayDirectoryError(#[from] NymAPIError),

    #[error("did not receive Validator endpoint details")]
    NoValidatorDetailsAvailable,

    #[error("did not receive URL")]
    NoValidatorAPIUrl,

    #[error("did not receive NymVPN API URL")]
    NoNymAPIUrl,

    #[error("no available gateway")]
    NoGatewayAvailable,

    #[error("no IPR address on selected gateway")]
    NoIPRAvailable,
}

impl Error {
    pub fn new_unsupported<S: Into<String>>(details: S) -> Self {
        Error::Unsupported {
            details: details.into(),
        }
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
