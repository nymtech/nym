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

impl From<v1::registration::ClientMac> for v2::registration::ClientMac {
    fn from(mac: v1::registration::ClientMac) -> Self {
        Self::new(mac.to_vec())
    }
}
