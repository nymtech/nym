#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("mixnet client stopped returning responses")]
    NoMixnetMessagesReceived,

    #[error("failed to get version from message")]
    NoVersionInMessage,

    #[error(
        "received response with version v{received}, the client is too new and can only understand v{expected}"
    )]
    ReceivedResponseWithOldVersion { expected: u8, received: u8 },

    #[error(
        "received response with version v{received}, the client is too old and can only understand v{expected}"
    )]
    ReceivedResponseWithNewVersion { expected: u8, received: u8 },

    #[error("failed to send mixnet message")]
    SendMixnetMessage(#[source] Box<nym_sdk::Error>),

    #[error("timeout waiting for connect response from exit gateway (authenticator)")]
    TimeoutWaitingForConnectResponse,

    #[error("unable to get mixnet handle when sending authenticator message")]
    UnableToGetMixnetHandle,

    #[error("unknown version number")]
    UnknownVersion,

    #[error(transparent)]
    Bincode(#[from] bincode::Error),

    #[error("gateway doesn't support this type of message")]
    UnsupportedMessage,

    #[error(transparent)]
    AuthenticatorRequests(#[from] nym_authenticator_requests::Error),
}

// Result type based on our error type
pub type Result<T> = std::result::Result<T, Error>;
