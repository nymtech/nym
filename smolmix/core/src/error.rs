// Copyright 2024-2026 - Nym Technologies SA <contact@nymtech.net>

use thiserror::Error;

#[derive(Error, Debug)]
pub enum SmolmixError {
    #[error("Channel closed")]
    ChannelClosed,

    #[error("Not connected to IPR")]
    NotConnected,

    #[error("Nym SDK error: {0}")]
    NymSdk(#[from] nym_sdk::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
