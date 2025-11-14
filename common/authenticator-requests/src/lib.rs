// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod client_message;
pub mod models;
pub mod request;
pub mod response;
pub mod traits;
pub mod v1;
pub mod v2;
pub mod v3;
pub mod v4;
pub mod v5;
pub mod v6;

mod error;
mod util;
mod version;

pub use error::Error;
pub use v6 as latest;
pub use version::AuthenticatorVersion;

pub const CURRENT_VERSION: u8 = latest::VERSION;

fn make_bincode_serializer() -> impl bincode::Options {
    use bincode::Options;
    bincode::DefaultOptions::new()
        .with_big_endian()
        .with_varint_encoding()
}
