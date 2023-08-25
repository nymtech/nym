// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::interface::version::Version;
use crate::interface::{
    ControlRequest, EmptyMessage, ProviderInterfaceVersion, Serializable,
    ServiceProviderMessagingError, ServiceProviderResponse,
};
use log::warn;
use std::fmt::Debug;

pub trait ServiceProviderRequest: Serializable + Debug {
    type ProtocolVersion: Version + Debug + Clone;
    type Response: ServiceProviderResponse;
    // TODO: should this one perhaps be separated into RequestError and ResponseError?
    type Error: From<ServiceProviderMessagingError>
        + From<<Self as Serializable>::Error>
        + From<<Self::Response as Serializable>::Error>;

    /// The version of the provider protocol attached on the particular request.
    fn provider_specific_version(&self) -> Self::ProtocolVersion;

    /// The highest version of the provider protocol that can be supported by this party.
    fn max_supported_version() -> Self::ProtocolVersion;
}

#[derive(Debug)]
pub struct Request<T: ServiceProviderRequest = EmptyMessage> {
    pub interface_version: ProviderInterfaceVersion,
    pub content: RequestContent<T>,
}

#[derive(Debug)]
pub enum RequestContent<T: ServiceProviderRequest = EmptyMessage> {
    Control(ControlRequest),
    ProviderData(T),
}

#[repr(u8)]
pub enum RequestTag {
    /// Value tag representing [`Control`] variant of the [`Request`]
    Control = 0x00,

    /// Value tag representing [`ProviderData`] variant of the [`Request`]
    ProviderData = 0x01,
}

impl TryFrom<u8> for RequestTag {
    type Error = ServiceProviderMessagingError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            _ if value == (Self::Control as u8) => Ok(Self::Control),
            _ if value == (Self::ProviderData as u8) => Ok(Self::ProviderData),
            received => Err(ServiceProviderMessagingError::InvalidRequestTag { received }),
        }
    }
}

impl<T> Request<T>
where
    T: ServiceProviderRequest,
{
    pub fn new_control(
        interface_version: ProviderInterfaceVersion,
        content: ControlRequest,
    ) -> Self {
        Request {
            interface_version,
            content: RequestContent::Control(content),
        }
    }

    pub fn new_provider_data(interface_version: ProviderInterfaceVersion, content: T) -> Self {
        Request {
            interface_version,
            content: RequestContent::ProviderData(content),
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

    pub fn try_from_bytes(b: &[u8]) -> Result<Request<T>, <T as ServiceProviderRequest>::Error> {
        if b.is_empty() {
            return Err(ServiceProviderMessagingError::EmptyRequest.into());
        }

        let interface_version = ProviderInterfaceVersion::from(b[0]);
        let content = if interface_version.is_legacy() {
            RequestContent::try_from_bytes(b, interface_version)
        } else {
            RequestContent::try_from_bytes(&b[1..], interface_version)
        }?;

        Ok(Request {
            interface_version,
            content,
        })
    }
}

impl<T> RequestContent<T>
where
    T: ServiceProviderRequest,
{
    fn tag(&self) -> RequestTag {
        match self {
            RequestContent::Control(_) => RequestTag::Control,
            RequestContent::ProviderData(_) => RequestTag::ProviderData,
        }
    }

    fn serialize_inner(self) -> Vec<u8> {
        match self {
            RequestContent::Control(control) => control.into_bytes(),
            RequestContent::ProviderData(provider_data) => provider_data.into_bytes(),
        }
    }

    fn into_bytes(self, interface_version: ProviderInterfaceVersion) -> Vec<u8> {
        if interface_version.is_legacy() {
            if matches!(self, RequestContent::Control(_)) {
                // this shouldn't ever happen, since if client is aware of control requests,
                // it should be aware of versioning and shouldn't attempt to send those
                warn!("attempted to serialize a control request in legacy mode");
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
    ) -> Result<RequestContent<T>, <T as ServiceProviderRequest>::Error> {
        if interface_version.is_legacy() {
            // we received a request from an old client which can only possibly
            // use an old Socks5Message, which uses the entire buffer for deserialization
            Ok(RequestContent::ProviderData(T::try_from_bytes(b)?))
        } else {
            if b.is_empty() {
                return Err(ServiceProviderMessagingError::IncompleteRequest {
                    received: b.len() + 1,
                }
                .into());
            }

            let request_tag = RequestTag::try_from(b[0])?;
            match request_tag {
                RequestTag::Control => Ok(RequestContent::Control(ControlRequest::try_from_bytes(
                    &b[1..],
                )?)),
                RequestTag::ProviderData => {
                    Ok(RequestContent::ProviderData(T::try_from_bytes(&b[1..])?))
                }
            }
        }
    }
}
