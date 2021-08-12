// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod client;
mod error;
#[cfg(feature = "nymd-client")]
pub mod nymd;
pub mod validator_api;

pub use crate::client::ApiClient;
pub use crate::error::ValidatorClientError;

#[cfg(feature = "nymd-client")]
pub use client::{Client, Config};
