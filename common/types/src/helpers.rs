// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Serialize, Deserialize)]
pub struct ConsoleSigningOutput {
    pub encoded_message: String,
    pub encoded_signature: String,
}

impl ConsoleSigningOutput {
    pub fn new(encoded_message: impl Into<String>, encoded_signature: impl Into<String>) -> Self {
        Self {
            encoded_message: encoded_message.into(),
            encoded_signature: encoded_signature.into(),
        }
    }
}

impl Display for ConsoleSigningOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the base58-encoded signature on: '{}' is:\n{}",
            self.encoded_message, self.encoded_signature
        )
    }
}
