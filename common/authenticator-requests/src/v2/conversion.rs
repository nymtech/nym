// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_service_provider_requests_common::{Protocol, ServiceProviderType};

use crate::{v1, v2};

impl From<v1::request::AuthenticatorRequest> for v2::request::AuthenticatorRequest {
    fn from(authenticator_request: v1::request::AuthenticatorRequest) -> Self {
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

impl From<v1::request::AuthenticatorRequestData> for v2::request::AuthenticatorRequestData {
    fn from(authenticator_request_data: v1::request::AuthenticatorRequestData) -> Self {
        match authenticator_request_data {
            v1::request::AuthenticatorRequestData::Initial(init_msg) => {
                v2::request::AuthenticatorRequestData::Initial(init_msg.into())
            }
            v1::request::AuthenticatorRequestData::Final(gw_client) => {
                v2::request::AuthenticatorRequestData::Final(gw_client.into())
            }
            v1::request::AuthenticatorRequestData::QueryBandwidth(pub_key) => {
                v2::request::AuthenticatorRequestData::QueryBandwidth(pub_key)
            }
        }
    }
}

impl From<v1::registration::InitMessage> for v2::registration::InitMessage {
    fn from(init_msg: v1::registration::InitMessage) -> Self {
        Self {
            pub_key: init_msg.pub_key,
        }
    }
}

impl From<v1::registration::GatewayClient> for Box<v2::registration::FinalMessage> {
    fn from(gw_client: v1::registration::GatewayClient) -> Self {
        Box::new(v2::registration::FinalMessage {
            gateway_client: gw_client.into(),
            credential: None,
        })
    }
}

impl From<v1::registration::GatewayClient> for v2::registration::GatewayClient {
    fn from(gw_client: v1::registration::GatewayClient) -> Self {
        Self {
            pub_key: gw_client.pub_key,
            private_ip: gw_client.private_ip,
            mac: gw_client.mac.into(),
        }
    }
}

impl From<v2::registration::GatewayClient> for v1::registration::GatewayClient {
    fn from(gw_client: v2::registration::GatewayClient) -> Self {
        Self {
            pub_key: gw_client.pub_key,
            private_ip: gw_client.private_ip,
            mac: gw_client.mac.into(),
        }
    }
}

impl From<v1::registration::ClientMac> for v2::registration::ClientMac {
    fn from(mac: v1::registration::ClientMac) -> Self {
        Self::new(mac.to_vec())
    }
}

impl From<v2::registration::ClientMac> for v1::registration::ClientMac {
    fn from(mac: v2::registration::ClientMac) -> Self {
        Self::new(mac.to_vec())
    }
}

impl From<v2::response::AuthenticatorResponse> for v1::response::AuthenticatorResponse {
    fn from(authenticator_response: v2::response::AuthenticatorResponse) -> Self {
        Self {
            version: authenticator_response.protocol.version,
            data: authenticator_response.data.into(),
            reply_to: authenticator_response.reply_to,
        }
    }
}

impl From<v2::response::AuthenticatorResponseData> for v1::response::AuthenticatorResponseData {
    fn from(authenticator_response_data: v2::response::AuthenticatorResponseData) -> Self {
        match authenticator_response_data {
            v2::response::AuthenticatorResponseData::PendingRegistration(
                pending_registration_response,
            ) => v1::response::AuthenticatorResponseData::PendingRegistration(
                pending_registration_response.into(),
            ),
            v2::response::AuthenticatorResponseData::Registered(registered_response) => {
                v1::response::AuthenticatorResponseData::Registered(registered_response.into())
            }
            v2::response::AuthenticatorResponseData::RemainingBandwidth(
                remaining_bandwidth_response,
            ) => v1::response::AuthenticatorResponseData::RemainingBandwidth(
                remaining_bandwidth_response.into(),
            ),
        }
    }
}

impl From<v2::response::PendingRegistrationResponse> for v1::response::PendingRegistrationResponse {
    fn from(value: v2::response::PendingRegistrationResponse) -> Self {
        Self {
            request_id: value.request_id,
            reply_to: value.reply_to,
            reply: value.reply.into(),
        }
    }
}

impl From<v2::response::RegisteredResponse> for v1::response::RegisteredResponse {
    fn from(value: v2::response::RegisteredResponse) -> Self {
        Self {
            request_id: value.request_id,
            reply_to: value.reply_to,
            reply: value.reply.into(),
        }
    }
}

impl From<v2::response::RemainingBandwidthResponse> for v1::response::RemainingBandwidthResponse {
    fn from(value: v2::response::RemainingBandwidthResponse) -> Self {
        Self {
            request_id: value.request_id,
            reply_to: value.reply_to,
            reply: value.reply.map(Into::into),
        }
    }
}

impl From<v2::registration::RegistrationData> for v1::registration::RegistrationData {
    fn from(value: v2::registration::RegistrationData) -> Self {
        Self {
            nonce: value.nonce,
            gateway_data: value.gateway_data.into(),
            wg_port: value.wg_port,
        }
    }
}

impl From<v2::registration::RegistredData> for v1::registration::RegistredData {
    fn from(value: v2::registration::RegistredData) -> Self {
        Self {
            pub_key: value.pub_key,
            private_ip: value.private_ip,
            wg_port: value.wg_port,
        }
    }
}

impl From<v2::registration::RemainingBandwidthData> for v1::registration::RemainingBandwidthData {
    fn from(value: v2::registration::RemainingBandwidthData) -> Self {
        Self {
            available_bandwidth: value.available_bandwidth,
            suspended: false,
        }
    }
}
