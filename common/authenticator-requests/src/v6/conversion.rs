// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{v5, v6};

impl TryFrom<v5::request::AuthenticatorRequest> for v6::request::AuthenticatorRequest {
    type Error = crate::Error;

    fn try_from(
        authenticator_request: v5::request::AuthenticatorRequest,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            protocol: v6::PROTOCOL,
            data: authenticator_request.data.try_into()?,
            request_id: authenticator_request.request_id,
        })
    }
}

impl TryFrom<v5::request::AuthenticatorRequestData> for v6::request::AuthenticatorRequestData {
    type Error = crate::Error;

    fn try_from(
        authenticator_request_data: v5::request::AuthenticatorRequestData,
    ) -> Result<Self, Self::Error> {
        match authenticator_request_data {
            v5::request::AuthenticatorRequestData::Initial(init_msg) => Ok(
                v6::request::AuthenticatorRequestData::Initial(init_msg.into()),
            ),
            v5::request::AuthenticatorRequestData::Final(final_msg) => Ok(
                v6::request::AuthenticatorRequestData::Final(Box::new((*final_msg).try_into()?)),
            ),
            v5::request::AuthenticatorRequestData::QueryBandwidth(pub_key) => Ok(
                v6::request::AuthenticatorRequestData::QueryBandwidth(pub_key),
            ),
            v5::request::AuthenticatorRequestData::TopUpBandwidth(top_up_message) => Ok(
                v6::request::AuthenticatorRequestData::TopUpBandwidth(top_up_message.into()),
            ),
        }
    }
}

impl From<v5::registration::InitMessage> for v6::registration::InitMessage {
    fn from(init_msg: v5::registration::InitMessage) -> Self {
        Self {
            pub_key: init_msg.pub_key,
        }
    }
}

impl TryFrom<v5::registration::FinalMessage> for v6::registration::FinalMessage {
    type Error = crate::Error;

    fn try_from(final_msg: v5::registration::FinalMessage) -> Result<Self, Self::Error> {
        Ok(Self {
            gateway_client: final_msg.gateway_client.into(),
            credential: final_msg
                .credential
                .map(TryInto::try_into)
                .transpose()
                .map_err(Self::Error::conversion_display)?,
        })
    }
}

impl From<v5::registration::GatewayClient> for v6::registration::GatewayClient {
    fn from(gateway_client: v5::registration::GatewayClient) -> Self {
        Self {
            pub_key: gateway_client.pub_key,
            private_ips: gateway_client.private_ips.into(),
            mac: gateway_client.mac.into(),
        }
    }
}

impl From<v6::registration::GatewayClient> for v5::registration::GatewayClient {
    fn from(gateway_client: v6::registration::GatewayClient) -> Self {
        Self {
            pub_key: gateway_client.pub_key,
            private_ips: gateway_client.private_ips.into(),
            mac: gateway_client.mac.into(),
        }
    }
}

impl From<v5::registration::ClientMac> for v6::registration::ClientMac {
    fn from(client_mac: v5::registration::ClientMac) -> Self {
        Self::new((*client_mac).clone())
    }
}

impl From<v6::registration::ClientMac> for v5::registration::ClientMac {
    fn from(client_mac: v6::registration::ClientMac) -> Self {
        Self::new((*client_mac).clone())
    }
}

impl From<Box<v5::topup::TopUpMessage>> for Box<v6::topup::TopUpMessage> {
    fn from(top_up_message: Box<v5::topup::TopUpMessage>) -> Self {
        Box::new(v6::topup::TopUpMessage {
            pub_key: top_up_message.pub_key,
            credential: top_up_message.credential,
        })
    }
}

impl From<v5::response::AuthenticatorResponse> for v6::response::AuthenticatorResponse {
    fn from(value: v5::response::AuthenticatorResponse) -> Self {
        Self {
            protocol: v6::PROTOCOL,
            data: value.data.into(),
        }
    }
}

impl From<v5::response::AuthenticatorResponseData> for v6::response::AuthenticatorResponseData {
    fn from(authenticator_response_data: v5::response::AuthenticatorResponseData) -> Self {
        match authenticator_response_data {
            v5::response::AuthenticatorResponseData::PendingRegistration(pending_response) => {
                v6::response::AuthenticatorResponseData::PendingRegistration(
                    pending_response.into(),
                )
            }
            v5::response::AuthenticatorResponseData::Registered(registered_response) => {
                v6::response::AuthenticatorResponseData::Registered(registered_response.into())
            }
            v5::response::AuthenticatorResponseData::RemainingBandwidth(
                remaining_bandwidth_response,
            ) => v6::response::AuthenticatorResponseData::RemainingBandwidth(
                remaining_bandwidth_response.into(),
            ),
            v5::response::AuthenticatorResponseData::TopUpBandwidth(top_up_response) => {
                v6::response::AuthenticatorResponseData::TopUpBandwidth(top_up_response.into())
            }
        }
    }
}

