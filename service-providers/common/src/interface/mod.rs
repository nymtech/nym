// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub use control::{ControlRequest, ControlResponse};
use thiserror::Error;

mod control;

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

    #[error("control request did not contain any data")]
    EmptyControlRequest,

    #[error("control response did not contain any data")]
    EmptyControlResponse,

    #[error("the received binary information control response was malformed: {source}")]
    MalformedBinaryInfoControlResponse { source: serde_json::Error },
}

pub trait ServiceProviderRequest: Serializable {
    type Response: ServiceProviderResponse;
    // TODO: should this one perhaps be separated into RequestError and ResponseError?
    type Error: From<ServiceProviderMessagingError>
        + From<<Self as Serializable>::Error>
        + From<<Self::Response as Serializable>::Error>;
}

pub trait ServiceProviderResponse: Serializable {}

// can't use 'normal' trait (i.e. Serialize/Deserialize from serde) as `Socks5Message` uses custom serialization
// and we don't want to break backwards compatibility
pub trait Serializable: Sized {
    type Error;

    fn into_bytes(self) -> Vec<u8>;

    fn try_from_bytes(b: &[u8]) -> Result<Self, Self::Error>;
}

pub enum Request<T: ServiceProviderRequest = EmptyMessage> {
    Control(ControlRequest),
    ProviderData(T),
}

pub enum Response<T: ServiceProviderRequest = EmptyMessage> {
    Control(ControlResponse),
    ProviderData(T::Response),
}

#[repr(u8)]
pub enum RequestTag {
    /// Value tag representing legacy value for `Socks5Message::Request`
    LegacySocks5Request = 0,

    /// Value tag representing legacy value for `Socks5Message::Response`
    LegacySocks5Response = 1,

    /// Value tag representing legacy value for `Socks5Message::NetworkRequesterResponse`
    LegacySocks5NRResponse = 2,

    /// Value tag representing [`Control`] variant of the [`Request`]
    Control = 0x03,

    /// Value tag representing [`ProviderData`] variant of the [`Request`]
    ProviderData = 0x04,
}

impl TryFrom<u8> for RequestTag {
    type Error = ServiceProviderMessagingError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            _ if value == (Self::LegacySocks5Request as u8) => Ok(Self::LegacySocks5Request),
            _ if value == (Self::LegacySocks5Response as u8) => Ok(Self::LegacySocks5Response),
            _ if value == (Self::LegacySocks5NRResponse as u8) => Ok(Self::LegacySocks5NRResponse),
            _ if value == (Self::Control as u8) => Ok(Self::Control),
            _ if value == (Self::ProviderData as u8) => Ok(Self::ProviderData),
            received => Err(ServiceProviderMessagingError::InvalidRequestTag { received }),
        }
    }
}

impl RequestTag {
    pub fn is_legacy(&self) -> bool {
        matches!(
            self,
            RequestTag::LegacySocks5Request
                | RequestTag::LegacySocks5Response
                | RequestTag::LegacySocks5NRResponse
        )
    }
}

impl<T> Request<T>
where
    T: ServiceProviderRequest,
{
    fn tag(&self) -> RequestTag {
        match self {
            Request::Control(_) => RequestTag::Control,
            Request::ProviderData(_) => RequestTag::ProviderData,
        }
    }

    fn serialize_inner(self) -> Vec<u8> {
        match self {
            Request::Control(control) => control.into_bytes(),
            Request::ProviderData(provider_data) => provider_data.into_bytes(),
        }
    }

    pub fn peek_tag(b: &[u8]) -> Result<RequestTag, <T as ServiceProviderRequest>::Error> {
        if b.is_empty() {
            return Err(ServiceProviderMessagingError::EmptyRequest.into());
        }

        RequestTag::try_from(b[0]).map_err(Into::into)
    }

    pub fn into_bytes(self) -> Vec<u8> {
        std::iter::once(self.tag() as u8)
            .chain(self.serialize_inner().into_iter())
            .collect()
    }

    pub fn try_from_bytes(b: &[u8]) -> Result<Request<T>, <T as ServiceProviderRequest>::Error> {
        if b.is_empty() {
            return Err(ServiceProviderMessagingError::EmptyRequest.into());
        }

        let request_tag = RequestTag::try_from(b[0])?;
        match request_tag {
            RequestTag::Control => Ok(Request::Control(ControlRequest::try_from_bytes(&b[1..])?)),
            RequestTag::ProviderData => Ok(Request::ProviderData(T::try_from_bytes(&b[1..])?)),
            _ => todo!("handle legacy"),
        }
    }
}

