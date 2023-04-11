// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub mod client;
#[cfg(feature = "nyxd-client")]
pub mod connection_tester;
pub mod error;
pub mod nym_api;
#[cfg(feature = "nyxd-client")]
pub mod nyxd;

#[cfg(feature = "signing")]
pub mod signing;

pub use crate::error::ValidatorClientError;
pub use nym_api_requests::*;

#[cfg(feature = "nyxd-client")]
pub use client::{Client, CoconutApiClient, Config, NymApiClient};
