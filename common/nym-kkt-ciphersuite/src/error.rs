// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum KKTCiphersuiteError {
    #[error(
        "attempted to use an insecure encapsulation key hash length. requested: {requested}. minimum: {minimum}"
    )]
    InsecureHashLen { requested: u8, minimum: u8 },

    #[error("{raw} does not correspond to any known KEM type encoding")]
    UnknownKEMType { raw: u8 },

    #[error("{raw} does not correspond to any known Hash Function type encoding")]
    UnknownHashFunctionType { raw: u8 },

    #[error("{raw} does not correspond to any known Signature Scheme type encoding")]
    UnknownSignatureSchemeType { raw: u8 },
}
