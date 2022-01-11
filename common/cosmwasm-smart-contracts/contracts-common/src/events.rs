// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Event;

/// Looks up value of particular attribute in the provided event. If it fails to find it,
/// the function panics.
///
/// # Arguments
///
/// * `event`: event to search through.
/// * `key`: key associated with the particular attribute
pub fn must_find_attribute(event: &Event, key: &str) -> String {
    may_find_attribute(event, key).unwrap()
}

/// Looks up value of particular attribute in the provided event. Returns None if it does not exist.
///
/// # Arguments
///
/// * `event`: event to search through.
/// * `key`: key associated with the particular attribute
pub fn may_find_attribute(event: &Event, key: &str) -> Option<String> {
    for attr in &event.attributes {
        if attr.key == key {
            return Some(attr.value.clone());
        }
    }
    None
}
