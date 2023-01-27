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

pub type Socks5ProviderRequest = interface::Request<Socks5Request>;
pub type Socks5ProviderResponse = interface::Response<Socks5Request>;
