// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use control::{ControlRequest, ControlResponse};
pub use request::{Request, RequestContent, ServiceProviderRequest};
pub use response::{Response, ResponseContent, ServiceProviderResponse};

use thiserror::Error;

mod control;
mod request;
mod response;

/// Defines initial version of the communication interface between clients and service providers.
// note: we start from '3' so that we could distinguish cases where no version is provided
// and legacy communication mode is used instead
pub const INITIAL_INTERFACE_VERSION: u8 = 3;

/// Defines the current version of the communication interface between clients and service providers.
/// It has to be incremented for any breaking change.
pub const INTERFACE_VERSION: u8 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InterfaceVersion {
    Legacy,
    Versioned(u8),
}

impl InterfaceVersion {
    pub fn new(use_legacy: bool) -> Self {
        if use_legacy {
            Self::new_legacy()
        } else {
            Self::new_versioned(INTERFACE_VERSION)
        }
    }

    pub fn new_legacy() -> Self {
        InterfaceVersion::Legacy
    }

    pub fn new_versioned(version: u8) -> Self {
        InterfaceVersion::Versioned(version)
    }

    pub fn is_legacy(&self) -> bool {
        matches!(self, InterfaceVersion::Legacy)
    }

    pub fn as_u8(&self) -> Option<u8> {
        match self {
            InterfaceVersion::Legacy => None,
            InterfaceVersion::Versioned(version) => Some(*version),
        }
    }
}

impl From<u8> for InterfaceVersion {
    fn from(v: u8) -> Self {
        match v {
            n if n < INITIAL_INTERFACE_VERSION => InterfaceVersion::Legacy,
            n => InterfaceVersion::Versioned(n),
        }
    }
}

impl Default for InterfaceVersion {
    fn default() -> Self {
        InterfaceVersion::Versioned(INTERFACE_VERSION)
    }
}

#[derive(Debug, Error)]
pub enum ServiceProviderMessagingError {
    #[error("{received} does not correspond to any valid request tag")]
    InvalidRequestTag { received: u8 },

    #[error("{received} does not correspond to any valid response tag")]
    InvalidResponseTag { received: u8 },

    #[error("{received} does not correspond to any valid control request tag")]
    InvalidControlRequestTag { received: u8 },

    #[error("{received} does not correspond to any valid control response tag")]
    InvalidControlResponseTag { received: u8 },

    #[error("request did not contain any data")]
    EmptyRequest,

    #[error("response did not contain any data")]
    EmptyResponse,

    #[error("request did not contain enough data to get deserialized. Got only {received} bytes.")]
    IncompleteRequest { received: usize },

    #[error(
        "response did not contain enough data to get deserialized. Got only {received} bytes."
    )]
    IncompleteResponse { received: usize },

    #[error("control request did not contain any data")]
    EmptyControlRequest,

    #[error("control response did not contain any data")]
    EmptyControlResponse,

    #[error("the received binary information control response was malformed: {source}")]
    MalformedBinaryInfoControlResponse { source: serde_json::Error },
}

// can't use 'normal' trait (i.e. Serialize/Deserialize from serde) as `Socks5Message` uses custom serialization
// and we don't want to break backwards compatibility
pub trait Serializable: Sized {
    type Error;

    fn into_bytes(self) -> Vec<u8>;

    fn try_from_bytes(b: &[u8]) -> Result<Self, Self::Error>;
}

// pub fn is_legacy_version(version: u8) -> bool {
//     if version < INITIAL_INTERFACE_VERSION {
//         true
//     } else {
//         false
//     }
// }

pub struct EmptyMessage;

impl ServiceProviderRequest for EmptyMessage {
    type Response = EmptyMessage;
    type Error = ServiceProviderMessagingError;

    // fn provider_specific_version(&self) -> u8 {
    //     1
    // }
}

impl ServiceProviderResponse for EmptyMessage {
    // fn provider_specific_version(&self) -> u8 {
    //     1
    // }
}

impl Serializable for EmptyMessage {
    type Error = ServiceProviderMessagingError;

    fn into_bytes(self) -> Vec<u8> {
        Vec::new()
    }

    fn try_from_bytes(_b: &[u8]) -> Result<Self, Self::Error> {
        Ok(EmptyMessage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyRequest {
        //
    }

    struct DummyResponse {
        //
    }

    #[test]
    fn old_client_vs_old_service_provider() {
        todo!()
    }

    #[test]
    fn old_client_vs_new_service_provider() {
        todo!()
    }

    #[test]
    fn new_client_vs_old_service_provider() {
        todo!()
    }

    #[test]
    fn new_client_vs_new_service_provider() {
        todo!()
    }
}
