// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("nym sdk")]
    SdkError(#[source] Box<crate::error::Error>),

    #[error(
        "received response with version v{received}, the client is too new and can only understand v{expected}"
    )]
    ReceivedResponseWithOldVersion { expected: u8, received: u8 },

    #[error(
        "received response with version v{received}, the client is too old and can only understand v{expected}"
    )]
    ReceivedResponseWithNewVersion { expected: u8, received: u8 },

    #[error("failed to get version from message")]
    NoVersionInMessage,

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
