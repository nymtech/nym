// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bytecodec::bytes::BytesEncoder;
use bytecodec::bytes::RemainingBytesDecoder;
use bytecodec::io::IoEncodeExt;
use bytecodec::{DecodeExt, Encode};
use httpcodec::{BodyDecoder, ResponseDecoder};
use httpcodec::{BodyEncoder, Request, RequestEncoder};

use crate::error;
use nym_ordered_buffer::OrderedMessage;
use nym_socks5_requests::{Socks5ProtocolVersion, Socks5Response, Socks5ResponseContent};

pub fn encode_http_request_as_socks_request(
    socks5_version: Socks5ProtocolVersion,
    conn_id: u64,
    request: Request<Vec<u8>>,
    ordered_message_index: Option<u64>,
    local_closed: bool,
) -> Result<nym_socks5_requests::request::Socks5Request, error::MixHttpRequestError> {
    // Encode HTTP request as bytes
    let mut encoder = RequestEncoder::new(BodyEncoder::new(BytesEncoder::new()));
    encoder.start_encoding(request)?;
    let mut buf = Vec::new();
    encoder.encode_all(&mut buf)?;

    // Create an ordered message
    let ordered_msg = OrderedMessage {
        data: buf,
        index: ordered_message_index.unwrap_or(0u64),
    };

    // Wrap is SOCKS send request
    Ok(nym_socks5_requests::request::Socks5Request::new_send(
        socks5_version,
        conn_id,
        ordered_msg.into_bytes(),
        local_closed,
    ))
}

#[derive(Debug)]
pub struct MixHttpResponse {
    pub connection_id: u64,
    pub is_closed: bool,
    pub http_response: httpcodec::Response<Vec<u8>>,
    pub ordered_message_index: u64,
}

pub fn decode_socks_response_as_http_response(
    socks5_response: Socks5Response,
) -> Result<MixHttpResponse, error::MixHttpRequestError> {
    if let Socks5ResponseContent::NetworkData(data) = socks5_response.content {
        // data.data is really an OrderedMessage
        let response_ordered_message = OrderedMessage::try_from_bytes(data.data)?;

        if !response_ordered_message.data.is_empty() {
            let mut decoder = ResponseDecoder::<BodyDecoder<RemainingBytesDecoder>>::default();
            let http_response =
                decoder.decode_from_bytes(response_ordered_message.data.as_ref())?;

            return Ok(MixHttpResponse {
                connection_id: data.connection_id,
                is_closed: data.is_closed,
                http_response,
                ordered_message_index: response_ordered_message.index,
            });
        }
    }
    Err(error::MixHttpRequestError::InvalidSocks5Response)
}

#[cfg(test)]
mod http_requests_tests {
    use super::*;
    use httpcodec::{HeaderField, HttpVersion, Method, RequestTarget};
    use nym_service_providers_common::interface::Serializable;
    use nym_socks5_requests::NetworkData;

    fn create_socks5_request_buffer() -> Vec<u8> {
        let request = create_http_get_request();
        let socks5_request = encode_http_request_as_socks_request(
            Socks5ProtocolVersion::Versioned(5),
            99u64,
            request,
            Some(42u64),
            true,
        )
        .unwrap();
        socks5_request.into_bytes()
    }

    #[test]
    fn request_http_request_content_ok() {
        let buffer = create_socks5_request_buffer();

        // HTTP request string content is as expected
        assert_eq!(
            [71u8, 69u8, 84u8, 32u8, 47u8, 46u8, 119u8, 101u8],
            buffer[19..27]
        );
    }

    /// This test will fail if the framing of the request buffer changes, e.g. when OrderedMessage
    /// changes to have the `index` value as a field, instead of packed with the `data`
    #[test]
    fn request_size_as_expected_ok() {
        let buffer = create_socks5_request_buffer();
        // println!("{:?}", buffer) // uncomment and run `cargo test -- --nocapture` to view

        assert_eq!(108, buffer.len()); // version set to SOCKS5
    }

    #[test]
    fn request_socks5_headers_ok() {
        let buffer = create_socks5_request_buffer();

        assert_eq!(5u8, buffer[0]); // version set to SOCKS5
        assert_eq!(1u8, buffer[1]); // type is SEND
        assert_eq!(99u8, buffer[9]); // ConnectionId is correct
        assert_eq!(1u8, buffer[10]); // local_closed is true
    }

    #[test]
    fn request_ordered_message_ok() {
        let buffer = create_socks5_request_buffer();

        // OrderedMessage index is correct
        assert_eq!(42u8, buffer[18]);
    }

    fn create_socks_response() -> Socks5Response {
        // HTTP response is just a string
        let http_response_string = "HTTP/1.1 200 OK\r\nServer: foo/0.0.1\r\n\r\n";

        // wrap in an ordered message
        let data = OrderedMessage {
            data: http_response_string.as_bytes().to_vec(),
            index: 42u64,
        }
        .into_bytes();

        // wrap in `NetworkData`, then Socks5Response
        Socks5Response::new(
            Socks5ProtocolVersion::Versioned(5),
            Socks5ResponseContent::NetworkData(NetworkData::new(99u64, data, false)),
        )
    }

    /// This test will fail is anything in the framing of the socks5_response byte
    /// representation changes
    #[test]
    fn response_byte_size_is_as_expected() {
        let socks5_response = create_socks_response();
        let buf = socks5_response.into_bytes();

        assert_eq!(57, buf.len());
    }

    #[test]
    fn response_parses() {
        let socks5_response = create_socks_response();
        let response = decode_socks_response_as_http_response(socks5_response).unwrap();

        assert_eq!(42u64, response.ordered_message_index); // OrderedMessage index as expected
        assert_eq!(HttpVersion::V1_1, response.http_response.http_version());
        assert_eq!(200u16, response.http_response.status_code().as_u16());
        assert_eq!(
            "foo/0.0.1",
            response.http_response.header().get_field("Server").unwrap()
        );
    }
}
