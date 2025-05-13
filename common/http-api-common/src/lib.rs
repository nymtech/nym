// Copyright 2023-2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "middleware")]
pub mod middleware;

#[cfg(feature = "output")]
pub mod response;

// don't break existing imports
#[cfg(feature = "output")]
pub use response::*;

// be explicit about those values because bincode uses different defaults in different places
#[cfg(feature = "bincode")]
pub fn make_bincode_serializer() -> impl ::bincode::Options {
    use ::bincode::Options;
    ::bincode::DefaultOptions::new()
        .with_little_endian()
        .with_varint_encoding()
}
