// Copyright 2020-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::request::RequestDeserializationError;
use crate::response::ResponseDeserializationError;
use crate::Socks5Request;
use service_providers_common::interface::{self, ServiceProviderMessagingError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MessageError {
    #[error("failed to deserialize received request: {source}")]
    Request {
        #[from]
        source: RequestDeserializationError,
    },

    #[error("failed to deserialize received response: {source}")]
    Response {
        #[from]
        source: ResponseDeserializationError,
    },

    #[error("no data")]
    NoData,

    #[error("unknown message type received")]
    UnknownMessageType,

    // TODO:
    // TODO:
    // TODO:
    // TODO:
    #[error(transparent)]
    Placeholder(#[from] ServiceProviderMessagingError),
}

// those are placeholders until I figure out proper serialization to preserve backwards compatibility
// pub type PlaceholderRequest = interface::Request<NewSocks5Request>;
// pub type PlaceholderResponse = interface::Response<NewSocks5Request>;
pub type PlaceholderRequest = interface::Request<Socks5Request>;
pub type PlaceholderResponse = interface::Response<Socks5Request>;

// #[derive(Debug)]
// pub enum NewSocks5Response {
//     // TODO: flatten the inner Response
//     NetworkData(NetworkData),
//
//     // TODO: flatten the inner Response
//     ConnectionError(ConnectionError),
// }
//
// impl ServiceProviderResponse for NewSocks5Response {}
//
// impl Serializable for NewSocks5Response {
//     type Error = MessageError;
//
//     fn into_bytes(self) -> Vec<u8> {
//         // for now use the existing one
//         match self {
//             NewSocks5Response::NetworkData(m) => Message::Response(m).into_bytes(),
//             NewSocks5Response::ConnectionError(m) => {
//                 Message::NetworkRequesterResponse(m).into_bytes()
//             }
//         }
//     }
//
//     fn try_from_bytes(b: &[u8]) -> Result<Self, Self::Error> {
//         match Message::try_from_bytes(b)? {
//             Message::Request(_) => todo!(),
//             Message::Response(m) => Ok(Self::NetworkData(m)),
//             Message::NetworkRequesterResponse(m) => Ok(Self::ConnectionError(m)),
//         }
//     }
// }
//
// impl NewSocks5Response {
//     pub fn new_network_data(content: NetworkData) -> Self {
//         NewSocks5Response::NetworkData(content)
//     }
//
//     pub fn new_connection_error(content: ConnectionError) -> Self {
//         NewSocks5Response::ConnectionError(content)
//     }
// }
//
// #[derive(Debug)]
// pub enum Message {
//     Request(Socks5RequestContent),
//     Response(NetworkData),
//     NetworkRequesterResponse(ConnectionError),
// }
//
// impl Message {
//     const REQUEST_FLAG: u8 = 0;
//     const RESPONSE_FLAG: u8 = 1;
//     const NR_RESPONSE_FLAG: u8 = 2;
//
//     pub fn conn_id(&self) -> u64 {
//         match self {
//             Message::Request(req) => match req {
//                 Socks5RequestContent::Connect(c) => c.conn_id,
//                 Socks5RequestContent::Send(s) => s.conn_id,
//             },
//             Message::Response(resp) => resp.connection_id,
//             Message::NetworkRequesterResponse(resp) => resp.connection_id,
//         }
//     }
//
//     pub fn size(&self) -> usize {
//         match self {
//             Message::Request(req) => match req {
//                 Socks5RequestContent::Connect(_) => 0,
//                 Socks5RequestContent::Send(s) => s.data.len(),
//             },
//             Message::Response(resp) => resp.data.len(),
//             Message::NetworkRequesterResponse(_) => 0,
//         }
//     }
//
//     pub fn try_from_bytes(b: &[u8]) -> Result<Message, MessageError> {
//         if b.is_empty() {
//             return Err(MessageError::NoData);
//         }
//
//         if b[0] == Self::REQUEST_FLAG {
//             Socks5RequestContent::try_from_bytes(&b[1..])
//                 .map(Message::Request)
//                 .map_err(Into::into)
//         } else if b[0] == Self::RESPONSE_FLAG {
//             NetworkData::try_from_bytes(&b[1..])
//                 .map(Message::Response)
//                 .map_err(Into::into)
//         } else if b[0] == Self::NR_RESPONSE_FLAG {
//             ConnectionError::try_from_bytes(&b[1..])
//                 .map(Message::NetworkRequesterResponse)
//                 .map_err(MessageError::NetworkRequesterResponseError)
//         } else {
//             Err(MessageError::UnknownMessageType)
//         }
//     }
//
//     pub fn into_bytes(self) -> Vec<u8> {
//         match self {
//             Self::Request(r) => std::iter::once(Self::REQUEST_FLAG)
//                 .chain(r.into_bytes().into_iter())
//                 .collect(),
//             Self::Response(r) => std::iter::once(Self::RESPONSE_FLAG)
//                 .chain(r.into_bytes().into_iter())
//                 .collect(),
//             Self::NetworkRequesterResponse(r) => std::iter::once(Self::NR_RESPONSE_FLAG)
//                 .chain(r.into_bytes().into_iter())
//                 .collect(),
//         }
//     }
// }
