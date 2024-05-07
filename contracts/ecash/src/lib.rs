// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![warn(clippy::expect_used)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::todo)]
#![warn(clippy::dbg_macro)]

pub mod contract;
mod deposit;
mod helpers;
#[cfg(test)]
pub mod multitest;
mod support;
