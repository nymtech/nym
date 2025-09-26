// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-2.0-only

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SmolmixError {
    #[error("Channel closed")]
    ChannelClosed,

    #[error("Not connected to IPR")]
    NotConnected,

    #[error("Nym SDK error: {0}")]
    NymSdk(#[from] nym_sdk::Error),

    #[error("TLS handshake failed")]
    TlsHandshakeFailed,

    #[error("TLS encrypt/decrypt error")]
    TlsCrypto,

    #[error("DNS err placeholder")]
    InvalidDnsName,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
