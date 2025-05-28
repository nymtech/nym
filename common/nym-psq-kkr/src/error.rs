// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Error, Debug)]
pub enum KKTError {
    #[error("Signature constructor error")]
    SigConstructorError,

    #[error("Signature verification error")]
    SigVerifError,
    // #[error("Protocol did not complete")]
    // ProtocolError,

    // #[error("encountered an IO error: {0}")]
    // IoError(#[from] io::Error),

    // #[error("Handshake timeout")]
    // HandshakeTimeout(#[from] tokio::time::error::Elapsed),
}
