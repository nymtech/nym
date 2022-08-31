// Copyright 2020-2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

use crate::request::{Request, RequestError};
use crate::response::{Response, ResponseError};

#[derive(Debug, Error)]
pub enum MessageError {
    #[error("{0}")]
    Request(RequestError),

    #[error("{0:?}")]
    Response(ResponseError),

    #[error("no data")]
    NoData,

    #[error("unknown message type received")]
    UnknownMessageType,
}

pub enum Message {
    Request(Request),
    Response(Response),
}

impl Message {
    const REQUEST_FLAG: u8 = 0;
    const RESPONSE_FLAG: u8 = 1;

    pub fn conn_id(&self) -> u64 {
        match self {
            Message::Request(req) => match req {
                Request::Connect(c) => c.conn_id,
                Request::Send(conn_id, _, _) => *conn_id,
            },
            Message::Response(resp) => resp.connection_id,
        }
    }

    pub fn size(&self) -> usize {
        match self {
            Message::Request(req) => match req {
                Request::Connect(_) => 0,
                Request::Send(_, data, _) => data.len(),
            },
            Message::Response(resp) => resp.data.len(),
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
        } else {
            Err(MessageError::UnknownMessageType)
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            Self::Request(r) => std::iter::once(Self::REQUEST_FLAG)
                .chain(r.into_bytes().iter().cloned())
                .collect(),
            Self::Response(r) => std::iter::once(Self::RESPONSE_FLAG)
                .chain(r.into_bytes().iter().cloned())
                .collect(),
        }
    }
}
