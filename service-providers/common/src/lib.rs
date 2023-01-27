// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::interface::{
    BinaryInformation, ControlRequest, ControlResponse, EmptyMessage, ProviderInterfaceVersion,
    Request, RequestContent, Response, ResponseContent, ServiceProviderRequest,
};
use async_trait::async_trait;

pub mod interface;

/// Trait that every ServiceProvider on the Nym network should implement and adhere to.
#[async_trait]
pub trait ServiceProvider<T: ServiceProviderRequest = EmptyMessage>
where
    T: Send + 'static,
    Self: Sync,
{
    type ServiceProviderError: From<<T as ServiceProviderRequest>::Error>;

    async fn handle_request(
        &mut self,
        request: Request<T>,
    ) -> Result<Option<Response<T>>, Self::ServiceProviderError> {
        match request.content {
            RequestContent::Control(control_request) => self
                .handle_control_request(control_request, request.interface_version)
                .await
                .map(|maybe_res| {
                    maybe_res.map(|control_res| Response {
                        interface_version: request.interface_version,
                        content: ResponseContent::Control(control_res),
                    })
                }),
            RequestContent::ProviderData(provider_data_request) => self
                .handle_provider_data_request(provider_data_request, request.interface_version)
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
                    // let versions = self.handle_supported_request_versions_request().await?;
                    // Some(ControlResponse::SupportedRequestVersions)
                    todo!()
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

    // async fn handle_supported_request_versions_request(
    //     &self,
    // ) -> Result<RequestVersion<T>, Self::ServiceProviderError> {
    //     todo!("figure out how to get rid of that generic here since control responses shouldn't have to know about it")
    //     // Ok(RequestVersion {
    //     //     provider_interface: ProviderInterfaceVersion::new_current(),
    //     //     provider_protocol: T::max_supported_version(),
    //     // })
    // }

    async fn handle_provider_data_request(
        &mut self,
        request: T,
        interface_version: ProviderInterfaceVersion,
    ) -> Result<Option<T::Response>, Self::ServiceProviderError>;
}