impl From<v5::response::RegisteredResponse> for v6::response::RegisteredResponse {
    fn from(value: v5::response::RegisteredResponse) -> Self {
        Self {
            request_id: value.request_id,
            reply: value.reply.into(),
            upgrade_mode_enabled: false,
        }
    }
}

impl From<v5::response::PendingRegistrationResponse> for v6::response::PendingRegistrationResponse {
    fn from(value: v5::response::PendingRegistrationResponse) -> Self {
        Self {
            request_id: value.request_id,
            reply: value.reply.into(),
            upgrade_mode_enabled: false,
        }
    }
}

impl From<v5::registration::RegistrationData> for v6::registration::RegistrationData {
    fn from(value: v5::registration::RegistrationData) -> Self {
        Self {
            nonce: value.nonce,
            gateway_data: value.gateway_data.into(),
            wg_port: value.wg_port,
        }
    }
}

impl From<v6::registration::RegistrationData> for v5::registration::RegistrationData {
    fn from(value: v6::registration::RegistrationData) -> Self {
        Self {
            nonce: value.nonce,
            gateway_data: value.gateway_data.into(),
            wg_port: value.wg_port,
        }
    }
}

impl From<v5::response::RemainingBandwidthResponse> for v6::response::RemainingBandwidthResponse {
    fn from(value: v5::response::RemainingBandwidthResponse) -> Self {
        Self {
            request_id: value.request_id,
            reply: value.reply.map(Into::into),
            upgrade_mode_enabled: false,
        }
    }
}

impl From<v5::response::TopUpBandwidthResponse> for v6::response::TopUpBandwidthResponse {
    fn from(value: v5::response::TopUpBandwidthResponse) -> Self {
        Self {
            request_id: value.request_id,
            reply: value.reply.into(),
            upgrade_mode_enabled: false,
        }
    }
}

impl From<v5::registration::RegisteredData> for v6::registration::RegisteredData {
    fn from(value: v5::registration::RegisteredData) -> Self {
        Self {
            pub_key: value.pub_key,
            private_ips: value.private_ips.into(),
            wg_port: value.wg_port,
        }
    }
}

impl From<v5::registration::RemainingBandwidthData> for v6::registration::RemainingBandwidthData {
    fn from(value: v5::registration::RemainingBandwidthData) -> Self {
        Self {
            available_bandwidth: value.available_bandwidth,
        }
    }
}

impl From<v5::registration::IpPair> for v6::registration::IpPair {
    fn from(value: v5::registration::IpPair) -> Self {
        Self {
            ipv4: value.ipv4,
            ipv6: value.ipv6,
        }
    }
}

