// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_service_provider_requests_common::{Protocol, ServiceProviderType};

use crate::{v4, v5};

impl From<v4::request::AuthenticatorRequest> for v5::request::AuthenticatorRequest {
    fn from(authenticator_request: v4::request::AuthenticatorRequest) -> Self {
        Self {
            protocol: Protocol {
                version: 5,
                service_provider_type: ServiceProviderType::Authenticator,
            },
            data: authenticator_request.data.into(),
            request_id: authenticator_request.request_id,
        }
    }
}

impl From<v4::request::AuthenticatorRequestData> for v5::request::AuthenticatorRequestData {
    fn from(authenticator_request_data: v4::request::AuthenticatorRequestData) -> Self {
        match authenticator_request_data {
            v4::request::AuthenticatorRequestData::Initial(init_msg) => {
                v5::request::AuthenticatorRequestData::Initial(init_msg.into())
            }
            v4::request::AuthenticatorRequestData::Final(final_msg) => {
                v5::request::AuthenticatorRequestData::Final(Box::new((*final_msg).into()))
            }
            v4::request::AuthenticatorRequestData::QueryBandwidth(pub_key) => {
                v5::request::AuthenticatorRequestData::QueryBandwidth(pub_key)
            }
            v4::request::AuthenticatorRequestData::TopUpBandwidth(top_up_message) => {
                v5::request::AuthenticatorRequestData::TopUpBandwidth(top_up_message.into())
            }
        }
    }
}

impl From<v4::registration::InitMessage> for v5::registration::InitMessage {
    fn from(init_msg: v4::registration::InitMessage) -> Self {
        Self {
            pub_key: init_msg.pub_key,
        }
    }
}

impl From<v4::registration::FinalMessage> for v5::registration::FinalMessage {
    fn from(final_msg: v4::registration::FinalMessage) -> Self {
        Self {
            gateway_client: final_msg.gateway_client.into(),
            credential: final_msg.credential,
        }
    }
}

impl From<v4::registration::GatewayClient> for v5::registration::GatewayClient {
    fn from(gateway_client: v4::registration::GatewayClient) -> Self {
        Self {
            pub_key: gateway_client.pub_key,
            private_ips: gateway_client.private_ips.into(),
            mac: gateway_client.mac.into(),
        }
    }
}

impl From<v5::registration::GatewayClient> for v4::registration::GatewayClient {
    fn from(gateway_client: v5::registration::GatewayClient) -> Self {
        Self {
            pub_key: gateway_client.pub_key,
            private_ips: gateway_client.private_ips.into(),
            mac: gateway_client.mac.into(),
        }
    }
}

impl From<v4::registration::ClientMac> for v5::registration::ClientMac {
    fn from(client_mac: v4::registration::ClientMac) -> Self {
        Self::new((*client_mac).clone())
    }
}

impl From<v5::registration::ClientMac> for v4::registration::ClientMac {
    fn from(client_mac: v5::registration::ClientMac) -> Self {
        Self::new((*client_mac).clone())
    }
}

impl From<Box<v4::topup::TopUpMessage>> for Box<v5::topup::TopUpMessage> {
    fn from(top_up_message: Box<v4::topup::TopUpMessage>) -> Self {
        Box::new(v5::topup::TopUpMessage {
            pub_key: top_up_message.pub_key,
            credential: top_up_message.credential,
        })
    }
}

impl From<v4::response::AuthenticatorResponse> for v5::response::AuthenticatorResponse {
    fn from(value: v4::response::AuthenticatorResponse) -> Self {
        Self {
            protocol: Protocol {
                version: 5,
                service_provider_type: value.protocol.service_provider_type,
            },
            data: value.data.into(),
        }
    }
}