#[repr(u8)]
pub enum ResponseTag {
    /// Value tag representing legacy value for `Socks5Message::Request`
    LegacySocks5Request = 0,

    /// Value tag representing legacy value for `Socks5Message::Response`
    LegacySocks5Response = 1,

    /// Value tag representing legacy value for `Socks5Message::NetworkRequesterResponse`
    LegacySocks5NRResponse = 2,

    /// Value tag representing [`Control`] variant of the [`Reponse`]
    Control = 0x03,

    /// Value tag representing [`ProviderData`] variant of the [`Reponse`]
    ProviderData = 0x04,
}

impl TryFrom<u8> for ResponseTag {
    type Error = ServiceProviderMessagingError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            _ if value == (Self::LegacySocks5Request as u8) => Ok(Self::LegacySocks5Request),
            _ if value == (Self::LegacySocks5Response as u8) => Ok(Self::LegacySocks5Response),
            _ if value == (Self::LegacySocks5NRResponse as u8) => Ok(Self::LegacySocks5NRResponse),
            _ if value == (Self::Control as u8) => Ok(Self::Control),
            _ if value == (Self::ProviderData as u8) => Ok(Self::ProviderData),
            received => Err(ServiceProviderMessagingError::InvalidResponseTag { received }),
        }
    }
}

impl ResponseTag {
    pub fn is_legacy(&self) -> bool {
        matches!(
            self,
            ResponseTag::LegacySocks5Request
                | ResponseTag::LegacySocks5Response
                | ResponseTag::LegacySocks5NRResponse
        )
    }
}

impl<T> Response<T>
where
    T: ServiceProviderRequest,
{
    fn tag(&self) -> ResponseTag {
        match self {
            Response::Control(_) => ResponseTag::Control,
            Response::ProviderData(_) => ResponseTag::ProviderData,
        }
    }

    fn serialize_inner(self) -> Vec<u8> {
        match self {
            Response::Control(control) => control.into_bytes(),
            Response::ProviderData(provider_data) => provider_data.into_bytes(),
        }
    }

    pub fn peek_tag(b: &[u8]) -> Result<ResponseTag, <T as ServiceProviderRequest>::Error> {
        if b.is_empty() {
            return Err(ServiceProviderMessagingError::EmptyResponse.into());
        }

        ResponseTag::try_from(b[0]).map_err(Into::into)
    }

    pub fn into_bytes(self) -> Vec<u8> {
        std::iter::once(self.tag() as u8)
            .chain(self.serialize_inner().into_iter())
            .collect()
    }

    pub fn try_from_bytes(b: &[u8]) -> Result<Response<T>, <T as ServiceProviderRequest>::Error> {
        if b.is_empty() {
            return Err(ServiceProviderMessagingError::EmptyResponse.into());
        }

        let request_tag = ResponseTag::try_from(b[0])?;
        match request_tag {
            ResponseTag::Control => {
                Ok(Response::Control(ControlResponse::try_from_bytes(&b[1..])?))
            }
            ResponseTag::ProviderData => Ok(Response::ProviderData(T::Response::try_from_bytes(
                &b[1..],
            )?)),
            _ => todo!("handle legacy"),
        }
    }
}

pub struct EmptyMessage;

impl ServiceProviderRequest for EmptyMessage {
    type Response = EmptyMessage;
    type Error = ServiceProviderMessagingError;
}

impl ServiceProviderResponse for EmptyMessage {}

impl Serializable for EmptyMessage {
    type Error = ServiceProviderMessagingError;

    fn into_bytes(self) -> Vec<u8> {
        Vec::new()
    }

    fn try_from_bytes(_b: &[u8]) -> Result<Self, Self::Error> {
        Ok(EmptyMessage)
    }
}
