// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bincode::Options;

pub use bincode::Error as BincodeError;
pub use bincode::Options as BincodeOptions;

/// Create explicit bincode options for consistent serialization across versions.
///
/// Using explicit options future-proofs against bincode 1.x/2.x default changes.
pub fn lp_bincode_serializer() -> impl BincodeOptions {
    bincode::DefaultOptions::new()
        .with_big_endian()
        .with_varint_encoding()
}
