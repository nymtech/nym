// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Event;
use std::str::FromStr;

/// Looks up value of particular attribute in the provided event. If it fails to find it,
/// the function panics.
///
/// # Arguments
///
/// * `event`: event to search through.
/// * `key`: key associated with the particular attribute
pub fn must_find_attribute(event: &Event, key: &str) -> String {
    // due to how the function is supposed to work, the unwrap is fine in this instance
    #[allow(clippy::unwrap_used)]
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

pub fn try_find_attribute<T, E>(
    events: &[Event],
    event_name: &str,
    key: &str,
) -> Option<Result<T, E>>
where
    T: FromStr<Err = E>,
{
    for event in events {
        if event.ty == event_name {
            let value = may_find_attribute(event, key)?;
            return Some(value.parse());
        }
    }
    None
}

pub trait OptionallyAddAttribute {
    fn add_optional_attribute(
        self,
        key: impl Into<String>,
        value: Option<impl Into<String>>,
    ) -> Self;
}

impl OptionallyAddAttribute for Event {
    fn add_optional_attribute(
        self,
        key: impl Into<String>,
        value: Option<impl Into<String>>,
    ) -> Self {
        if let Some(value) = value {
            self.add_attribute(key, value)
        } else {
            // TODO: perhaps if value doesn't exist, we should emit explicit 'null'?
            self
        }
    }
}
