// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error;
use bytecodec::bytes::BytesEncoder;
use bytecodec::bytes::RemainingBytesDecoder;
use bytecodec::io::IoEncodeExt;
use bytecodec::{DecodeExt, Encode};
use httpcodec::{BodyDecoder, ResponseDecoder};
use httpcodec::{BodyEncoder, Request, RequestEncoder};
use nym_service_providers_common::interface::ProviderInterfaceVersion;
use nym_socks5_requests::{SocketData, Socks5ProtocolVersion, Socks5ProviderRequest};

pub fn encode_http_request_as_socks_send_request(
    provider_interface: ProviderInterfaceVersion,
    socks5_protocol: Socks5ProtocolVersion,
    conn_id: u64,
    request: Request<Vec<u8>>,
    seq: Option<u64>,
    local_closed: bool,
) -> Result<nym_socks5_requests::Socks5ProviderRequest, error::MixHttpRequestError> {
    // Encode HTTP request as bytes
    let mut encoder = RequestEncoder::new(BodyEncoder::new(BytesEncoder::new()));
    encoder.start_encoding(request)?;
    let mut buf = Vec::new();
    encoder.encode_all(&mut buf)?;

    // Wrap it as SOCKS send request
    let request_content = nym_socks5_requests::request::Socks5Request::new_send(
        socks5_protocol,
        SocketData::new(seq.unwrap_or_default(), conn_id, local_closed, buf),
    );

    // and wrap it in provider request
    Ok(Socks5ProviderRequest::new_provider_data(
        provider_interface,
        request_content,
    ))
}

#[derive(Debug)]
pub struct MixHttpResponse {
    // pub connection_id: u64,
    // #[deprecated]
    // pub is_closed: bool,
    pub http_response: httpcodec::Response<Vec<u8>>,
    // #[deprecated]
    // pub seq: u64,
}

impl MixHttpResponse {
    pub fn try_from_bytes(b: &[u8]) -> Result<MixHttpResponse, error::MixHttpRequestError> {
        if b.is_empty() {
            Err(error::MixHttpRequestError::EmptySocks5Response)
        } else {
            let mut decoder = ResponseDecoder::<BodyDecoder<RemainingBytesDecoder>>::default();
            let http_response = decoder.decode_from_bytes(b)?;

            Ok(MixHttpResponse { http_response })
        }
    }
}

// impl TryFrom<Socks5Response> for MixHttpResponse {
//     type Error = error::MixHttpRequestError;
//
//     fn try_from(value: Socks5Response) -> Result<Self, Self::Error> {
//         if let Socks5ResponseContent::NetworkData { content } = value.content {
//             content.try_into()
//         } else {
//             Err(error::MixHttpRequestError::InvalidSocks5Response)
//         }
//     }
// }
//
// impl TryFrom<SocketData> for MixHttpResponse {
//     type Error = error::MixHttpRequestError;
//
//     fn try_from(value: SocketData) -> Result<Self, Self::Error> {
//         if value.data.is_empty() {
//             Err(error::MixHttpRequestError::EmptySocks5Response)
//         } else {
//             let mut decoder = ResponseDecoder::<BodyDecoder<RemainingBytesDecoder>>::default();
//             let http_response = decoder.decode_from_bytes(value.data.as_ref())?;
//
//             Ok(MixHttpResponse {
//                 connection_id: value.header.connection_id,
//                 is_closed: value.header.local_socket_closed,
//                 http_response,
//                 seq: value.header.seq,
//             })
//         }
//     }
// }

// pub fn decode_socks_response_as_http_response(
//     socks5_response: Socks5Response,
// ) -> Result<MixHttpResponse, error::MixHttpRequestError> {
//     socks5_response.try_into()
// }

#[cfg(test)]
mod http_requests_tests {
    use super::*;
    use httpcodec::{HeaderField, HttpVersion, Method, RequestTarget};
    use nym_service_providers_common::interface::Serializable;
    use nym_socks5_requests::Socks5Response;

    fn create_http_get_request() -> Request<Vec<u8>> {
        let mut request = Request::new(
            Method::new("GET").unwrap(),
            RequestTarget::new("/.wellknown/wallet/validators.json").unwrap(),
            HttpVersion::V1_1,
            b"".to_vec(),
        );
        let mut headers = request.header_mut();
        headers.add_field(HeaderField::new("Host", "nymtech.net").unwrap());

        request
    }

    fn create_socks5_request_buffer() -> Vec<u8> {
        let request = create_http_get_request();
        let socks5_request = encode_http_request_as_socks_send_request(
            ProviderInterfaceVersion::new_current(),
            Socks5ProtocolVersion::new_current(),
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

        let data = http_response_string.as_bytes().to_vec();

        // wrap in `NetworkData`, then Socks5Response
        Socks5Response::new(
            Socks5ProtocolVersion::new_current(),
            Socks5ResponseContent::NetworkData {
                content: SocketData::new(42, 99u64, false, data),
            },
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
        unimplemented!()
        // let socks5_response = create_socks_response();
        // let response = decode_socks_response_as_http_response(socks5_response).unwrap();
        //
        // assert_eq!(42u64, response.seq); // OrderedMessage index as expected
        // assert_eq!(HttpVersion::V1_1, response.http_response.http_version());
        // assert_eq!(200u16, response.http_response.status_code().as_u16());
        // assert_eq!(
        //     "foo/0.0.1",
        //     response.http_response.header().get_field("Server").unwrap()
        // );
    }
}
