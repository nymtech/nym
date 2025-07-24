// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::net::IpAddr;

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("peer with IP {ip} doesn't exist")]
    NoPeer { ip: IpAddr },

    #[error("peers can't be interacted with anymore")]
    PeerInteractionStopped,

    #[error("no response received")]
    NoResponse,

    #[error("query was not successful")]
    Unsuccessful,
}
