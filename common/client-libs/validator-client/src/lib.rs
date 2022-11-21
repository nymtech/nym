// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod client;
#[cfg(feature = "nymd-client")]
pub mod connection_tester;
mod error;
#[cfg(feature = "nymd-client")]
pub mod nymd;
pub mod validator_api;

#[cfg(feature = "nymd-client")]
pub use crate::client::{ApiClient, CoconutApiClient};
pub use crate::error::ValidatorClientError;
pub use validator_api_requests::*;

#[cfg(feature = "nymd-client")]
pub use client::{Client, Config};
