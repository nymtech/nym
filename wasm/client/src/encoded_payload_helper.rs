// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use anyhow::bail;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[wasm_bindgen(typescript_custom_section)]
const TS_DEFS: &'static str = r#"
export interface EncodedPayload {
    mimeType: string,
    payload: Uint8Array;
    headers: string,
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "EncodedPayload")]
    pub type IEncodedPayload;
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncodedPayload {
    pub mime_type: String,
    pub payload: Vec<u8>,
    pub headers: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncodedPayloadMetadata {
    pub mime_type: String,
    pub headers: Option<String>,
}

/// Encode a payload
#[wasm_bindgen]
pub fn encode_payload(mime_type: String, payload: Vec<u8>) -> Result<Vec<u8>, JsValue> {
    encode_payload_with_headers(mime_type, payload, None)
}

/// Create a new binary message with a user-specified `kind`, and `headers` as a string.
#[wasm_bindgen]
pub fn encode_payload_with_headers(
    mime_type: String,
    payload: Vec<u8>,
    headers: Option<String>,
) -> Result<Vec<u8>, JsValue> {
    match serde_json::to_string(&EncodedPayloadMetadata { mime_type, headers }) {
        Ok(metadata) => {
            let metadata = metadata.as_bytes().to_vec();
            let size = (metadata.len() as u64).to_be_bytes().to_vec();
            Ok([size, metadata, payload].concat())
        }
        Err(e) => Err(JsValue::from(JsError::new(
            format!("Could not encode message: {}", e).as_str(),
        ))),
    }
}

/// Parse the `kind` and byte array `payload` from a byte array
#[wasm_bindgen]
pub fn decode_payload(message: Vec<u8>) -> Result<IEncodedPayload, JsValue> {
    if message.len() < 8 {
        return Err(JsValue::from(JsError::new(
            "Could not parse message, as less than 8 bytes long",
        )));
    }

    match parse_payload(&message) {
        Ok((metadata, payload)) => Ok(serde_wasm_bindgen::to_value(&EncodedPayload {
            mime_type: metadata.mime_type,
            payload: payload.to_vec(),
            headers: metadata.headers,
        })
        .unwrap()
        .unchecked_into::<IEncodedPayload>()),
        Err(e) => Err(JsValue::from(JsError::new(
            format!("Could not parse message: {}", e).as_str(),
        ))),
    }
}

pub(crate) fn parse_payload(message: &[u8]) -> anyhow::Result<(EncodedPayloadMetadata, &[u8])> {
    // 1st 8 bytes are the size (as u64 big endian)
    let mut size = [0u8; 8];
    if message.len() < 8 {
        bail!("Message is too short to contain size information")
    }
    size.clone_from_slice(&message[0..8]);
    let metadata_size = u64::from_be_bytes(size) as usize;

    if metadata_size + 8 != message.len() {
        return Err(anyhow::anyhow!(format!(
            "Metadata size: {}, exceeds message with length of: {}",
            metadata_size,
            message.len()
        ),));
    }

    //then the metadata
    let metadata: EncodedPayloadMetadata = serde_json::from_slice(&message[8..8 + metadata_size])?;

    //finally the payload
    let payload = &message[8 + metadata_size..];

    Ok((metadata, payload))
}

/// Try parse a UTF-8 string from an array of bytes
#[wasm_bindgen]
pub fn parse_utf8_string(payload: Vec<u8>) -> String {
    String::from_utf8_lossy(&payload).into_owned()
}

/// Converts a UTF-8 string into an array of bytes
///
/// This method is provided as a replacement for the mess of `atob`
/// (https://developer.mozilla.org/en-US/docs/Web/API/atob) helpers provided by browsers and NodeJS.
///
/// Feel free to use `atob` if you know you won't have problems with polyfills or encoding issues.
#[wasm_bindgen]
pub fn utf8_string_to_byte_array(message: String) -> Vec<u8> {
    message.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    #[wasm_bindgen_test]
    async fn test_encode_payload_with_headers() {
        let message_as_bytes = encode_payload_with_headers(
            "text/plain".to_string(),
            vec![0u8, 1u8, 2u8],
            Some("test headers".to_string()),
        )
        .unwrap();

        // the expected message size
        let size = message_as_bytes.len();
        let expected_size = 61;
        assert_eq!(size, expected_size);

        let expected_header_size = 50usize;
        assert_eq!(
            message_as_bytes[0..8],
            (expected_header_size as u64).to_be_bytes()
        );

        assert_eq!(
            message_as_bytes[8 + expected_header_size..8 + expected_header_size + 3],
            vec![0u8, 1u8, 2u8]
        );

        let res = parse_payload(&message_as_bytes).unwrap();

        assert_eq!(res.0.mime_type, "text/plain");
        assert_eq!(res.0.headers.unwrap(), "test headers".to_string());
        assert_eq!(res.1, vec![0u8, 1u8, 2u8]);
    }

    #[wasm_bindgen_test]
    async fn test_encode_payload_with_empty_headers() {
        let message_as_bytes =
            encode_payload_with_headers("text/plain".to_string(), vec![0u8, 1u8, 2u8], None)
                .unwrap();

        // the expected message size
        let size = message_as_bytes.len();
        let expected_size = 51;
        assert_eq!(size, expected_size);

        let expected_header_size = 40usize;
        assert_eq!(
            message_as_bytes[0..8],
            (expected_header_size as u64).to_be_bytes()
        );

        assert_eq!(
            message_as_bytes[8 + expected_header_size..8 + expected_header_size + 3],
            vec![0u8, 1u8, 2u8]
        );
        let res = parse_payload(&message_as_bytes).unwrap();

        assert_eq!(res.0.mime_type, "text/plain");
        assert_eq!(res.0.headers, None);
        assert_eq!(res.1, vec![0u8, 1u8, 2u8]);
    }

    #[wasm_bindgen_test]
    async fn test_encode_payload_with_empty_headers_and_empty_mime_type() {
        let message_as_bytes =
            encode_payload_with_headers("".to_string(), vec![0u8, 1u8, 2u8], None).unwrap();

        // the expected message size
        let size = message_as_bytes.len();
        let expected_size = 41;
        assert_eq!(size, expected_size);

        let expected_header_size = 30usize;
        assert_eq!(
            message_as_bytes[0..8],
            (expected_header_size as u64).to_be_bytes()
        );

        assert_eq!(
            message_as_bytes[8 + expected_header_size..8 + expected_header_size + 3],
            vec![0u8, 1u8, 2u8]
        );
        let res = parse_payload(&message_as_bytes).unwrap();

        assert_eq!(res.0.mime_type, "");
        assert_eq!(res.0.headers, None);
        assert_eq!(res.1, vec![0u8, 1u8, 2u8]);
    }

    #[wasm_bindgen_test]
    async fn test_encode_payload_with_all_empty() {
        let empty: Vec<u8> = vec![];
        let message_as_bytes =
            encode_payload_with_headers("".to_string(), empty.clone(), None).unwrap();

        // the expected message size
        let size = message_as_bytes.len();
        let expected_size = 38;
        assert_eq!(size, expected_size);

        let expected_header_size = 30usize;
        assert_eq!(
            message_as_bytes[0..8],
            (expected_header_size as u64).to_be_bytes()
        );

        assert_eq!(
            message_as_bytes[8 + expected_header_size..8 + expected_header_size],
            empty
        );
        let res = parse_payload(&message_as_bytes).unwrap();

        assert_eq!(res.0.mime_type, "");
        assert_eq!(res.0.headers, None);
        assert_eq!(res.1, empty);
    }

    #[wasm_bindgen_test]
    async fn test_parse_payload_too_short() {
        let message = vec![0u8; 7];
        let result = parse_payload(&message);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(
            error.to_string(),
            "Message is too short to contain size information"
        );
    }

    #[wasm_bindgen_test]
    async fn test_parse_payload_size_exceeds_length() {
        let mut message = vec![0u8; 8];
        message.extend(vec![1u8; 10]);
        message[0..8].copy_from_slice(&(20u64.to_be_bytes()));

        let result = parse_payload(&message);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(
            error.to_string(),
            "Metadata size: 20, exceeds message with length of: 18"
        );
    }

    #[wasm_bindgen_test]
    async fn test_parse_payload_valid() {
        let metadata = EncodedPayloadMetadata {
            mime_type: "text/plain".to_string(),
            headers: Some("test headers".to_string()),
        };
        let payload_data = vec![2u8, 3u8, 5u8];

        let serialized_metadata = serde_json::to_string(&metadata).unwrap();
        let metadata_length = serialized_metadata.len() as u64;

        let mut message = metadata_length.to_be_bytes().to_vec();
        message.extend_from_slice(serialized_metadata.as_bytes());
        message.extend_from_slice(&payload_data);

        let (parsed_metadata, parsed_payload) = parse_payload(&message).unwrap();

        assert_eq!(parsed_metadata.mime_type, metadata.mime_type);
        assert!(parsed_metadata.headers.is_some());
        assert_eq!(parsed_metadata.headers.unwrap(), "test headers");
        assert_eq!(parsed_payload, payload_data.as_slice());
    }
}