impl From<v6::registration::IpPair> for v5::registration::IpPair {
    fn from(value: v6::registration::IpPair) -> Self {
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

    use nym_credentials_interface::{BandwidthCredential, CredentialSpendingData, TicketType};
    use nym_crypto::asymmetric::x25519::PrivateKey;
    use nym_wireguard_types::PeerPublicKey;
    use x25519_dalek::PublicKey;

    use super::*;
    use crate::models::BandwidthClaim;
    use crate::{util::tests::CREDENTIAL_BYTES, v5};

    #[test]
    fn upgrade_initial_req() {
        let pub_key = PeerPublicKey::new(PublicKey::from([0; 32]));

        let (msg, _) = v5::request::AuthenticatorRequest::new_initial_request(
            v5::registration::InitMessage::new(pub_key),
        );
        let upgraded_msg = v6::request::AuthenticatorRequest::try_from(msg).unwrap();

        assert_eq!(upgraded_msg.protocol, v6::PROTOCOL);
        assert_eq!(
            upgraded_msg.data,
            v6::request::AuthenticatorRequestData::Initial(v6::registration::InitMessage {
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
        let ips = v5::registration::IpPair::new(ipv4, ipv6);
        let nonce = 42;
        let gateway_client = v5::registration::GatewayClient::new(
            &local_secret,
            (&remote_secret).into(),
            ips,
            nonce,
        );
        let credential = CredentialSpendingData::try_from_bytes(&CREDENTIAL_BYTES).unwrap();
        let final_message = v5::registration::FinalMessage {
            gateway_client: gateway_client.clone(),
            credential: Some(credential.clone()),
        };

        let (msg, _) = v5::request::AuthenticatorRequest::new_final_request(final_message);
        let upgraded_msg = v6::request::AuthenticatorRequest::try_from(msg).unwrap();

        assert_eq!(upgraded_msg.protocol, v6::PROTOCOL);
        assert_eq!(
            upgraded_msg.data,
            v6::request::AuthenticatorRequestData::Final(Box::new(
                v6::registration::FinalMessage {
                    gateway_client: v6::registration::GatewayClient::new(
                        &local_secret,
                        (&remote_secret).into(),
                        v6::registration::IpPair::new(ipv4, ipv6),
                        nonce
                    ),
                    credential: Some(BandwidthClaim {
                        credential: BandwidthCredential::ZkNym(Box::new(credential)),
                        kind: TicketType::V1MixnetEntry,
                    })
                }
            ))
        );
    }

    #[test]
    fn upgrade_query_req() {
        let pub_key = PeerPublicKey::new(PublicKey::from([0; 32]));

        let (msg, _) = v5::request::AuthenticatorRequest::new_query_request(pub_key);
        let upgraded_msg = v6::request::AuthenticatorRequest::try_from(msg).unwrap();

        assert_eq!(upgraded_msg.protocol, v6::PROTOCOL);
        assert_eq!(
            upgraded_msg.data,
            v6::request::AuthenticatorRequestData::QueryBandwidth(pub_key)
        );
    }

    #[test]
    fn upgrade_pending_reg_resp() {
        let mut rng = rand::thread_rng();

        let local_secret = PrivateKey::new(&mut rng);
        let remote_secret = x25519_dalek::StaticSecret::random_from_rng(&mut rng);
        let ipv4 = Ipv4Addr::from_str("10.10.10.10").unwrap();
        let ipv6 = Ipv6Addr::from_str("fc01::a0a").unwrap();
        let ips = v5::registration::IpPair::new(ipv4, ipv6);
        let nonce = 42;
        let wg_port = 51822;
        let gateway_data = v5::registration::GatewayClient::new(
            &local_secret,
            (&remote_secret).into(),
            ips,
            nonce,
        );
        let registration_data = v5::registration::RegistrationData {
            nonce,
            gateway_data,
            wg_port,
        };
        let request_id = 123;

        let msg = v5::response::AuthenticatorResponse::new_pending_registration_success(
            registration_data,
            request_id,
        );
        let upgraded_msg = v6::response::AuthenticatorResponse::from(msg);

        assert_eq!(upgraded_msg.protocol, v6::PROTOCOL);

        assert_eq!(
            upgraded_msg.data,
            v6::response::AuthenticatorResponseData::PendingRegistration(
                v6::response::PendingRegistrationResponse {
                    request_id,
                    reply: v6::registration::RegistrationData {
                        nonce,
                        gateway_data: v6::registration::GatewayClient::new(
                            &local_secret,
                            (&remote_secret).into(),
                            v6::registration::IpPair::new(ipv4, ipv6),
                            nonce
                        ),
                        wg_port
                    },
                    upgrade_mode_enabled: false,
                }
            )
        );
    }

    #[test]
    fn upgrade_registered_resp() {
        let pub_key = PeerPublicKey::new(PublicKey::from([0; 32]));
        let ipv4 = Ipv4Addr::from_str("10.1.10.10").unwrap();
        let ipv6 = Ipv6Addr::from_str("fc01::a0a").unwrap();
        let private_ips = v5::registration::IpPair::new(ipv4, ipv6);
        let wg_port = 51822;
        let registered_data = v5::registration::RegisteredData {
            pub_key,
            private_ips,
            wg_port,
        };
        let request_id = 123;

        let msg = v5::response::AuthenticatorResponse::new_registered(registered_data, request_id);
        let upgraded_msg = v6::response::AuthenticatorResponse::from(msg);

        assert_eq!(upgraded_msg.protocol, v6::PROTOCOL);
        assert_eq!(
            upgraded_msg.data,
            v6::response::AuthenticatorResponseData::Registered(v6::response::RegisteredResponse {
                request_id,
                reply: v6::registration::RegisteredData {
                    wg_port,
                    pub_key,
                    private_ips: v6::registration::IpPair::new(ipv4, ipv6)
                },
                upgrade_mode_enabled: false,
            })
        );
    }

    #[test]
    fn upgrade_remaining_bandwidth_resp() {
        let available_bandwidth = 42;
        let remaining_bandwidth_data = Some(v5::registration::RemainingBandwidthData {
            available_bandwidth,
        });
        let request_id = 123;

        let msg = v5::response::AuthenticatorResponse::new_remaining_bandwidth(
            remaining_bandwidth_data,
            request_id,
        );
        let upgraded_msg = v6::response::AuthenticatorResponse::from(msg);

        assert_eq!(upgraded_msg.protocol, v6::PROTOCOL);
        assert_eq!(
            upgraded_msg.data,
            v6::response::AuthenticatorResponseData::RemainingBandwidth(
                v6::response::RemainingBandwidthResponse {
                    request_id,
                    reply: Some(v6::registration::RemainingBandwidthData {
                        available_bandwidth,
                    }),
                    upgrade_mode_enabled: false,
                }
            )
        );
    }
}
