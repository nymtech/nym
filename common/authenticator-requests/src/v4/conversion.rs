// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_service_provider_requests_common::{Protocol, ServiceProviderType};

use crate::{v3, v4};

impl From<v3::request::AuthenticatorRequest> for v4::request::AuthenticatorRequest {
    fn from(authenticator_request: v3::request::AuthenticatorRequest) -> Self {
        Self {
            protocol: Protocol {
                version: 2,
                service_provider_type: ServiceProviderType::Authenticator,
            },
            data: authenticator_request.data.into(),
            reply_to: authenticator_request.reply_to,
            request_id: authenticator_request.request_id,
        }
    }
}

impl From<v3::request::AuthenticatorRequestData> for v4::request::AuthenticatorRequestData {
    fn from(authenticator_request_data: v3::request::AuthenticatorRequestData) -> Self {
        match authenticator_request_data {
            v3::request::AuthenticatorRequestData::Initial(init_msg) => {
                v4::request::AuthenticatorRequestData::Initial(init_msg.into())
            }
            v3::request::AuthenticatorRequestData::Final(gw_client) => {
                v4::request::AuthenticatorRequestData::Final(gw_client.into())
            }
            v3::request::AuthenticatorRequestData::QueryBandwidth(pub_key) => {
                v4::request::AuthenticatorRequestData::QueryBandwidth(pub_key)
            }
            v3::request::AuthenticatorRequestData::TopUpBandwidth(top_up_message) => {
                v4::request::AuthenticatorRequestData::TopUpBandwidth(top_up_message.into())
            }
        }
    }
}

impl From<v3::registration::InitMessage> for v4::registration::InitMessage {
    fn from(init_msg: v3::registration::InitMessage) -> Self {
        Self {
            pub_key: init_msg.pub_key,
        }
    }
}

impl From<Box<v3::registration::FinalMessage>> for Box<v4::registration::FinalMessage> {
    fn from(gw_client: Box<v3::registration::FinalMessage>) -> Self {
        Box::new(v4::registration::FinalMessage {
            gateway_client: gw_client.gateway_client.into(),
            credential: gw_client.credential,
        })
    }
}

impl From<Box<v3::topup::TopUpMessage>> for Box<v4::topup::TopUpMessage> {
    fn from(top_up_message: Box<v3::topup::TopUpMessage>) -> Self {
        Box::new(v4::topup::TopUpMessage {
            pub_key: top_up_message.pub_key,
            credential: top_up_message.credential,
        })
    }
}

impl From<v3::registration::GatewayClient> for v4::registration::GatewayClient {
    fn from(gw_client: v3::registration::GatewayClient) -> Self {
        Self {
            pub_key: gw_client.pub_key,
            private_ips: gw_client.private_ip.into(),
            mac: gw_client.mac.into(),
        }
    }
}

impl From<v4::registration::GatewayClient> for v3::registration::GatewayClient {
    fn from(gw_client: v4::registration::GatewayClient) -> Self {
        Self {
            pub_key: gw_client.pub_key,
            private_ip: gw_client.private_ips.ipv4.into(),
            mac: gw_client.mac.into(),
        }
    }
}

impl From<v3::registration::ClientMac> for v4::registration::ClientMac {
    fn from(mac: v3::registration::ClientMac) -> Self {
        Self::new(mac.to_vec())
    }
}

impl From<v4::registration::ClientMac> for v3::registration::ClientMac {
    fn from(mac: v4::registration::ClientMac) -> Self {
        Self::new(mac.to_vec())
    }
}

impl TryFrom<v4::response::AuthenticatorResponse> for v3::response::AuthenticatorResponse {
    type Error = crate::Error;

    fn try_from(
        authenticator_response: v4::response::AuthenticatorResponse,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            data: authenticator_response.data.try_into()?,
            reply_to: authenticator_response.reply_to,
            protocol: authenticator_response.protocol,
        })
    }
}

impl TryFrom<v4::response::AuthenticatorResponseData> for v3::response::AuthenticatorResponseData {
    type Error = crate::Error;

    fn try_from(
        authenticator_response_data: v4::response::AuthenticatorResponseData,
    ) -> Result<Self, Self::Error> {
        match authenticator_response_data {
            v4::response::AuthenticatorResponseData::PendingRegistration(
                pending_registration_response,
            ) => Ok(
                v3::response::AuthenticatorResponseData::PendingRegistration(
                    pending_registration_response.into(),
                ),
            ),
            v4::response::AuthenticatorResponseData::Registered(registered_response) => Ok(
                v3::response::AuthenticatorResponseData::Registered(registered_response.into()),
            ),
            v4::response::AuthenticatorResponseData::RemainingBandwidth(
                remaining_bandwidth_response,
            ) => Ok(v3::response::AuthenticatorResponseData::RemainingBandwidth(
                remaining_bandwidth_response.into(),
            )),
            v4::response::AuthenticatorResponseData::TopUpBandwidth(_) => {
                Err(Self::Error::Conversion(
                    "a v3 request couldn't produce a v4 only type of response".to_string(),
                ))
            }
        }
    }
}

impl From<v4::response::PendingRegistrationResponse> for v3::response::PendingRegistrationResponse {
    fn from(value: v4::response::PendingRegistrationResponse) -> Self {
        Self {
            request_id: value.request_id,
            reply_to: value.reply_to,
            reply: value.reply.into(),
        }
    }
}

impl From<v4::response::RegisteredResponse> for v3::response::RegisteredResponse {
    fn from(value: v4::response::RegisteredResponse) -> Self {
        Self {
            request_id: value.request_id,
            reply_to: value.reply_to,
            reply: value.reply.into(),
        }
    }
}

impl From<v4::response::RemainingBandwidthResponse> for v3::response::RemainingBandwidthResponse {
    fn from(value: v4::response::RemainingBandwidthResponse) -> Self {
        Self {
            request_id: value.request_id,
            reply_to: value.reply_to,
            reply: value.reply.map(Into::into),
        }
    }
}

impl From<v4::registration::RegistrationData> for v3::registration::RegistrationData {
    fn from(value: v4::registration::RegistrationData) -> Self {
        Self {
            nonce: value.nonce,
            gateway_data: value.gateway_data.into(),
            wg_port: value.wg_port,
        }
    }
}

impl From<v4::registration::RegistredData> for v3::registration::RegistredData {
    fn from(value: v4::registration::RegistredData) -> Self {
        Self {
            pub_key: value.pub_key,
            private_ip: value.private_ips.ipv4.into(),
            wg_port: value.wg_port,
        }
    }
}

impl From<v4::registration::RemainingBandwidthData> for v3::registration::RemainingBandwidthData {
    fn from(value: v4::registration::RemainingBandwidthData) -> Self {
        Self {
            available_bandwidth: value.available_bandwidth,
        }
    }
}
