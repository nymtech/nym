// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct PlaceholderStruct {
    pub(crate) typ: String,
    pub(crate) placeholder: String,
}

impl PlaceholderStruct {
    pub(crate) fn new<T>(_: T) -> Self {
        PlaceholderStruct {
            typ: std::any::type_name::<T>().to_string(),
            placeholder: "PLACEHOLDER CONTENT - SOMETHING IS MISSING serde DERIVES".to_string(),
        }
    }
}
