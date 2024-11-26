// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

pub(crate) mod api_routes;
pub(crate) mod client;
pub(crate) mod comm;
mod deposit;
pub(crate) mod dkg;
pub(crate) mod error;
pub(crate) mod helpers;
pub(crate) mod keys;
pub(crate) mod state;
pub(crate) mod storage;
#[cfg(test)]
pub(crate) mod tests;

// equivalent of 100nym
pub(crate) const MINIMUM_BALANCE: u128 = 100_000000;
