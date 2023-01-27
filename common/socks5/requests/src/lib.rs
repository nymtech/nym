// Copyright 2020-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use service_providers_common::interface;
use service_providers_common::interface::ServiceProviderMessagingError;
use thiserror::Error;

pub use request::*;
pub use response::*;
pub use version::*;

pub mod request;
pub mod response;
pub mod version;

pub type Socks5ProviderRequest = interface::Request<Socks5Request>;
pub type Socks5ProviderResponse = interface::Response<Socks5Request>;

#[derive(Debug, Error)]
pub enum Socks5RequestError {
    #[error("failed to deserialize received request: {source}")]
    RequestDeserialization {
        #[from]
        source: RequestDeserializationError,
    },

    #[error("failed to deserialize received response: {source}")]
    ResponseDeserialization {
        #[from]
        source: ResponseDeserializationError,
    },

    #[error(transparent)]
    ProviderInterfaceError(#[from] ServiceProviderMessagingError),
}
