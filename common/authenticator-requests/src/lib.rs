// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod v1;
pub mod v2;

mod error;

pub use error::Error;
pub use v2 as latest;

pub const CURRENT_VERSION: u8 = 2;

fn make_bincode_serializer() -> impl bincode::Options {
    use bincode::Options;
    bincode::DefaultOptions::new()
        .with_big_endian()
        .with_varint_encoding()
}
