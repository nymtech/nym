// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::crypto::{generate_key, KeyUsage};
use wasm_bindgen::{JsCast, JsValue};

pub use web_sys::CryptoKeyPair;

pub async fn generate_asymmetric_keypair(
    algorithm: &str,
    extractable: bool,
    key_usages: &[KeyUsage],
) -> Result<CryptoKeyPair, JsValue> {
    let key = generate_key(algorithm, extractable, key_usages).await?;
    key.dyn_into::<CryptoKeyPair>()
}