impl From<v4::response::AuthenticatorResponseData> for v5::response::AuthenticatorResponseData {
    fn from(authenticator_response_data: v4::response::AuthenticatorResponseData) -> Self {
        match authenticator_response_data {
            v4::response::AuthenticatorResponseData::PendingRegistration(pending_response) => {
                v5::response::AuthenticatorResponseData::PendingRegistration(
                    pending_response.into(),
                )
            }
            v4::response::AuthenticatorResponseData::Registered(registered_response) => {
                v5::response::AuthenticatorResponseData::Registered(registered_response.into())
            }
            v4::response::AuthenticatorResponseData::RemainingBandwidth(
                remaining_bandwidth_response,
            ) => v5::response::AuthenticatorResponseData::RemainingBandwidth(
                remaining_bandwidth_response.into(),
            ),
            v4::response::AuthenticatorResponseData::TopUpBandwidth(top_up_response) => {
                v5::response::AuthenticatorResponseData::TopUpBandwidth(top_up_response.into())
            }
        }
    }
}

impl From<v4::response::RegisteredResponse> for v5::response::RegisteredResponse {
    fn from(value: v4::response::RegisteredResponse) -> Self {
        Self {
            request_id: value.request_id,
            reply: value.reply.into(),
        }
    }
}

impl From<v4::response::PendingRegistrationResponse> for v5::response::PendingRegistrationResponse {
    fn from(value: v4::response::PendingRegistrationResponse) -> Self {
        Self {
            request_id: value.request_id,
            reply: value.reply.into(),
        }
    }
}

impl From<v4::registration::RegistrationData> for v5::registration::RegistrationData {
    fn from(value: v4::registration::RegistrationData) -> Self {
        Self {
            nonce: value.nonce,
            gateway_data: value.gateway_data.into(),
            wg_port: value.wg_port,
        }
    }
}

impl From<v5::registration::RegistrationData> for v4::registration::RegistrationData {
    fn from(value: v5::registration::RegistrationData) -> Self {
        Self {
            nonce: value.nonce,
            gateway_data: value.gateway_data.into(),
            wg_port: value.wg_port,
        }
    }
}

impl From<v4::response::RemainingBandwidthResponse> for v5::response::RemainingBandwidthResponse {
    fn from(value: v4::response::RemainingBandwidthResponse) -> Self {
        Self {
            request_id: value.request_id,
            reply: value.reply.map(Into::into),
        }
    }
}

impl From<v4::response::TopUpBandwidthResponse> for v5::response::TopUpBandwidthResponse {
    fn from(value: v4::response::TopUpBandwidthResponse) -> Self {
        Self {
            request_id: value.request_id,
            reply: value.reply.into(),
        }
    }
}

impl From<v4::registration::RegistredData> for v5::registration::RegistredData {
    fn from(value: v4::registration::RegistredData) -> Self {
        Self {
            pub_key: value.pub_key,
            private_ips: value.private_ips.into(),
            wg_port: value.wg_port,
        }
    }
}

impl From<v4::registration::RemainingBandwidthData> for v5::registration::RemainingBandwidthData {
    fn from(value: v4::registration::RemainingBandwidthData) -> Self {
        Self {
            available_bandwidth: value.available_bandwidth,
        }
    }
}

impl From<v4::registration::IpPair> for v5::registration::IpPair {
    fn from(value: v4::registration::IpPair) -> Self {
        Self {
            ipv4: value.ipv4,
            ipv6: value.ipv6,
        }
    }
}

