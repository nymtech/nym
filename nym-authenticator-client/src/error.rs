use nym_credentials_interface::TicketType;
use nym_sdk::mixnet::InputMessage;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("mixnet client stopped returning responses")]
    NoMixnetMessagesReceived,

    #[error("failed to send mixnet message")]
    SendMixnetMessage(#[source] Box<tokio::sync::mpsc::error::SendError<InputMessage>>),

    #[error("timeout waiting for connect response from exit gateway (authenticator)")]
    TimeoutWaitingForConnectResponse,

    #[error("unknown version number")]
    UnknownVersion,

    #[error("unsupported request version")]
    UnsupportedVersion,

    #[error(transparent)]
    Bincode(#[from] bincode::Error),

    #[error(transparent)]
    AuthenticatorRequests(#[from] nym_authenticator_requests::Error),

    #[error("verification failure")]
    VerificationFailed(#[source] nym_authenticator_requests::Error),

    #[error("failed to parse entry gateway socket addr")]
    FailedToParseEntryGatewaySocketAddr(#[source] std::net::AddrParseError),

    #[error("received invalid response from gateway authenticator")]
    InvalidGatewayAuthResponse,

    #[error("failed to get {ticketbook_type} ticket")]
    GetTicket {
        ticketbook_type: TicketType,
        #[source]
        source: nym_bandwidth_controller::error::BandwidthControllerError,
    },

    #[error("unknown authenticator version number")]
    UnsupportedAuthenticatorVersion,

    #[error("failed to wait on AuthenticatorClientListener")]
    FailedToJoinOnTask(#[from] tokio::task::JoinError),

    #[error("encountered an internal error")]
    InternalError,
}

// Result type based on our error type
pub type Result<T> = std::result::Result<T, Error>;
