// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bytecodec::bytes::BytesEncoder;
use bytecodec::bytes::RemainingBytesDecoder;
use bytecodec::io::IoEncodeExt;
use bytecodec::{DecodeExt, Encode};
use httpcodec::{BodyDecoder, ResponseDecoder};
use httpcodec::{
    BodyEncoder, HeaderField, HttpVersion, Method, Request, RequestEncoder, RequestTarget,
};
use nym_ordered_buffer::OrderedMessage;

use crate::mix_fetch::error::MixFetchError;
use nym_service_providers_common::interface::{ProviderInterfaceVersion, Serializable};
use nym_socks5_requests::{
    NetworkData, Socks5ProtocolVersion, Socks5ProviderRequest, Socks5Response,
    Socks5ResponseContent,
};

pub mod error;

pub fn encode_fetch_request_as_socks5_request(
    socks5_version: Socks5ProtocolVersion,
    conn_id: u64,
    request: Request<Vec<u8>>,
) -> Result<nym_socks5_requests::request::Socks5Request, MixFetchError> {
    // Encode HTTP request as bytes
    let mut encoder = RequestEncoder::new(BodyEncoder::new(BytesEncoder::new()));
    encoder.start_encoding(request).unwrap();
    let mut buf = Vec::new();
    encoder.encode_all(&mut buf).unwrap();

    // Create an ordered message
    let ordered_msg = OrderedMessage {
        data: buf,
        index: 0,
    };

    // Wrap is SOCKS5 send request
    Ok(nym_socks5_requests::request::Socks5Request::new_send(
        socks5_version,
        conn_id,
        ordered_msg.into_bytes(),
        true,
    ))
}

pub fn decode_socks5_response_as_fetch_response(
    socks5_response: Socks5Response,
) -> Result<httpcodec::Response<Vec<u8>>, MixFetchError> {
    if let Socks5ResponseContent::NetworkData(data) = socks5_response.content {
        // data.data is really an OrderedMessage
        let response_ordered_message = OrderedMessage::try_from_bytes(data.data).unwrap();

        if !response_ordered_message.data.is_empty() {
            println!("️✅  resp: {:?}", response_ordered_message.data);

            let mut decoder = ResponseDecoder::<BodyDecoder<RemainingBytesDecoder>>::default();
            let response = decoder
                .decode_from_bytes(response_ordered_message.data.as_ref())
                .unwrap();

            return Ok(response);
        }
    }
    Err(MixFetchError::InvalidSocks5Response)
}
