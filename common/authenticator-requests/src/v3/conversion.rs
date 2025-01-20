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

impl TryFrom<v3::request::AuthenticatorRequest> for v2::request::AuthenticatorRequest {
    type Error = crate::Error;

    fn try_from(
        authenticator_request: v3::request::AuthenticatorRequest,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            protocol: Protocol {
                version: 2,
                service_provider_type: ServiceProviderType::Authenticator,
            },
            data: authenticator_request.data.try_into()?,
            reply_to: authenticator_request.reply_to,
            request_id: authenticator_request.request_id,
        })
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

impl TryFrom<v3::request::AuthenticatorRequestData> for v2::request::AuthenticatorRequestData {
    type Error = crate::Error;

    fn try_from(
        authenticator_request_data: v3::request::AuthenticatorRequestData,
    ) -> Result<Self, Self::Error> {
        match authenticator_request_data {
            v3::request::AuthenticatorRequestData::Initial(init_msg) => Ok(
                v2::request::AuthenticatorRequestData::Initial(init_msg.into()),
            ),
            v3::request::AuthenticatorRequestData::Final(gw_client) => Ok(
                v2::request::AuthenticatorRequestData::Final(gw_client.into()),
            ),
            v3::request::AuthenticatorRequestData::QueryBandwidth(pub_key) => Ok(
                v2::request::AuthenticatorRequestData::QueryBandwidth(pub_key),
            ),
            v3::request::AuthenticatorRequestData::TopUpBandwidth(_) => Err(
                Self::Error::Conversion("no top up bandwidth variant in v2".to_string()),
            ),
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

impl From<v3::registration::InitMessage> for v2::registration::InitMessage {
    fn from(init_msg: v3::registration::InitMessage) -> Self {
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

impl From<Box<v3::registration::FinalMessage>> for Box<v2::registration::FinalMessage> {
    fn from(gw_client: Box<v3::registration::FinalMessage>) -> Self {
        Box::new(v2::registration::FinalMessage {
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
            protocol: Protocol {
                version: 2,
                service_provider_type: authenticator_response.protocol.service_provider_type,
            },
        })
    }
}

impl From<v2::response::AuthenticatorResponse> for v3::response::AuthenticatorResponse {
    fn from(value: v2::response::AuthenticatorResponse) -> Self {
        Self {
            protocol: Protocol {
                version: 3,
                service_provider_type: value.protocol.service_provider_type,
            },
            data: value.data.into(),
            reply_to: value.reply_to,
        }
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

impl From<v2::response::AuthenticatorResponseData> for v3::response::AuthenticatorResponseData {
    fn from(value: v2::response::AuthenticatorResponseData) -> Self {
        match value {
            v2::response::AuthenticatorResponseData::PendingRegistration(
                pending_registration_response,
            ) => Self::PendingRegistration(pending_registration_response.into()),
            v2::response::AuthenticatorResponseData::Registered(registered_response) => {
                Self::Registered(registered_response.into())
            }
            v2::response::AuthenticatorResponseData::RemainingBandwidth(
                remaining_bandwidth_response,
            ) => Self::RemainingBandwidth(remaining_bandwidth_response.into()),
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

impl From<v2::response::PendingRegistrationResponse> for v3::response::PendingRegistrationResponse {
    fn from(value: v2::response::PendingRegistrationResponse) -> Self {
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

impl From<v2::response::RegisteredResponse> for v3::response::RegisteredResponse {
    fn from(value: v2::response::RegisteredResponse) -> Self {
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

impl From<v2::response::RemainingBandwidthResponse> for v3::response::RemainingBandwidthResponse {
    fn from(value: v2::response::RemainingBandwidthResponse) -> Self {
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

impl From<v2::registration::RegistrationData> for v3::registration::RegistrationData {
    fn from(value: v2::registration::RegistrationData) -> Self {
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

impl From<v2::registration::RegistredData> for v3::registration::RegistredData {
    fn from(value: v2::registration::RegistredData) -> Self {
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

impl From<v2::registration::RemainingBandwidthData> for v3::registration::RemainingBandwidthData {
    fn from(value: v2::registration::RemainingBandwidthData) -> Self {
        Self {
            available_bandwidth: value.available_bandwidth,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{net::IpAddr, str::FromStr};

    use nym_credentials_interface::CredentialSpendingData;
    use nym_crypto::asymmetric::encryption::PrivateKey;
    use nym_sphinx::addressing::Recipient;
    use nym_wireguard_types::PeerPublicKey;
    use x25519_dalek::PublicKey;

    use super::*;
    use crate::util::tests::{CREDENTIAL_BYTES, RECIPIENT};

    #[test]
    fn upgrade_initial_req() {
        let pub_key = PeerPublicKey::new(PublicKey::from([0; 32]));
        let reply_to = Recipient::try_from_base58_string(RECIPIENT).unwrap();

        let (msg, _) = v2::request::AuthenticatorRequest::new_initial_request(
            v2::registration::InitMessage::new(pub_key),
            reply_to,
        );
        let upgraded_msg = v3::request::AuthenticatorRequest::from(msg);

        assert_eq!(
            upgraded_msg.protocol,
            Protocol {
                version: 3,
                service_provider_type: ServiceProviderType::Authenticator
            }
        );
        assert_eq!(
            upgraded_msg.data,
            v3::request::AuthenticatorRequestData::Initial(v3::registration::InitMessage {
                pub_key
            })
        );
    }

    #[test]
    fn downgrade_initial_req() {
        let pub_key = PeerPublicKey::new(PublicKey::from([0; 32]));
        let reply_to = Recipient::try_from_base58_string(RECIPIENT).unwrap();

        let (msg, _) = v3::request::AuthenticatorRequest::new_initial_request(
            v3::registration::InitMessage::new(pub_key),
            reply_to,
        );
        let downgraded_msg = v2::request::AuthenticatorRequest::try_from(msg).unwrap();

        assert_eq!(
            downgraded_msg.protocol,
            Protocol {
                version: 2,
                service_provider_type: ServiceProviderType::Authenticator
            }
        );
        assert_eq!(
            downgraded_msg.data,
            v2::request::AuthenticatorRequestData::Initial(v2::registration::InitMessage {
                pub_key
            })
        );
    }

    #[test]
    fn upgrade_final_req() {
        let mut rng = rand::thread_rng();

        let local_secret = PrivateKey::new(&mut rng);
        let remote_secret = x25519_dalek::StaticSecret::random_from_rng(&mut rng);
        let private_ip = IpAddr::from_str("10.10.10.10").unwrap();
        let nonce = 42;
        let gateway_client = v2::registration::GatewayClient::new(
            &local_secret,
            (&remote_secret).into(),
            private_ip,
            nonce,
        );
        let credential = Some(CredentialSpendingData::try_from_bytes(&CREDENTIAL_BYTES).unwrap());
        let final_message = v2::registration::FinalMessage {
            gateway_client,
            credential: credential.clone(),
        };
        let reply_to = Recipient::try_from_base58_string(RECIPIENT).unwrap();

        let (msg, _) =
            v2::request::AuthenticatorRequest::new_final_request(final_message, reply_to);
        let upgraded_msg = v3::request::AuthenticatorRequest::from(msg);

        assert_eq!(
            upgraded_msg.protocol,
            Protocol {
                version: 3,
                service_provider_type: ServiceProviderType::Authenticator
            }
        );
        assert_eq!(
            upgraded_msg.data,
            v3::request::AuthenticatorRequestData::Final(Box::new(
                v3::registration::FinalMessage {
                    gateway_client: v3::registration::GatewayClient::new(
                        &local_secret,
                        (&remote_secret).into(),
                        private_ip,
                        nonce,
                    ),
                    credential
                }
            ))
        );
    }

    #[test]
    fn downgrade_final_req() {
        let mut rng = rand::thread_rng();

        let local_secret = PrivateKey::new(&mut rng);
        let remote_secret = x25519_dalek::StaticSecret::random_from_rng(&mut rng);
        let private_ip = IpAddr::from_str("10.10.10.10").unwrap();
        let nonce = 42;
        let gateway_client = v3::registration::GatewayClient::new(
            &local_secret,
            (&remote_secret).into(),
            private_ip,
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
        let upgraded_msg = v2::request::AuthenticatorRequest::try_from(msg).unwrap();

        assert_eq!(
            upgraded_msg.protocol,
            Protocol {
                version: 2,
                service_provider_type: ServiceProviderType::Authenticator
            }
        );
        assert_eq!(
            upgraded_msg.data,
            v2::request::AuthenticatorRequestData::Final(Box::new(
                v2::registration::FinalMessage {
                    gateway_client: v2::registration::GatewayClient::new(
                        &local_secret,
                        (&remote_secret).into(),
                        private_ip,
                        nonce,
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

        let (msg, _) = v2::request::AuthenticatorRequest::new_query_request(pub_key, reply_to);
        let upgraded_msg = v3::request::AuthenticatorRequest::from(msg);

        assert_eq!(
            upgraded_msg.protocol,
            Protocol {
                version: 3,
                service_provider_type: ServiceProviderType::Authenticator
            }
        );
        assert_eq!(
            upgraded_msg.data,
            v3::request::AuthenticatorRequestData::QueryBandwidth(pub_key)
        );
    }

    #[test]
    fn downgrade_query_req() {
        let pub_key = PeerPublicKey::new(PublicKey::from([0; 32]));
        let reply_to = Recipient::try_from_base58_string(RECIPIENT).unwrap();

        let (msg, _) = v3::request::AuthenticatorRequest::new_query_request(pub_key, reply_to);
        let downgraded_msg = v2::request::AuthenticatorRequest::try_from(msg).unwrap();

        assert_eq!(
            downgraded_msg.protocol,
            Protocol {
                version: 2,
                service_provider_type: ServiceProviderType::Authenticator
            }
        );
        assert_eq!(
            downgraded_msg.data,
            v2::request::AuthenticatorRequestData::QueryBandwidth(pub_key)
        );
    }

    #[test]
    fn downgrade_topup_req() {
        let pub_key = PeerPublicKey::new(PublicKey::from([0; 32]));
        let credential = CredentialSpendingData::try_from_bytes(&CREDENTIAL_BYTES).unwrap();
        let top_up_message = v3::topup::TopUpMessage {
            pub_key,
            credential,
        };
        let reply_to = Recipient::try_from_base58_string(RECIPIENT).unwrap();

        let (msg, _) =
            v3::request::AuthenticatorRequest::new_topup_request(top_up_message, reply_to);
        assert!(v2::request::AuthenticatorRequest::try_from(msg).is_err());
    }

    #[test]
    fn upgrade_pending_reg_resp() {
        let mut rng = rand::thread_rng();

        let local_secret = PrivateKey::new(&mut rng);
        let remote_secret = x25519_dalek::StaticSecret::random_from_rng(&mut rng);
        let private_ip = IpAddr::from_str("10.10.10.10").unwrap();
        let nonce = 42;
        let wg_port = 51822;
        let gateway_data = v2::registration::GatewayClient::new(
            &local_secret,
            (&remote_secret).into(),
            private_ip,
            nonce,
        );
        let registration_data = v2::registration::RegistrationData {
            nonce,
            gateway_data,
            wg_port,
        };
        let request_id = 123;
        let reply_to = Recipient::try_from_base58_string(RECIPIENT).unwrap();

        let msg = v2::response::AuthenticatorResponse::new_pending_registration_success(
            registration_data,
            request_id,
            reply_to,
        );
        let upgraded_msg = v3::response::AuthenticatorResponse::from(msg);

        assert_eq!(
            upgraded_msg.protocol,
            Protocol {
                version: 3,
                service_provider_type: ServiceProviderType::Authenticator
            }
        );
        assert_eq!(
            upgraded_msg.data,
            v3::response::AuthenticatorResponseData::PendingRegistration(
                v3::response::PendingRegistrationResponse {
                    request_id,
                    reply_to,
                    reply: v3::registration::RegistrationData {
                        nonce,
                        gateway_data: v3::registration::GatewayClient::new(
                            &local_secret,
                            (&remote_secret).into(),
                            private_ip,
                            nonce,
                        ),
                        wg_port,
                    }
                }
            )
        );
    }

    #[test]
    fn downgrade_pending_reg_resp() {
        let mut rng = rand::thread_rng();

        let local_secret = PrivateKey::new(&mut rng);
        let remote_secret = x25519_dalek::StaticSecret::random_from_rng(&mut rng);
        let private_ip = IpAddr::from_str("10.10.10.10").unwrap();
        let nonce = 42;
        let wg_port = 51822;
        let gateway_data = v3::registration::GatewayClient::new(
            &local_secret,
            (&remote_secret).into(),
            private_ip,
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
        let downgraded_msg = v2::response::AuthenticatorResponse::try_from(msg).unwrap();

        assert_eq!(
            downgraded_msg.protocol,
            Protocol {
                version: 2,
                service_provider_type: ServiceProviderType::Authenticator
            }
        );
        assert_eq!(
            downgraded_msg.data,
            v2::response::AuthenticatorResponseData::PendingRegistration(
                v2::response::PendingRegistrationResponse {
                    request_id,
                    reply_to,
                    reply: v2::registration::RegistrationData {
                        nonce,
                        gateway_data: v2::registration::GatewayClient::new(
                            &local_secret,
                            (&remote_secret).into(),
                            private_ip,
                            nonce,
                        ),
                        wg_port,
                    }
                }
            )
        );
    }

    #[test]
    fn upgrade_registered_resp() {
        let pub_key = PeerPublicKey::new(PublicKey::from([0; 32]));
        let private_ip = IpAddr::from_str("10.10.10.10").unwrap();
        let wg_port = 51822;
        let registred_data = v2::registration::RegistredData {
            pub_key,
            private_ip,
            wg_port,
        };
        let request_id = 123;
        let reply_to = Recipient::try_from_base58_string(RECIPIENT).unwrap();

        let msg = v2::response::AuthenticatorResponse::new_registered(
            registred_data,
            reply_to,
            request_id,
        );
        let upgraded_msg = v3::response::AuthenticatorResponse::from(msg);

        assert_eq!(
            upgraded_msg.protocol,
            Protocol {
                version: 3,
                service_provider_type: ServiceProviderType::Authenticator
            }
        );
        assert_eq!(
            upgraded_msg.data,
            v3::response::AuthenticatorResponseData::Registered(v3::response::RegisteredResponse {
                request_id,
                reply_to,
                reply: v3::registration::RegistredData {
                    wg_port,
                    pub_key,
                    private_ip
                }
            })
        );
    }

    #[test]
    fn downgrade_registered_resp() {
        let pub_key = PeerPublicKey::new(PublicKey::from([0; 32]));
        let private_ip = IpAddr::from_str("10.10.10.10").unwrap();
        let wg_port = 51822;
        let registred_data = v3::registration::RegistredData {
            pub_key,
            private_ip,
            wg_port,
        };
        let request_id = 123;
        let reply_to = Recipient::try_from_base58_string(RECIPIENT).unwrap();

        let msg = v3::response::AuthenticatorResponse::new_registered(
            registred_data,
            reply_to,
            request_id,
        );
        let downgraded_msg = v2::response::AuthenticatorResponse::try_from(msg).unwrap();

        assert_eq!(
            downgraded_msg.protocol,
            Protocol {
                version: 2,
                service_provider_type: ServiceProviderType::Authenticator
            }
        );
        assert_eq!(
            downgraded_msg.data,
            v2::response::AuthenticatorResponseData::Registered(v2::response::RegisteredResponse {
                request_id,
                reply_to,
                reply: v2::registration::RegistredData {
                    wg_port,
                    pub_key,
                    private_ip
                }
            })
        );
    }

    #[test]
    fn upgrade_remaining_bandwidth_resp() {
        let available_bandwidth = 42;
        let remaining_bandwidth_data = Some(v2::registration::RemainingBandwidthData {
            available_bandwidth,
        });
        let request_id = 123;
        let reply_to = Recipient::try_from_base58_string(RECIPIENT).unwrap();

        let msg = v2::response::AuthenticatorResponse::new_remaining_bandwidth(
            remaining_bandwidth_data,
            reply_to,
            request_id,
        );
        let upgraded_msg = v3::response::AuthenticatorResponse::from(msg);

        assert_eq!(
            upgraded_msg.protocol,
            Protocol {
                version: 3,
                service_provider_type: ServiceProviderType::Authenticator
            }
        );
        assert_eq!(
            upgraded_msg.data,
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
    fn downgrade_remaining_bandwidth_resp() {
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
        let downgraded_msg = v2::response::AuthenticatorResponse::try_from(msg).unwrap();

        assert_eq!(
            downgraded_msg.protocol,
            Protocol {
                version: 2,
                service_provider_type: ServiceProviderType::Authenticator
            }
        );
        assert_eq!(
            downgraded_msg.data,
            v2::response::AuthenticatorResponseData::RemainingBandwidth(
                v2::response::RemainingBandwidthResponse {
                    request_id,
                    reply_to,
                    reply: Some(v2::registration::RemainingBandwidthData {
                        available_bandwidth,
                    })
                }
            )
        );
    }

    #[test]
    fn downgrade_topup_resp() {
        let available_bandwidth = 42;
        let remaining_bandwidth_data = v3::registration::RemainingBandwidthData {
            available_bandwidth,
        };
        let request_id = 123;
        let reply_to = Recipient::try_from_base58_string(RECIPIENT).unwrap();

        let msg = v3::response::AuthenticatorResponse::new_topup_bandwidth(
            remaining_bandwidth_data,
            reply_to,
            request_id,
        );
        assert!(v2::response::AuthenticatorResponse::try_from(msg).is_err());
    }
}
