// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_service_provider_requests_common::{Protocol, ServiceProviderType};

use crate::{v3, v4};

impl TryFrom<v3::request::AuthenticatorRequest> for v4::request::AuthenticatorRequest {
    type Error = crate::Error;
    fn try_from(
        authenticator_request: v3::request::AuthenticatorRequest,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            protocol: Protocol {
                version: 4,
                service_provider_type: ServiceProviderType::Authenticator,
            },
            data: authenticator_request.data.try_into()?,
            reply_to: authenticator_request.reply_to,
            request_id: authenticator_request.request_id,
        })
    }
}

impl TryFrom<v4::request::AuthenticatorRequest> for v3::request::AuthenticatorRequest {
    type Error = crate::Error;
    fn try_from(
        authenticator_request: v4::request::AuthenticatorRequest,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            protocol: Protocol {
                version: 3,
                service_provider_type: ServiceProviderType::Authenticator,
            },
            data: authenticator_request.data.try_into()?,
            reply_to: authenticator_request.reply_to,
            request_id: authenticator_request.request_id,
        })
    }
}

impl TryFrom<v3::request::AuthenticatorRequestData> for v4::request::AuthenticatorRequestData {
    type Error = crate::Error;
    fn try_from(
        authenticator_request_data: v3::request::AuthenticatorRequestData,
    ) -> Result<Self, Self::Error> {
        match authenticator_request_data {
            v3::request::AuthenticatorRequestData::Initial(init_msg) => Ok(
                v4::request::AuthenticatorRequestData::Initial(init_msg.into()),
            ),
            v3::request::AuthenticatorRequestData::Final(_) => Err(Self::Error::Conversion(
                "mac hash breaking change".to_string(),
            )),
            v3::request::AuthenticatorRequestData::QueryBandwidth(pub_key) => Ok(
                v4::request::AuthenticatorRequestData::QueryBandwidth(pub_key),
            ),
            v3::request::AuthenticatorRequestData::TopUpBandwidth(top_up_message) => Ok(
                v4::request::AuthenticatorRequestData::TopUpBandwidth(top_up_message.into()),
            ),
        }
    }
}

impl TryFrom<v4::request::AuthenticatorRequestData> for v3::request::AuthenticatorRequestData {
    type Error = crate::Error;
    fn try_from(
        authenticator_request_data: v4::request::AuthenticatorRequestData,
    ) -> Result<Self, Self::Error> {
        match authenticator_request_data {
            v4::request::AuthenticatorRequestData::Initial(init_msg) => Ok(
                v3::request::AuthenticatorRequestData::Initial(init_msg.into()),
            ),
            v4::request::AuthenticatorRequestData::Final(_) => Err(Self::Error::Conversion(
                "mac hash breaking change".to_string(),
            )),
            v4::request::AuthenticatorRequestData::QueryBandwidth(pub_key) => Ok(
                v3::request::AuthenticatorRequestData::QueryBandwidth(pub_key),
            ),
            v4::request::AuthenticatorRequestData::TopUpBandwidth(top_up_message) => Ok(
                v3::request::AuthenticatorRequestData::TopUpBandwidth(top_up_message.into()),
            ),
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

impl From<v4::registration::InitMessage> for v3::registration::InitMessage {
    fn from(init_msg: v4::registration::InitMessage) -> Self {
        Self {
            pub_key: init_msg.pub_key,
        }
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

impl From<Box<v4::topup::TopUpMessage>> for Box<v3::topup::TopUpMessage> {
    fn from(top_up_message: Box<v4::topup::TopUpMessage>) -> Self {
        Box::new(v3::topup::TopUpMessage {
            pub_key: top_up_message.pub_key,
            credential: top_up_message.credential,
        })
    }
}

impl TryFrom<v3::response::AuthenticatorResponse> for v4::response::AuthenticatorResponse {
    type Error = crate::Error;
    fn try_from(value: v3::response::AuthenticatorResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            protocol: Protocol {
                version: 4,
                service_provider_type: value.protocol.service_provider_type,
            },
            data: value.data.try_into()?,
            reply_to: value.reply_to,
        })
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
            protocol: Protocol {
                version: 3,
                service_provider_type: authenticator_response.protocol.service_provider_type,
            },
        })
    }
}

impl TryFrom<v3::response::AuthenticatorResponseData> for v4::response::AuthenticatorResponseData {
    type Error = crate::Error;
    fn try_from(
        authenticator_response_data: v3::response::AuthenticatorResponseData,
    ) -> Result<Self, Self::Error> {
        match authenticator_response_data {
            v3::response::AuthenticatorResponseData::PendingRegistration(_) => Err(
                Self::Error::Conversion("mac hash breaking change".to_string()),
            ),

            v3::response::AuthenticatorResponseData::Registered(registered_response) => Ok(
                v4::response::AuthenticatorResponseData::Registered(registered_response.into()),
            ),

            v3::response::AuthenticatorResponseData::RemainingBandwidth(
                remaining_bandwidth_response,
            ) => Ok(v4::response::AuthenticatorResponseData::RemainingBandwidth(
                remaining_bandwidth_response.into(),
            )),
            v3::response::AuthenticatorResponseData::TopUpBandwidth(top_up_response) => Ok(
                v4::response::AuthenticatorResponseData::TopUpBandwidth(top_up_response.into()),
            ),
        }
    }
}

impl TryFrom<v4::response::AuthenticatorResponseData> for v3::response::AuthenticatorResponseData {
    type Error = crate::Error;

