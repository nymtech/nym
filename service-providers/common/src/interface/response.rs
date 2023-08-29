// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::interface::{
    ControlResponse, EmptyMessage, ProviderInterfaceVersion, Serializable,
    ServiceProviderMessagingError, ServiceProviderRequest,
};
use log::warn;
use std::fmt::Debug;

pub trait ServiceProviderResponse: Serializable + Debug {}

#[derive(Debug)]
pub struct Response<T: ServiceProviderRequest = EmptyMessage> {
    pub interface_version: ProviderInterfaceVersion,
    pub content: ResponseContent<T>,
}

#[derive(Debug)]
pub enum ResponseContent<T: ServiceProviderRequest = EmptyMessage> {
    Control(ControlResponse),
    ProviderData(T::Response),
}

#[derive(Debug)]
#[repr(u8)]
pub enum ResponseTag {
    /// Value tag representing [`Control`] variant of the [`Response`]
    Control = 0x00,

    /// Value tag representing [`ProviderData`] variant of the [`Response`]
    ProviderData = 0x01,
}

impl TryFrom<u8> for ResponseTag {
    type Error = ServiceProviderMessagingError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            _ if value == (Self::Control as u8) => Ok(Self::Control),
            _ if value == (Self::ProviderData as u8) => Ok(Self::ProviderData),
            received => Err(ServiceProviderMessagingError::InvalidResponseTag { received }),
        }
    }
}

impl<T> Response<T>
where
    T: ServiceProviderRequest,
{
    pub fn new_control(
        interface_version: ProviderInterfaceVersion,
        content: ControlResponse,
    ) -> Self {
        Response {
            interface_version,
            content: ResponseContent::Control(content),
        }
    }

    pub fn new_provider_data(
        interface_version: ProviderInterfaceVersion,
        content: T::Response,
    ) -> Self {
        Response {
            interface_version,
            content: ResponseContent::ProviderData(content),
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        if let Some(version) = self.interface_version.as_u8() {
            std::iter::once(version)
                .chain(self.content.into_bytes(self.interface_version))
                .collect()
        } else {
            self.content.into_bytes(self.interface_version)
        }
    }

    pub fn try_from_bytes(b: &[u8]) -> Result<Response<T>, <T as ServiceProviderRequest>::Error> {
        if b.is_empty() {
            return Err(ServiceProviderMessagingError::EmptyResponse.into());
        }

        let interface_version = ProviderInterfaceVersion::from(b[0]);
        let content = if interface_version.is_legacy() {
            ResponseContent::try_from_bytes(b, interface_version)
        } else {
            ResponseContent::try_from_bytes(&b[1..], interface_version)
        }?;

        Ok(Response {
            interface_version,
            content,
        })
    }
}

impl<T> ResponseContent<T>
where
    T: ServiceProviderRequest,
{
    fn tag(&self) -> ResponseTag {
        match self {
            ResponseContent::Control(_) => ResponseTag::Control,
            ResponseContent::ProviderData(_) => ResponseTag::ProviderData,
        }
    }

    fn serialize_inner(self) -> Vec<u8> {
        match self {
            ResponseContent::Control(control) => control.into_bytes(),
            ResponseContent::ProviderData(provider_data) => provider_data.into_bytes(),
        }
    }

    fn into_bytes(self, interface_version: ProviderInterfaceVersion) -> Vec<u8> {
        if interface_version.is_legacy() {
            if matches!(self, ResponseContent::Control(_)) {
                // this shouldn't ever happen, since if service provider received a legacy request
                // it couldn't have possibly received a control request (unless client is trying to be funny)
                warn!("attempted to serialize a control response in legacy mode");
                Vec::new()
            } else {
                self.serialize_inner()
            }
        } else {
            std::iter::once(self.tag() as u8)
                .chain(self.serialize_inner())
                .collect()
        }
    }

    fn try_from_bytes(
        b: &[u8],
        interface_version: ProviderInterfaceVersion,
    ) -> Result<ResponseContent<T>, <T as ServiceProviderRequest>::Error> {
        if interface_version.is_legacy() {
            // we received a request from an old client which can only possibly
            // use an old Socks5Message, which uses the entire buffer for deserialization
            Ok(ResponseContent::ProviderData(T::Response::try_from_bytes(
                b,
            )?))
        } else {
            if b.is_empty() {
                return Err(ServiceProviderMessagingError::IncompleteResponse {
                    received: b.len() + 1,
                }
                .into());
            }

            let request_tag = ResponseTag::try_from(b[0])?;
            match request_tag {
                ResponseTag::Control => Ok(ResponseContent::Control(
                    ControlResponse::try_from_bytes(&b[1..])?,
                )),
                ResponseTag::ProviderData => Ok(ResponseContent::ProviderData(
                    T::Response::try_from_bytes(&b[1..])?,
                )),
            }
        }
    }
}
