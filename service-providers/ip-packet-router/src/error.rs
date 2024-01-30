use std::net::SocketAddr;

pub use nym_client_core::error::ClientCoreError;
use nym_exit_policy::PolicyError;

#[derive(thiserror::Error, Debug)]
pub enum IpPacketRouterError {
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("client-core error: {0}")]
    ClientCoreError(#[from] ClientCoreError),

    #[error("failed to load configuration file: {0}")]
    FailedToLoadConfig(String),

    // TODO: add more details here
    #[error("failed to validate the loaded config")]
    ConfigValidationFailure,

    #[error("failed local version check, client and config mismatch")]
    FailedLocalVersionCheck,

    #[error("failed to setup mixnet client: {source}")]
    FailedToSetupMixnetClient { source: nym_sdk::Error },

    #[error("failed to connect to mixnet: {source}")]
    FailedToConnectToMixnet { source: nym_sdk::Error },

    #[error("the entity wrapping the network requester has disconnected")]
    DisconnectedParent,

    #[error("received packet has an invalid version: {0}")]
    InvalidPacketVersion(u8),

    #[error("failed to serialize response packet: {source}")]
    FailedToSerializeResponsePacket { source: Box<bincode::ErrorKind> },

    #[error("failed to deserialize tagged packet: {source}")]
    FailedToDeserializeTaggedPacket { source: bincode::Error },

    #[error("failed to parse incoming packet: {source}")]
    PacketParseFailed { source: etherparse::ReadError },

    #[error("parsed packet is missing IP header")]
    PacketMissingIpHeader,

    #[error("parsed packet is missing transport header")]
    PacketMissingTransportHeader,

    #[error("failed to write packet to tun")]
    FailedToWritePacketToTun,

    #[error("failed to send packet to mixnet: {source}")]
    FailedToSendPacketToMixnet { source: nym_sdk::Error },

    #[error("the provided socket address, '{addr}' is not covered by the exit policy!")]
    AddressNotCoveredByExitPolicy { addr: SocketAddr },

    // #[error("the provided ip address, '{ip}' is not covered by the exit policy!")]
    // IpNotCoveredByExitPolicy { ip: IpAddr },
    #[error("failed filter check: '{addr}'")]
    AddressFailedFilterCheck { addr: SocketAddr },

    // #[error("failed filter check: '{ip}'")]
    // IpFailedFilterCheck { ip: IpAddr },
    #[error("failed to apply the exit policy: {source}")]
    ExitPolicyFailure {
        #[from]
        source: PolicyError,
    },

    #[error("the url provided for the upstream exit policy source is malformed: {source}")]
    MalformedExitPolicyUpstreamUrl {
        #[source]
        source: reqwest::Error,
    },

    #[error("can't setup an exit policy without any upstream urls")]
    NoUpstreamExitPolicy,

    #[error("no recipient in response packet")]
    NoRecipientInResponse,

    #[error("failed to update client activity")]
    FailedToUpdateClientActivity,
}

pub type Result<T> = std::result::Result<T, IpPacketRouterError>;
