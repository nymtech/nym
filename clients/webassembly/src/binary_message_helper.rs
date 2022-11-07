// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use serde::{Serialize, Deserialize};

#[wasm_bindgen(typescript_custom_section)]
const TS_DEFS: &'static str = r#"
export interface BinaryMessage {
    kind: number,
    payload: Uint8Array;
}

export interface StringMessage {
    kind: number,
    payload: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "BinaryMessage")]
    pub type IBinaryMessage;
    #[wasm_bindgen(typescript_type = "StringMessage")]
    pub type IStringMessage;
}

#[derive(Serialize, Deserialize)]
pub struct BinaryMessage {
    pub kind: u8,
    pub payload: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct StringMessage {
    pub kind: u8,
    pub payload: String,
}

/// Create a new binary message with a user-specified `kind`.
#[wasm_bindgen]
pub fn create_binary_message(kind: u8, payload: Vec<u8>) -> Vec<u8> {
    vec![vec![kind], payload].concat()
}

/// Create a new message with a UTF-8 encoded string `payload` and a user-specified `kind`.
#[wasm_bindgen]
pub fn create_binary_message_from_string(kind: u8, payload: String) -> Vec<u8> {
    vec!(vec![kind], payload.into_bytes()).concat()
}

/// Parse the `kind` and byte array `payload` from a byte array
#[wasm_bindgen]
pub async fn parse_binary_message(message: Vec<u8>) -> Result<IBinaryMessage, JsError> {
    if message.len() < 2 {
        return Err(JsError::new("Could not parse message, as less than 2 bytes long"));
    }

    let kind = message[0];
    let payload = &message[1..];

    Ok(serde_wasm_bindgen::to_value(&BinaryMessage {
        kind,
        payload: payload.to_vec(),
    }).unwrap().unchecked_into::<IBinaryMessage>())
}

/// Parse the `kind` and UTF-8 string `payload` from a byte array
#[wasm_bindgen]
pub async fn parse_string_message(message: Vec<u8>) -> Result<IStringMessage, JsError> {
    if message.len() < 2 {
        return Err(JsError::new("Could not parse message, as less than 2 bytes long"));
    }

    let kind = message[0];
    let payload = String::from_utf8_lossy(&message[1..]).into_owned();

    Ok(serde_wasm_bindgen::to_value(&StringMessage {
        kind,
        payload,
    }).unwrap().unchecked_into::<IStringMessage>())
}
