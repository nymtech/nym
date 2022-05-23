// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::io;
use thiserror::Error;

// this would probably need to adjusted or maybe incorporated into existing stuff as originally
// this error was more extensive by being a global `DkgError`
#[derive(Error, Debug)]
pub enum NetworkingError {
    #[error("Networking / IO error - {0}")]
    Io(#[from] io::Error),

    #[error("Received message with specified size bigger than the supported maximum.  Received: {received}, supported: {supported}")]
    MessageTooLarge { supported: u64, received: u64 },

    #[error("Received message with unexpected protocol version. Received: {received}, expected: {expected}")]
    MismatchedProtocolVersion { expected: u32, received: u32 },

    #[error("Failed to deal with serialization (or deserialization) of the message - {0}")]
    SerializationError(#[from] bincode::Error),
}
