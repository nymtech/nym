// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DirectoryTypesError {
    #[error("the encoded x25519 public key has invalid length. got {got} bytes and expected 32")]
    InvalidX25519KeyLength { got: usize },

    #[error(
        "the encoded service provider Recipient has invalid length. got {got} bytes and expected 96"
    )]
    InvalidRecipientLength { got: usize },
}
