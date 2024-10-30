// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_service_provider_requests_common::{Protocol, ServiceProviderType};

use crate::{v2, v3};

impl From<v2::request::AuthenticatorRequest> for v3::request::AuthenticatorRequest {
    fn from(authenticator_request: v2::request::AuthenticatorRequest) -> Self {
        Self {
            protocol: Protocol {
                version: 3,
                service_provider_type: ServiceProviderType::Authenticator,
            },
            data: authenticator_request.data.into(),
            reply_to: authenticator_request.reply_to,
            request_id: authenticator_request.request_id,
        }
    }
}

impl From<v2::request::AuthenticatorRequestData> for v3::request::AuthenticatorRequestData {
    fn from(authenticator_request_data: v2::request::AuthenticatorRequestData) -> Self {
        match authenticator_request_data {
            v2::request::AuthenticatorRequestData::Initial(init_msg) => {
                v3::request::AuthenticatorRequestData::Initial(init_msg.into())
            }
            v2::request::AuthenticatorRequestData::Final(gw_client) => {
                v3::request::AuthenticatorRequestData::Final(gw_client.into())
            }
            v2::request::AuthenticatorRequestData::QueryBandwidth(pub_key) => {
                v3::request::AuthenticatorRequestData::QueryBandwidth(pub_key)
            }
        }
    }
}

impl From<v2::registration::InitMessage> for v3::registration::InitMessage {
    fn from(init_msg: v2::registration::InitMessage) -> Self {
        Self {
            pub_key: init_msg.pub_key,
        }
    }
}

impl From<Box<v2::registration::FinalMessage>> for Box<v3::registration::FinalMessage> {
    fn from(gw_client: Box<v2::registration::FinalMessage>) -> Self {
        Box::new(v3::registration::FinalMessage {
            gateway_client: gw_client.gateway_client.into(),
            credential: gw_client.credential,
        })
    }
}

impl From<v2::registration::GatewayClient> for v3::registration::GatewayClient {
    fn from(gw_client: v2::registration::GatewayClient) -> Self {
        Self {
            pub_key: gw_client.pub_key,
            private_ip: gw_client.private_ip,
            mac: gw_client.mac.into(),
        }
    }
}

impl From<v3::registration::GatewayClient> for v2::registration::GatewayClient {
    fn from(gw_client: v3::registration::GatewayClient) -> Self {
        Self {
            pub_key: gw_client.pub_key,
            private_ip: gw_client.private_ip,
            mac: gw_client.mac.into(),
        }
    }
}

impl From<v2::registration::ClientMac> for v3::registration::ClientMac {
    fn from(mac: v2::registration::ClientMac) -> Self {
        Self::new(mac.to_vec())
    }
}

impl From<v3::registration::ClientMac> for v2::registration::ClientMac {
    fn from(mac: v3::registration::ClientMac) -> Self {
        Self::new(mac.to_vec())
    }
}

impl TryFrom<v3::response::AuthenticatorResponse> for v2::response::AuthenticatorResponse {
    type Error = crate::Error;

    fn try_from(
        authenticator_response: v3::response::AuthenticatorResponse,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            data: authenticator_response.data.try_into()?,
            reply_to: authenticator_response.reply_to,
            protocol: authenticator_response.protocol,
        })
    }
}

impl TryFrom<v3::response::AuthenticatorResponseData> for v2::response::AuthenticatorResponseData {
    type Error = crate::Error;

    fn try_from(
        authenticator_response_data: v3::response::AuthenticatorResponseData,
    ) -> Result<Self, Self::Error> {
        match authenticator_response_data {
            v3::response::AuthenticatorResponseData::PendingRegistration(
                pending_registration_response,
            ) => Ok(
                v2::response::AuthenticatorResponseData::PendingRegistration(
                    pending_registration_response.into(),
                ),
            ),
            v3::response::AuthenticatorResponseData::Registered(registered_response) => Ok(
                v2::response::AuthenticatorResponseData::Registered(registered_response.into()),
            ),
            v3::response::AuthenticatorResponseData::RemainingBandwidth(
                remaining_bandwidth_response,
            ) => Ok(v2::response::AuthenticatorResponseData::RemainingBandwidth(
                remaining_bandwidth_response.into(),
            )),
            v3::response::AuthenticatorResponseData::TopUpBandwidth(_) => {
                Err(Self::Error::Conversion(
                    "a v2 request couldn't produce a v3 only type of response".to_string(),
                ))
            }
        }
    }
}

impl From<v3::response::PendingRegistrationResponse> for v2::response::PendingRegistrationResponse {
    fn from(value: v3::response::PendingRegistrationResponse) -> Self {
        Self {
            request_id: value.request_id,
            reply_to: value.reply_to,
            reply: value.reply.into(),
        }
    }
}

impl From<v3::response::RegisteredResponse> for v2::response::RegisteredResponse {
    fn from(value: v3::response::RegisteredResponse) -> Self {
        Self {
            request_id: value.request_id,
            reply_to: value.reply_to,
            reply: value.reply.into(),
        }
    }
}

impl From<v3::response::RemainingBandwidthResponse> for v2::response::RemainingBandwidthResponse {
    fn from(value: v3::response::RemainingBandwidthResponse) -> Self {
        Self {
            request_id: value.request_id,
            reply_to: value.reply_to,
            reply: value.reply.map(Into::into),
        }
    }
}

impl From<v3::registration::RegistrationData> for v2::registration::RegistrationData {
    fn from(value: v3::registration::RegistrationData) -> Self {
        Self {
            nonce: value.nonce,
            gateway_data: value.gateway_data.into(),
            wg_port: value.wg_port,
        }
    }
}

impl From<v3::registration::RegistredData> for v2::registration::RegistredData {
    fn from(value: v3::registration::RegistredData) -> Self {
        Self {
            pub_key: value.pub_key,
            private_ip: value.private_ip,
            wg_port: value.wg_port,
        }
    }
}

impl From<v3::registration::RemainingBandwidthData> for v2::registration::RemainingBandwidthData {
    fn from(value: v3::registration::RemainingBandwidthData) -> Self {
        Self {
            available_bandwidth: value.available_bandwidth,
        }
    }
}
