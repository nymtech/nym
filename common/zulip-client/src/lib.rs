// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

pub mod client;
pub mod error;
pub mod message;

pub type Id = u32;

pub use client::{Client, ClientBuilder};
pub use error::ZulipClientError;
