// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ip_packet_client::current::response::ConnectFailureReason;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("nym sdk")]
    // SdkError(#[source] Box<nym_sdk::Error>),
    SdkError(#[source] Box<crate::error::Error>),

    #[error(
        "received response with version v{received}, the client is too new and can only understand v{expected}"
    )]
    ReceivedResponseWithOldVersion { expected: u8, received: u8 },

    #[error(
        "received response with version v{received}, the client is too old and can only understand v{expected}"
    )]
    ReceivedResponseWithNewVersion { expected: u8, received: u8 },

    #[error("got reply for connect request, but it appears intended for the wrong address?")]
    GotReplyIntendedForWrongAddress,

    #[error("unexpected connect response")]
    UnexpectedConnectResponse,

    #[error("mixnet client stopped returning responses")]
    NoMixnetMessagesReceived,

    #[error("timeout waiting for connect response from exit gateway (ipr)")]
    TimeoutWaitingForConnectResponse,

    #[error("connection cancelled")]
    Cancelled,

    #[error("connect request denied: {reason}")]
    ConnectRequestDenied { reason: ConnectFailureReason },

    #[error("failed to get version from message")]
    NoVersionInMessage,

    #[error("already connected to the mixnet")]
    AlreadyConnected,

    #[error("failed to create connect request")]
    FailedToCreateConnectRequest {
        source: nym_ip_packet_requests::sign::SignatureError,
    },

    /// Below error types are from the nym-connection-monitor crate
    #[error(
        "timeout waiting for mixnet self ping, the entry gateway is not routing our mixnet traffic"
    )]
    TimeoutWaitingForMixnetSelfPing,

    #[error("failed to serialize message")]
    FailedToSerializeMessage {
        #[from]
        source: bincode::Error,
    },

    #[error("failed to create icmp echo request packet")]
    IcmpEchoRequestPacketCreationFailure,

    #[error("failed to create icmp packet")]
    IcmpPacketCreationFailure,

    #[error("failed to create ipv4 packet")]
    Ipv4PacketCreationFailure,
}

impl From<crate::error::Error> for Error {
    fn from(err: crate::error::Error) -> Self {
        Error::SdkError(Box::new(err))
    }
}

pub type Result<T> = std::result::Result<T, Error>;
