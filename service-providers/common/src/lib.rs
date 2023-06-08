// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::interface::{
    BinaryInformation, ControlRequest, ControlResponse, EmptyMessage, ProviderInterfaceVersion,
    Request, RequestContent, Response, ResponseContent, ServiceProviderRequest, SupportedVersions,
};
use async_trait::async_trait;
use nym_sphinx_anonymous_replies::requests::AnonymousSenderTag;

pub mod interface;

// FUTURE WORK:
// for next version (v4) of the interface, refactor `reply::MixnetAddress` from the socks5 SP
// and move it here so that you could optionally attach your sender address with any request for easier responses

/// Trait that every ServiceProvider on the Nym network should implement and adhere to.
#[async_trait]
pub trait ServiceProvider<T: ServiceProviderRequest = EmptyMessage>
where
    T: Send + 'static,
    Self: Sync,
{
    type ServiceProviderError: From<<T as ServiceProviderRequest>::Error>;

    // TODO: refactor to use some version of `reply::MixnetAddress`
    // in case explicit address was provided
    async fn on_request(
        &mut self,
        sender: Option<AnonymousSenderTag>,
        request: Request<T>,
    ) -> Result<(), Self::ServiceProviderError>;

    async fn handle_request(
        &mut self,
        sender: Option<AnonymousSenderTag>,
        request: Request<T>,
    ) -> Result<Option<Response<T>>, Self::ServiceProviderError> {
        match request.content {
            RequestContent::Control(control_request) => self
                .handle_control_request(sender, control_request, request.interface_version)
                .await
                .map(|maybe_res| {
                    maybe_res.map(|control_res| Response {
                        interface_version: request.interface_version,
                        content: ResponseContent::Control(control_res),
                    })
                }),
            RequestContent::ProviderData(provider_data_request) => self
                .handle_provider_data_request(
                    sender,
                    provider_data_request,
                    request.interface_version,
                )
                .await
                .map(|maybe_res| {
                    maybe_res.map(|provider_data_res| Response {
                        interface_version: request.interface_version,
                        content: ResponseContent::ProviderData(provider_data_res),
                    })
                }),
        }
    }

    async fn handle_control_request(
        &mut self,
        _sender: Option<AnonymousSenderTag>,
        request: ControlRequest,
        interface_version: ProviderInterfaceVersion,
    ) -> Result<Option<ControlResponse>, Self::ServiceProviderError> {
        if interface_version.is_legacy() {
            // control requests didn't exist in the legacy version
            Ok(None)
        } else {
            let response = match request {
                // Version 3 requests:
                ControlRequest::Health => {
                    self.handle_health_control_request().await?;
                    Some(ControlResponse::Health)
                }
                ControlRequest::BinaryInfo => {
                    let info = self.handle_binary_info_control_request().await?;
                    Some(ControlResponse::BinaryInfo(Box::new(info)))
                }
                ControlRequest::SupportedRequestVersions => {
                    let versions = self.handle_supported_request_versions_request().await?;
                    Some(ControlResponse::SupportedRequestVersions(versions))
                } //
                  // TODO: if we ever add new request for interface version 4 (or higher),
                  // we need to include a check to make sure we return a `None` if passed `interface_version` was 3
            };
            Ok(response)
        }
    }

    // I don't think you can do anything more simple than that, but allow for custom implementations
    // in case, for example, you wanted to include additional statistics collection here
    async fn handle_health_control_request(&self) -> Result<(), Self::ServiceProviderError> {
        Ok(())
    }

    // well, this has to be handled manually
    async fn handle_binary_info_control_request(
        &self,
    ) -> Result<BinaryInformation, Self::ServiceProviderError>;

    async fn handle_supported_request_versions_request(
        &self,
    ) -> Result<SupportedVersions, Self::ServiceProviderError> {
        Ok(SupportedVersions {
            interface_version: ProviderInterfaceVersion::new_current().to_string(),
            provider_version: T::max_supported_version().to_string(),
        })
    }

    async fn handle_provider_data_request(
        &mut self,
        sender: Option<AnonymousSenderTag>,
        request: T,
        interface_version: ProviderInterfaceVersion,
    ) -> Result<Option<T::Response>, Self::ServiceProviderError>;
}

// #[async_trait]
// pub trait ServiceProviderClient<T: ServiceProviderRequest = EmptyMessage>
// where
//     T: Send + 'static,
//     Self: Sync,
// {
//     type ServiceProviderClientError: From<<T as ServiceProviderRequest>::Error>;
//
//     fn provider_interface_version(&self) -> ProviderInterfaceVersion;
//
//     fn get_control_response(
//         maybe_res: Option<Response<T>>,
//     ) -> Result<Option<ControlResponse>, Self::ServiceProviderClientError> {
//         let Some(res) = maybe_res else {
//                 return Ok(None)
//             };
//
//         match res.content {
//             ResponseContent::Control(res) => Ok(Some(res)),
//             ResponseContent::ProviderData(_) => Err(Self::ServiceProviderClientError::from(
//                 ServiceProviderMessagingError::UnexpectedProviderDataResponse.into(),
//             )),
//         }
//     }
//
//     fn get_provider_data_response(
//         maybe_res: Option<Response<T>>,
//     ) -> Result<Option<T::Response>, Self::ServiceProviderClientError> {
//         let Some(res) = maybe_res else {
//                 return Ok(None)
//             };
//
//         match res.content {
//             ResponseContent::ProviderData(res) => Ok(Some(res)),
//             ResponseContent::Control(_) => Err(Self::ServiceProviderClientError::from(
//                 ServiceProviderMessagingError::UnexpectedControlResponse.into(),
//             )),
//         }
//     }
//
//     // async fn send_control_request_anonymously(
//     //     &mut self,
//     //     request: ControlRequest,
//     // ) -> Result<Option<ControlResponse>, Self::ServiceProviderClientError>;
//     //
//     // async fn send_provider_data_request_anonymously(
//     //     &mut self,
//     //     request: T,
//     // ) -> Result<Option<T::Response>, Self::ServiceProviderClientError>;
//     //
//     // TODO: extend the traits and messaging to allow for attaching own address
//     // async fn send_control_request_with_address(
//     //     &mut self,
//     //     request: ControlRequest,
//     // ) -> Result<Option<ControlResponse>, Self::ServiceProviderClientError>;
//     //
//     // async fn send_provider_data_request_with_address(
//     //     &mut self,
//     //     request: T,
//     // ) -> Result<Option<T::Response>, Self::ServiceProviderClientError>;
// }
