// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use js_sys::Array;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Crypto, Window, WorkerGlobalScope};

pub mod asymmetric;
pub mod symmetric;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum KeyUsage {
    Encrypt,
    Decrypt,
    Sign,
    Verify,
    DeriveKey,
    DeriveBits,
    WrapKey,
    UnwrapKey,
}

pub struct InvalidKeyUsage;

impl FromStr for KeyUsage {
    type Err = InvalidKeyUsage;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "encrypt" => Ok(KeyUsage::Encrypt),
            "decrypt" => Ok(KeyUsage::Decrypt),
            "sign" => Ok(KeyUsage::Sign),
            "verify" => Ok(KeyUsage::Verify),
            "deriveKey" => Ok(KeyUsage::DeriveKey),
            "deriveBits" => Ok(KeyUsage::DeriveBits),
            "wrapKey" => Ok(KeyUsage::WrapKey),
            "unwrapKey" => Ok(KeyUsage::UnwrapKey),
            _ => Err(InvalidKeyUsage),
        }
    }
}

impl From<KeyUsage> for JsValue {
    fn from(value: KeyUsage) -> Self {
        JsValue::from_str(&value.to_string())
    }
}

impl Display for KeyUsage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyUsage::Encrypt => write!(f, "encrypt"),
            KeyUsage::Decrypt => write!(f, "decrypt"),
            KeyUsage::Sign => write!(f, "sign"),
            KeyUsage::Verify => write!(f, "verify"),
            KeyUsage::DeriveKey => write!(f, "deriveKey"),
            KeyUsage::DeriveBits => write!(f, "deriveBits"),
            KeyUsage::WrapKey => write!(f, "wrapKey"),
            KeyUsage::UnwrapKey => write!(f, "unwrapKey"),
        }
    }
}

#[derive(Debug)]
pub enum GlobalScope {
    Window(Window),
    Worker(WorkerGlobalScope),
}

impl GlobalScope {
    pub fn crypto(&self) -> Result<Crypto, JsValue> {
        match self {
            GlobalScope::Window(window) => window.crypto(),
            GlobalScope::Worker(worker) => worker.crypto(),
        }
    }
}

pub fn get_global_scope() -> Option<GlobalScope> {
    let global = js_sys::global();

    match global.dyn_into::<Window>() {
        Ok(window) => Some(GlobalScope::Window(window)),
        Err(maybe_worker) => maybe_worker
            .dyn_into::<WorkerGlobalScope>()
            .ok()
            .map(GlobalScope::Worker),
    }
}

pub async fn generate_key(
    algorithm: &str,
    extractable: bool,
    key_usages: &[KeyUsage],
) -> Result<JsValue, JsValue> {
    let key_usages = key_usages
        .iter()
        .map(|&usage| JsValue::from(usage))
        .collect::<Array>();

    JsFuture::from(
        get_global_scope()
            .expect("no global scope available!")
            .crypto()?
            .subtle()
            .generate_key_with_str(algorithm, extractable, &key_usages)?,
    )
    .await
}