impl From<v5::registration::IpPair> for v4::registration::IpPair {
    fn from(value: v5::registration::IpPair) -> Self {
        Self {
            ipv4: value.ipv4,
            ipv6: value.ipv6,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        net::{Ipv4Addr, Ipv6Addr},
        str::FromStr,
    };

    use nym_credentials_interface::CredentialSpendingData;
    use nym_crypto::asymmetric::encryption::PrivateKey;
    use nym_sphinx::addressing::Recipient;
    use nym_wireguard_types::PeerPublicKey;
    use x25519_dalek::PublicKey;

    use super::*;
    use crate::{
        util::tests::{CREDENTIAL_BYTES, RECIPIENT},
        v4,
    };

    #[test]
    fn upgrade_initial_req() {
        let pub_key = PeerPublicKey::new(PublicKey::from([0; 32]));
        let reply_to = Recipient::try_from_base58_string(RECIPIENT).unwrap();

        let (msg, _) = v4::request::AuthenticatorRequest::new_initial_request(
            v4::registration::InitMessage::new(pub_key),
            reply_to,
        );
        let upgraded_msg = v5::request::AuthenticatorRequest::from(msg);

        assert_eq!(
            upgraded_msg.protocol,
            Protocol {
                version: 5,
                service_provider_type: ServiceProviderType::Authenticator
            }
        );
        assert_eq!(
            upgraded_msg.data,
            v5::request::AuthenticatorRequestData::Initial(v5::registration::InitMessage {
                pub_key
            })
        );
    }

    #[test]
    fn upgrade_final_req() {
        let mut rng = rand::thread_rng();

        let local_secret = PrivateKey::new(&mut rng);
        let remote_secret = x25519_dalek::StaticSecret::random_from_rng(&mut rng);
        let ipv4 = Ipv4Addr::from_str("10.10.10.10").unwrap();
        let ipv6 = Ipv6Addr::from_str("fc01::a0a").unwrap();
        let ips = v4::registration::IpPair::new(ipv4, ipv6);
        let nonce = 42;
        let gateway_client = v4::registration::GatewayClient::new(
            &local_secret,
            (&remote_secret).into(),
            ips,
            nonce,
        );
        let credential = Some(CredentialSpendingData::try_from_bytes(&CREDENTIAL_BYTES).unwrap());
        let final_message = v4::registration::FinalMessage {
            gateway_client: gateway_client.clone(),
            credential: credential.clone(),
        };
        let reply_to = Recipient::try_from_base58_string(RECIPIENT).unwrap();

        let (msg, _) =
            v4::request::AuthenticatorRequest::new_final_request(final_message, reply_to);
        let upgraded_msg = v5::request::AuthenticatorRequest::from(msg);

        assert_eq!(
            upgraded_msg.protocol,
            Protocol {
                version: 5,
                service_provider_type: ServiceProviderType::Authenticator
            }
        );
        assert_eq!(
            upgraded_msg.data,
            v5::request::AuthenticatorRequestData::Final(Box::new(
                v5::registration::FinalMessage {
                    gateway_client: v5::registration::GatewayClient::new(
                        &local_secret,
                        (&remote_secret).into(),
                        v5::registration::IpPair::new(ipv4, ipv6),
                        nonce
                    ),
                    credential
                }
            ))
        );
    }

    #[test]
    fn upgrade_query_req() {
        let pub_key = PeerPublicKey::new(PublicKey::from([0; 32]));
        let reply_to = Recipient::try_from_base58_string(RECIPIENT).unwrap();

        let (msg, _) = v4::request::AuthenticatorRequest::new_query_request(pub_key, reply_to);
        let upgraded_msg = v5::request::AuthenticatorRequest::from(msg);

        assert_eq!(
            upgraded_msg.protocol,
            Protocol {
                version: 5,
                service_provider_type: ServiceProviderType::Authenticator
            }
        );
        assert_eq!(
            upgraded_msg.data,
            v5::request::AuthenticatorRequestData::QueryBandwidth(pub_key)
        );
    }

    #[test]
    fn upgrade_pending_reg_resp() {
        let mut rng = rand::thread_rng();

        let local_secret = PrivateKey::new(&mut rng);
        let remote_secret = x25519_dalek::StaticSecret::random_from_rng(&mut rng);
        let ipv4 = Ipv4Addr::from_str("10.10.10.10").unwrap();
        let ipv6 = Ipv6Addr::from_str("fc01::a0a").unwrap();
        let ips = v4::registration::IpPair::new(ipv4, ipv6);
        let nonce = 42;
        let wg_port = 51822;
        let gateway_data = v4::registration::GatewayClient::new(
            &local_secret,
            (&remote_secret).into(),
            ips,
            nonce,
        );
        let registration_data = v4::registration::RegistrationData {
            nonce,
            gateway_data,
            wg_port,
        };
        let request_id = 123;
        let reply_to = Recipient::try_from_base58_string(RECIPIENT).unwrap();

        let msg = v4::response::AuthenticatorResponse::new_pending_registration_success(
            registration_data,
            request_id,
            reply_to,
        );
        let upgraded_msg = v5::response::AuthenticatorResponse::from(msg);

        assert_eq!(
            upgraded_msg.protocol,
            Protocol {
                version: 5,
                service_provider_type: ServiceProviderType::Authenticator
            }
        );

        assert_eq!(
            upgraded_msg.data,
            v5::response::AuthenticatorResponseData::PendingRegistration(
                v5::response::PendingRegistrationResponse {
                    request_id,
                    reply: v5::registration::RegistrationData {
                        nonce,
                        gateway_data: v5::registration::GatewayClient::new(
                            &local_secret,
                            (&remote_secret).into(),
                            v5::registration::IpPair::new(ipv4, ipv6),
                            nonce
                        ),
                        wg_port
                    }
                }
            )
        );
    }

    #[test]
    fn upgrade_registered_resp() {
        let pub_key = PeerPublicKey::new(PublicKey::from([0; 32]));
        let ipv4 = Ipv4Addr::from_str("10.1.10.10").unwrap();
        let ipv6 = Ipv6Addr::from_str("fc01::a0a").unwrap();
        let private_ips = v4::registration::IpPair::new(ipv4, ipv6);
        let wg_port = 51822;
        let registred_data = v4::registration::RegistredData {
            pub_key,
            private_ips,
            wg_port,
        };
        let request_id = 123;
        let reply_to = Recipient::try_from_base58_string(RECIPIENT).unwrap();

        let msg = v4::response::AuthenticatorResponse::new_registered(
            registred_data,
            reply_to,
            request_id,
        );
        let upgraded_msg = v5::response::AuthenticatorResponse::from(msg);

        assert_eq!(
            upgraded_msg.protocol,
            Protocol {
                version: 5,
                service_provider_type: ServiceProviderType::Authenticator
            }
        );
        assert_eq!(
            upgraded_msg.data,
            v5::response::AuthenticatorResponseData::Registered(v5::response::RegisteredResponse {
                request_id,
                reply: v5::registration::RegistredData {
                    wg_port,
                    pub_key,
                    private_ips: v5::registration::IpPair::new(ipv4, ipv6)
                }
            })
        );
    }

    #[test]
    fn upgrade_remaining_bandwidth_resp() {
        let available_bandwidth = 42;
        let remaining_bandwidth_data = Some(v4::registration::RemainingBandwidthData {
            available_bandwidth,
        });
        let request_id = 123;
        let reply_to = Recipient::try_from_base58_string(RECIPIENT).unwrap();

        let msg = v4::response::AuthenticatorResponse::new_remaining_bandwidth(
            remaining_bandwidth_data,
            reply_to,
            request_id,
        );
        let upgraded_msg = v5::response::AuthenticatorResponse::from(msg);

        assert_eq!(
            upgraded_msg.protocol,
            Protocol {
                version: 5,
                service_provider_type: ServiceProviderType::Authenticator
            }
        );
        assert_eq!(
            upgraded_msg.data,
            v5::response::AuthenticatorResponseData::RemainingBandwidth(
                v5::response::RemainingBandwidthResponse {
                    request_id,
                    reply: Some(v5::registration::RemainingBandwidthData {
                        available_bandwidth,
                    })
                }
            )
        );
    }
}
