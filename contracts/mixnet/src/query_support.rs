// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub(crate) const BOND_PAGE_DEFAULT_LIMIT: u32 = 50;

/// Adds a 0 byte to terminate the `start_after` value given. This allows CosmWasm
/// to get the succeeding key as the start of the next page.
// S works for both `String` and `Addr` and that's what we wanted
pub fn calculate_start_value<S: AsRef<str>>(start_after: Option<S>) -> Option<Vec<u8>> {
    start_after.as_ref().map(|identity| {
        identity
            .as_ref()
            .as_bytes()
            .iter()
            .cloned()
            .chain(std::iter::once(0))
            .collect()
    })
}
