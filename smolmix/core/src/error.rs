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

// Map smol-core stack errors onto the existing public variants, so refactoring
// smolmix onto smol-core does not change SmolmixError's public surface.
impl From<nym_smol_core::SmolCoreError> for SmolmixError {
    fn from(err: nym_smol_core::SmolCoreError) -> Self {
        match err {
            nym_smol_core::SmolCoreError::Io(e) => SmolmixError::Io(e),
            nym_smol_core::SmolCoreError::ChannelClosed => SmolmixError::ChannelClosed,
            // DNS/other stack errors surface as IO (smolmix exposes no DNS API).
            other => SmolmixError::Io(std::io::Error::other(other.to_string())),
        }
    }
}
