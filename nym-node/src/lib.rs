// Copyright 2023-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]

// this crate will eventually get converted into proper binary

pub mod config;
pub mod error;
pub mod http;
pub mod wireguard;
