// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[wasm_bindgen(typescript_custom_section)]
const TS_DEFS: &'static str = r#"
export interface BinaryMessage {
    kind: number,
    payload: Uint8Array;
    headers: string,
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
    pub headers: String,
}

#[derive(Serialize, Deserialize)]
pub struct StringMessage {
    pub kind: u8,
    pub payload: String,
}

/// Create a new binary message with a user-specified `kind`.
#[wasm_bindgen]
pub fn create_binary_message(kind: u8, payload: Vec<u8>) -> Vec<u8> {
    create_binary_message_with_headers(kind, payload, "".to_string())
}

/// Create a new message with a UTF-8 encoded string `payload` and a user-specified `kind`.
#[wasm_bindgen]
pub fn create_binary_message_from_string(kind: u8, payload: String) -> Vec<u8> {
    create_binary_message_with_headers(kind, payload.as_bytes().to_vec(), "".to_string())
}

/// Create a new binary message with a user-specified `kind`, and `headers` as a string.
#[wasm_bindgen]
pub fn create_binary_message_with_headers(kind: u8, payload: Vec<u8>, headers: String) -> Vec<u8> {
    let headers = headers.as_bytes().to_vec();
    let size = (headers.len() as u64).to_be_bytes().to_vec();
    vec![vec![kind], size, headers, payload].concat()
}

/// Parse the `kind` and byte array `payload` from a byte array
#[wasm_bindgen]
pub async fn parse_binary_message(message: Vec<u8>) -> Result<IBinaryMessage, JsError> {
    if message.len() < 2 {
        return Err(JsError::new(
            "Could not parse message, as less than 2 bytes long",
        ));
    }

    let (kind, _headers, payload) = parse_binary_payload(&message);

    Ok(serde_wasm_bindgen::to_value(&BinaryMessage {
        kind,
        payload: payload.to_vec(),
        headers: "".to_string(),
    })
    .unwrap()
    .unchecked_into::<IBinaryMessage>())
}

/// Parse the `kind` and byte array `payload` from a byte array with headers
#[wasm_bindgen]
pub async fn parse_binary_message_with_headers(
    message: Vec<u8>,
) -> Result<IBinaryMessage, JsError> {
    if message.len() < 2 {
        return Err(JsError::new(
            "Could not parse message, as less than 2 bytes long",
        ));
    }

    let (kind, headers, payload) = parse_binary_payload(&message);

    Ok(serde_wasm_bindgen::to_value(&BinaryMessage {
        kind,
        payload: payload.to_vec(),
        headers,
    })
    .unwrap()
    .unchecked_into::<IBinaryMessage>())
}

/// Parse the `kind` and UTF-8 string `payload` from a byte array with headers
#[wasm_bindgen]
pub async fn parse_string_message_with_headers(
    message: Vec<u8>,
) -> Result<IStringMessage, JsError> {
    if message.len() < 2 {
        return Err(JsError::new(
            "Could not parse message, as less than 2 bytes long",
        ));
    }

    let (kind, _headers, payload) = parse_binary_payload(&message);
    let payload = String::from_utf8_lossy(payload).into_owned();

    Ok(
        serde_wasm_bindgen::to_value(&StringMessage { kind, payload })
            .unwrap()
            .unchecked_into::<IStringMessage>(),
    )
}
pub(crate) fn parse_binary_payload(message: &[u8]) -> (u8, String, &[u8]) {
    // 1st byte is the kind
    let kind = message[0];

    // then the size as u64 big endian
    let mut size = [0u8; 8];
    size.clone_from_slice(&message[1..9]);
    let size = u64::from_be_bytes(size) as usize;

    // then the headers
    let headers = String::from_utf8_lossy(&message[9..9 + size]).into_owned();

    // finally the payload
    let payload = &message[9 + size..];

    (kind, headers, payload)
}

/// Parse the `kind` and UTF-8 string `payload` from a byte array
#[wasm_bindgen]
pub async fn parse_string_message(message: Vec<u8>) -> Result<IStringMessage, JsError> {
    if message.len() < 2 {
        return Err(JsError::new(
            "Could not parse message, as less than 2 bytes long",
        ));
    }

    let kind = message[0];
    let payload = String::from_utf8_lossy(&message[1..]).into_owned();

    Ok(
        serde_wasm_bindgen::to_value(&StringMessage { kind, payload })
            .unwrap()
            .unchecked_into::<IStringMessage>(),
    )
}

#[cfg(test)]
mod tests {
    use super::{create_binary_message_with_headers, parse_binary_payload};
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    fn test_binary_with_headers() {
        let message_as_bytes = create_binary_message_with_headers(
            42u8,
            vec![0u8, 1u8, 2u8],
            "test headers".to_string(),
        );

        // calculate header size
        let headers = "test headers".as_bytes().to_vec();
        let size = headers.len();

        // the expected size
        let expected_size = 12;
        assert_eq!(size, expected_size);

        assert_eq!(message_as_bytes[0], 42u8);
        assert_eq!(message_as_bytes[1..9], 12u64.to_be_bytes());
        assert_eq!(
            message_as_bytes[9 + expected_size..9 + expected_size + 3],
            vec![0u8, 1u8, 2u8]
        );

        let res = parse_binary_payload(&message_as_bytes);

        assert_eq!(res.0, 42u8);
        assert_eq!(res.1, "test headers".to_string());
        assert_eq!(res.2, vec![0u8, 1u8, 2u8]);
    }

    #[wasm_bindgen_test]
    fn test_binary_with_empty_headers() {
        let message_as_bytes =
            create_binary_message_with_headers(42u8, vec![0u8, 1u8, 2u8], "".to_string());

        let expected_size = 0;

        assert_eq!(message_as_bytes[0], 42u8);
        assert_eq!(message_as_bytes[1..9], 0u64.to_be_bytes());
        assert_eq!(
            message_as_bytes[9 + expected_size..9 + expected_size + 3],
            vec![0u8, 1u8, 2u8]
        );

        let res = parse_binary_payload(&message_as_bytes);

        assert_eq!(res.0, 42u8);
        assert_eq!(res.1, "".to_string());
        assert_eq!(res.2, vec![0u8, 1u8, 2u8]);
    }
}
