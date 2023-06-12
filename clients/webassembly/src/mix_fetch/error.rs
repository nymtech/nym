// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;
use wasm_bindgen::JsValue;
use wasm_utils::simple_js_error;

#[derive(Debug, Error)]
pub enum MixFetchError {
    //
}

impl From<MixFetchError> for JsValue {
    fn from(value: MixFetchError) -> Self {
        simple_js_error(value.to_string())
    }
}
