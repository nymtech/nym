// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::crypto::{generate_key, KeyUsage};
use wasm_bindgen::{JsCast, JsValue};

pub use web_sys::CryptoKey;

pub async fn generate_symmetric_key(
    algorithm: &str,
    extractable: bool,
    key_usages: &[KeyUsage],
) -> Result<CryptoKey, JsValue> {
    let key = generate_key(algorithm, extractable, key_usages).await?;
    key.dyn_into::<CryptoKey>()
}