    fn try_from(
        authenticator_response_data: v4::response::AuthenticatorResponseData,
    ) -> Result<Self, Self::Error> {
        match authenticator_response_data {
            v4::response::AuthenticatorResponseData::PendingRegistration(_) => Err(
                Self::Error::Conversion("mac hash breaking change".to_string()),
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

impl From<v4::response::RegisteredResponse> for v3::response::RegisteredResponse {
    fn from(value: v4::response::RegisteredResponse) -> Self {
        Self {
            request_id: value.request_id,
            reply_to: value.reply_to,
            reply: value.reply.into(),
        }
    }
}

impl From<v3::response::RegisteredResponse> for v4::response::RegisteredResponse {
    fn from(value: v3::response::RegisteredResponse) -> Self {
        Self {
            request_id: value.request_id,
            reply_to: value.reply_to,
            reply: value.reply.into(),
        }
    }
}

impl From<v3::response::RemainingBandwidthResponse> for v4::response::RemainingBandwidthResponse {
    fn from(value: v3::response::RemainingBandwidthResponse) -> Self {
        Self {
            request_id: value.request_id,
            reply_to: value.reply_to,
            reply: value.reply.map(Into::into),
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

impl From<v3::response::TopUpBandwidthResponse> for v4::response::TopUpBandwidthResponse {
    fn from(value: v3::response::TopUpBandwidthResponse) -> Self {
        Self {
            request_id: value.request_id,
            reply_to: value.reply_to,
            reply: value.reply.into(),
        }
    }
}

impl From<v4::response::TopUpBandwidthResponse> for v3::response::TopUpBandwidthResponse {
    fn from(value: v4::response::TopUpBandwidthResponse) -> Self {
        Self {
            request_id: value.request_id,
            reply_to: value.reply_to,
            reply: value.reply.into(),
        }
    }
}

impl From<v3::registration::RegistredData> for v4::registration::RegistredData {
    fn from(value: v3::registration::RegistredData) -> Self {
        Self {
            pub_key: value.pub_key,
            private_ips: value.private_ip.into(),
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

impl From<v3::registration::RemainingBandwidthData> for v4::registration::RemainingBandwidthData {
    fn from(value: v3::registration::RemainingBandwidthData) -> Self {
        Self {
            available_bandwidth: value.available_bandwidth,
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

#[cfg(test)]
mod tests {
    use std::{
        net::{Ipv4Addr, Ipv6Addr},
        str::FromStr,
    };

    use nym_credentials_interface::CredentialSpendingData;
    use nym_crypto::asymmetric::x25519::PrivateKey;
    use nym_sphinx::addressing::Recipient;
    use nym_wireguard_types::PeerPublicKey;
    use x25519_dalek::PublicKey;

    use super::*;
    use crate::util::tests::{CREDENTIAL_BYTES, RECIPIENT};

    #[test]
    fn upgrade_initial_req() {
        let pub_key = PeerPublicKey::new(PublicKey::from([0; 32]));
        let reply_to = Recipient::try_from_base58_string(RECIPIENT).unwrap();

        let (msg, _) = v3::request::AuthenticatorRequest::new_initial_request(
            v3::registration::InitMessage::new(pub_key),
            reply_to,
        );
        let upgraded_msg = v4::request::AuthenticatorRequest::try_from(msg).unwrap();

        assert_eq!(
            upgraded_msg.protocol,
            Protocol {
                version: 4,
                service_provider_type: ServiceProviderType::Authenticator
            }
        );
        assert_eq!(
            upgraded_msg.data,
            v4::request::AuthenticatorRequestData::Initial(v4::registration::InitMessage {
                pub_key
            })
        );
    }

    #[test]
    fn downgrade_initial_req() {
        let pub_key = PeerPublicKey::new(PublicKey::from([0; 32]));
        let reply_to = Recipient::try_from_base58_string(RECIPIENT).unwrap();

        let (msg, _) = v4::request::AuthenticatorRequest::new_initial_request(
            v4::registration::InitMessage::new(pub_key),
            reply_to,
        );
        let downgraded_msg = v3::request::AuthenticatorRequest::try_from(msg).unwrap();

        assert_eq!(
            downgraded_msg.protocol,
            Protocol {
                version: 3,
                service_provider_type: ServiceProviderType::Authenticator
            }
        );
        assert_eq!(
            downgraded_msg.data,
            v3::request::AuthenticatorRequestData::Initial(v3::registration::InitMessage {
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
        let nonce = 42;
        let gateway_client = v3::registration::GatewayClient::new(
            &local_secret,
            (&remote_secret).into(),
            ipv4.into(),
            nonce,
        );
        let credential = Some(CredentialSpendingData::try_from_bytes(&CREDENTIAL_BYTES).unwrap());
        let final_message = v3::registration::FinalMessage {
            gateway_client,
            credential: credential.clone(),
        };
        let reply_to = Recipient::try_from_base58_string(RECIPIENT).unwrap();

        let (msg, _) =
            v3::request::AuthenticatorRequest::new_final_request(final_message, reply_to);
        assert!(v4::request::AuthenticatorRequest::try_from(msg).is_err());
    }

    #[test]
    fn downgrade_final_req() {
        let mut rng = rand::thread_rng();

        let local_secret = PrivateKey::new(&mut rng);
        let remote_secret = x25519_dalek::StaticSecret::random_from_rng(&mut rng);
        let ipv4 = Ipv4Addr::from_str("10.10.10.10").unwrap();
        let private_ips =
            v4::registration::IpPair::new(ipv4, Ipv6Addr::from_str("fc01::10").unwrap());
        let nonce = 42;
        let gateway_client = v4::registration::GatewayClient::new(
            &local_secret,
            (&remote_secret).into(),
            private_ips,
            nonce,
        );
        let credential = Some(CredentialSpendingData::try_from_bytes(&CREDENTIAL_BYTES).unwrap());
        let final_message = v4::registration::FinalMessage {
            gateway_client,
            credential: credential.clone(),
        };
        let reply_to = Recipient::try_from_base58_string(RECIPIENT).unwrap();

        let (msg, _) =
            v4::request::AuthenticatorRequest::new_final_request(final_message, reply_to);
        assert!(v3::request::AuthenticatorRequest::try_from(msg).is_err());
    }

    #[test]
    fn upgrade_query_req() {
        let pub_key = PeerPublicKey::new(PublicKey::from([0; 32]));
        let reply_to = Recipient::try_from_base58_string(RECIPIENT).unwrap();

        let (msg, _) = v3::request::AuthenticatorRequest::new_query_request(pub_key, reply_to);
        let upgraded_msg = v4::request::AuthenticatorRequest::try_from(msg).unwrap();

        assert_eq!(
            upgraded_msg.protocol,
            Protocol {
                version: 4,
                service_provider_type: ServiceProviderType::Authenticator
            }
        );
        assert_eq!(
            upgraded_msg.data,
            v4::request::AuthenticatorRequestData::QueryBandwidth(pub_key)
        );
    }

    #[test]
    fn downgrade_query_req() {
        let pub_key = PeerPublicKey::new(PublicKey::from([0; 32]));
        let reply_to = Recipient::try_from_base58_string(RECIPIENT).unwrap();

        let (msg, _) = v4::request::AuthenticatorRequest::new_query_request(pub_key, reply_to);
        let downgraded_msg = v3::request::AuthenticatorRequest::try_from(msg).unwrap();

        assert_eq!(
            downgraded_msg.protocol,
            Protocol {
                version: 3,
                service_provider_type: ServiceProviderType::Authenticator
            }
        );
        assert_eq!(
            downgraded_msg.data,
            v3::request::AuthenticatorRequestData::QueryBandwidth(pub_key)
        );
    }

    #[test]
    fn downgrade_topup_req() {
        let pub_key = PeerPublicKey::new(PublicKey::from([0; 32]));
        let credential = CredentialSpendingData::try_from_bytes(&CREDENTIAL_BYTES).unwrap();
        let top_up_message = v4::topup::TopUpMessage {
            pub_key,
            credential: credential.clone(),
        };
        let reply_to = Recipient::try_from_base58_string(RECIPIENT).unwrap();

        let (msg, _) =
            v4::request::AuthenticatorRequest::new_topup_request(top_up_message, reply_to);
        let downgraded_msg = v3::request::AuthenticatorRequest::try_from(msg).unwrap();

        assert_eq!(
            downgraded_msg.protocol,
            Protocol {
                version: 3,
                service_provider_type: ServiceProviderType::Authenticator
            }
        );
        assert_eq!(
            downgraded_msg.data,
            v3::request::AuthenticatorRequestData::TopUpBandwidth(Box::new(
                v3::topup::TopUpMessage {
                    pub_key,
                    credential
                }
            ))
        );
    }

    #[test]
    fn upgrade_pending_reg_resp() {
        let mut rng = rand::thread_rng();

        let local_secret = PrivateKey::new(&mut rng);
        let remote_secret = x25519_dalek::StaticSecret::random_from_rng(&mut rng);
        let ipv4 = Ipv4Addr::from_str("10.10.10.10").unwrap();
        let nonce = 42;
        let wg_port = 51822;
        let gateway_data = v3::registration::GatewayClient::new(
            &local_secret,
            (&remote_secret).into(),
            ipv4.into(),
            nonce,
        );
        let registration_data = v3::registration::RegistrationData {
            nonce,
            gateway_data,
            wg_port,
        };
        let request_id = 123;
        let reply_to = Recipient::try_from_base58_string(RECIPIENT).unwrap();

        let msg = v3::response::AuthenticatorResponse::new_pending_registration_success(
            registration_data,
            request_id,
            reply_to,
        );
        assert!(v4::response::AuthenticatorResponse::try_from(msg).is_err());
    }

    #[test]
    fn downgrade_pending_reg_resp() {
        let mut rng = rand::thread_rng();

        let local_secret = PrivateKey::new(&mut rng);
        let remote_secret = x25519_dalek::StaticSecret::random_from_rng(&mut rng);
        let ipv4 = Ipv4Addr::from_str("10.10.10.10").unwrap();
        let private_ips =
            v4::registration::IpPair::new(ipv4, Ipv6Addr::from_str("fc01::10").unwrap());
        let nonce = 42;
        let wg_port = 51822;
        let gateway_data = v4::registration::GatewayClient::new(
            &local_secret,
            (&remote_secret).into(),
            private_ips,
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
        assert!(v3::response::AuthenticatorResponse::try_from(msg).is_err());
    }

    #[test]
    fn upgrade_registered_resp() {
        let pub_key = PeerPublicKey::new(PublicKey::from([0; 32]));
        let ipv4 = Ipv4Addr::from_str("10.1.10.10").unwrap();
        let private_ips =
            v4::registration::IpPair::new(ipv4, Ipv6Addr::from_str("fc01::a0a").unwrap());
        let wg_port = 51822;
        let registred_data = v3::registration::RegistredData {
            pub_key,
            private_ip: ipv4.into(),
            wg_port,
        };
        let request_id = 123;
        let reply_to = Recipient::try_from_base58_string(RECIPIENT).unwrap();

        let msg = v3::response::AuthenticatorResponse::new_registered(
            registred_data,
            reply_to,
            request_id,
        );
        let upgraded_msg = v4::response::AuthenticatorResponse::try_from(msg).unwrap();

        assert_eq!(
            upgraded_msg.protocol,
            Protocol {
                version: 4,
                service_provider_type: ServiceProviderType::Authenticator
            }
        );
        assert_eq!(
            upgraded_msg.data,
            v4::response::AuthenticatorResponseData::Registered(v4::response::RegisteredResponse {
                request_id,
                reply_to,
                reply: v4::registration::RegistredData {
                    wg_port,
                    pub_key,
                    private_ips
                }
            })
        );
    }

    #[test]
    fn downgrade_registered_resp() {
        let pub_key = PeerPublicKey::new(PublicKey::from([0; 32]));
        let ipv4 = Ipv4Addr::from_str("10.10.10.10").unwrap();
        let private_ips =
            v4::registration::IpPair::new(ipv4, Ipv6Addr::from_str("fc01::10").unwrap());
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
        let downgraded_msg = v3::response::AuthenticatorResponse::try_from(msg).unwrap();

        assert_eq!(
            downgraded_msg.protocol,
            Protocol {
                version: 3,
                service_provider_type: ServiceProviderType::Authenticator
            }
        );
        assert_eq!(
            downgraded_msg.data,
            v3::response::AuthenticatorResponseData::Registered(v3::response::RegisteredResponse {
                request_id,
                reply_to,
                reply: v3::registration::RegistredData {
                    wg_port,
                    pub_key,
                    private_ip: ipv4.into()
                }
            })
        );
    }

    #[test]
    fn upgrade_remaining_bandwidth_resp() {
        let available_bandwidth = 42;
        let remaining_bandwidth_data = Some(v3::registration::RemainingBandwidthData {
            available_bandwidth,
        });
        let request_id = 123;
        let reply_to = Recipient::try_from_base58_string(RECIPIENT).unwrap();

        let msg = v3::response::AuthenticatorResponse::new_remaining_bandwidth(
            remaining_bandwidth_data,
            reply_to,
            request_id,
        );
        let upgraded_msg = v4::response::AuthenticatorResponse::try_from(msg).unwrap();

        assert_eq!(
            upgraded_msg.protocol,
            Protocol {
                version: 4,
                service_provider_type: ServiceProviderType::Authenticator
            }
        );
        assert_eq!(
            upgraded_msg.data,
            v4::response::AuthenticatorResponseData::RemainingBandwidth(
                v4::response::RemainingBandwidthResponse {
                    request_id,
                    reply_to,
                    reply: Some(v4::registration::RemainingBandwidthData {
                        available_bandwidth,
                    })
                }
            )
        );
    }

    #[test]
    fn downgrade_remaining_bandwidth_resp() {
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
        let downgraded_msg = v3::response::AuthenticatorResponse::try_from(msg).unwrap();

        assert_eq!(
            downgraded_msg.protocol,
            Protocol {
                version: 3,
                service_provider_type: ServiceProviderType::Authenticator
            }
        );
        assert_eq!(
            downgraded_msg.data,
            v3::response::AuthenticatorResponseData::RemainingBandwidth(
                v3::response::RemainingBandwidthResponse {
                    request_id,
                    reply_to,
                    reply: Some(v3::registration::RemainingBandwidthData {
                        available_bandwidth,
                    })
                }
            )
        );
    }

    #[test]
    fn downgrade_topup_resp() {
        let available_bandwidth = 42;
        let remaining_bandwidth_data = v4::registration::RemainingBandwidthData {
            available_bandwidth,
        };
        let request_id = 123;
        let reply_to = Recipient::try_from_base58_string(RECIPIENT).unwrap();

        let msg = v4::response::AuthenticatorResponse::new_topup_bandwidth(
            remaining_bandwidth_data,
            reply_to,
            request_id,
        );
        assert!(v3::response::AuthenticatorResponse::try_from(msg).is_err());
    }
}
