// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bytecodec::bytes::BytesEncoder;
use bytecodec::io::IoEncodeExt;
use bytecodec::Encode;
use httpcodec::{BodyEncoder, Request, RequestEncoder};

pub mod error;
pub mod socks;

pub fn encode_http_request_as_string(
    request: Request<Vec<u8>>,
) -> Result<String, error::MixHttpRequestError> {
    // Encode HTTP request as bytes
    let mut encoder = RequestEncoder::new(BodyEncoder::new(BytesEncoder::new()));
    encoder.start_encoding(request)?;
    let mut buf = Vec::new();
    encoder.encode_all(&mut buf)?;

    Ok(String::from_utf8_lossy(&buf).to_string())
}

#[cfg(test)]
mod http_requests_tests {
    use super::*;
    use httpcodec::{HeaderField, HttpVersion, Method, RequestTarget};

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

    #[test]
    fn http_request_ok() {
        // Encode HTTP request as bytes
        let request = create_http_get_request();
        let mut encoder = RequestEncoder::new(BodyEncoder::new(BytesEncoder::new()));
        encoder.start_encoding(request).unwrap();
        let mut buf = Vec::new();
        encoder.encode_all(&mut buf).unwrap();

        let body_as_string = String::from_utf8(buf).unwrap();

        // replace newlines with \r\n
        let expected = r"GET /.wellknown/wallet/validators.json HTTP/1.1
Host: nymtech.net
Content-Length: 0

"
        .replace('\n', "\r\n");

        assert_eq!(expected, body_as_string);
    }
}
