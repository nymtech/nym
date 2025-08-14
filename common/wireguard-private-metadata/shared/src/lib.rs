// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod error;
mod models;
pub mod routes;

#[cfg(feature = "testing")]
pub use models::v0;
pub use models::{
    interface, latest, v1, AxumErrorResponse, AxumResult, Construct, ErrorResponse, Extract,
    Request, Response, Version,
};

fn make_bincode_serializer() -> impl bincode::Options {
    use bincode::Options;
    bincode::DefaultOptions::new()
        .with_big_endian()
        .with_varint_encoding()
}
