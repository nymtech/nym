// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("nym sdk")]
    NymSdkError(#[source] Box<nym_sdk::Error>),

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

// Result type based on our error type
pub type Result<T> = std::result::Result<T, Error>;
