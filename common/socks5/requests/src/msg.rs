// Copyright 2020-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::network_requester_response::{Error as NrError, NetworkRequesterResponse};
use crate::request::{Request, RequestError};
use crate::response::{Response, ResponseError};
use crate::{ConnectRequest, ConnectionId, RemoteAddress, SendRequest};
use nymsphinx_addressing::clients::Recipient;
use service_providers_common::interface::{
    self, Serializable, ServiceProviderMessagingError, ServiceProviderRequest,
    ServiceProviderResponse,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MessageError {
    #[error(transparent)]
    Request(RequestError),

    #[error("{0:?}")]
    Response(ResponseError),

    #[error(transparent)]
    NetworkRequesterResponseError(NrError),

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
pub type PlaceholderRequest = interface::Request<NewSocks5Request>;
pub type PlaceholderResponse = interface::Response<NewSocks5Request>;

// TODO: remove the wrapper
#[derive(Debug)]
pub struct NewSocks5Request(pub Request);

impl ServiceProviderRequest for NewSocks5Request {
    type Response = NewSocks5Response;
    type Error = MessageError;
}

impl Serializable for NewSocks5Request {
    type Error = MessageError;

    fn into_bytes(self) -> Vec<u8> {
        // for now use the existing one
        Message::Request(self.0).into_bytes()
    }

    fn try_from_bytes(b: &[u8]) -> Result<Self, Self::Error> {
        if let Message::Request(m) = Message::try_from_bytes(b)? {
            Ok(Self(m))
        } else {
            todo!()
        }
    }
}

impl NewSocks5Request {
    pub fn new_connect(
        conn_id: ConnectionId,
        remote_addr: RemoteAddress,
        return_address: Option<Recipient>,
    ) -> NewSocks5Request {
        NewSocks5Request(Request::Connect(Box::new(ConnectRequest {
            conn_id,
            remote_addr,
            return_address,
        })))
    }

    pub fn new_send(conn_id: ConnectionId, data: Vec<u8>, local_closed: bool) -> NewSocks5Request {
        NewSocks5Request(Request::Send(SendRequest {
            conn_id,
            data,
            local_closed,
        }))
    }
}

#[derive(Debug)]
pub enum NewSocks5Response {
    // TODO: flatten the inner Response
    NetworkData(Response),

    // TODO: flatten the inner Response
    ConnectionError(NetworkRequesterResponse),
}

impl ServiceProviderResponse for NewSocks5Response {}

impl Serializable for NewSocks5Response {
    type Error = MessageError;

    fn into_bytes(self) -> Vec<u8> {
        // for now use the existing one
        match self {
            NewSocks5Response::NetworkData(m) => Message::Response(m).into_bytes(),
            NewSocks5Response::ConnectionError(m) => {
                Message::NetworkRequesterResponse(m).into_bytes()
            }
        }
    }

    fn try_from_bytes(b: &[u8]) -> Result<Self, Self::Error> {
        match Message::try_from_bytes(b)? {
            Message::Request(_) => todo!(),
            Message::Response(m) => Ok(Self::NetworkData(m)),
            Message::NetworkRequesterResponse(m) => Ok(Self::ConnectionError(m)),
        }
    }
}

impl NewSocks5Response {
    pub fn new_network_data(content: Response) -> Self {
        NewSocks5Response::NetworkData(content)
    }

    pub fn new_connection_error(content: NetworkRequesterResponse) -> Self {
        NewSocks5Response::ConnectionError(content)
    }
}

#[derive(Debug)]
pub enum Message {
    Request(Request),
    Response(Response),
    NetworkRequesterResponse(NetworkRequesterResponse),
}

impl Message {
    const REQUEST_FLAG: u8 = 0;
    const RESPONSE_FLAG: u8 = 1;
    const NR_RESPONSE_FLAG: u8 = 2;

    pub fn conn_id(&self) -> u64 {
        match self {
            Message::Request(req) => match req {
                Request::Connect(c) => c.conn_id,
                Request::Send(s) => s.conn_id,
            },
            Message::Response(resp) => resp.connection_id,
            Message::NetworkRequesterResponse(resp) => resp.connection_id,
        }
    }

    pub fn size(&self) -> usize {
        match self {
            Message::Request(req) => match req {
                Request::Connect(_) => 0,
                Request::Send(s) => s.data.len(),
            },
            Message::Response(resp) => resp.data.len(),
            Message::NetworkRequesterResponse(_) => 0,
        }
    }

    pub fn try_from_bytes(b: &[u8]) -> Result<Message, MessageError> {
        if b.is_empty() {
            return Err(MessageError::NoData);
        }

        if b[0] == Self::REQUEST_FLAG {
            Request::try_from_bytes(&b[1..])
                .map(Message::Request)
                .map_err(MessageError::Request)
        } else if b[0] == Self::RESPONSE_FLAG {
            Response::try_from_bytes(&b[1..])
                .map(Message::Response)
                .map_err(MessageError::Response)
        } else if b[0] == Self::NR_RESPONSE_FLAG {
            NetworkRequesterResponse::try_from_bytes(&b[1..])
                .map(Message::NetworkRequesterResponse)
                .map_err(MessageError::NetworkRequesterResponseError)
        } else {
            Err(MessageError::UnknownMessageType)
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            Self::Request(r) => std::iter::once(Self::REQUEST_FLAG)
                .chain(r.into_bytes().into_iter())
                .collect(),
            Self::Response(r) => std::iter::once(Self::RESPONSE_FLAG)
                .chain(r.into_bytes().into_iter())
                .collect(),
            Self::NetworkRequesterResponse(r) => std::iter::once(Self::NR_RESPONSE_FLAG)
                .chain(r.into_bytes().into_iter())
                .collect(),
        }
    }
}
