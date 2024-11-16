// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::net::IpAddr;

use nym_credentials_interface::CredentialSpendingData;
use nym_crypto::asymmetric::x25519::PrivateKey;
use nym_sdk::mixnet::Recipient;
use nym_service_provider_requests_common::{Protocol, ServiceProviderType};
use nym_wireguard_types::PeerPublicKey;

use crate::{v1, v2, v3, Error};

#[derive(Copy, Clone, Debug)]
pub enum AuthenticatorVersion {
    V1,
    V2,
    V3,
    UNKNOWN,
}

impl From<Protocol> for AuthenticatorVersion {
    fn from(value: Protocol) -> Self {
        if value.service_provider_type != ServiceProviderType::Authenticator {
            AuthenticatorVersion::UNKNOWN
        } else if value.version == v1::VERSION {
            AuthenticatorVersion::V1
        } else if value.version == v2::VERSION {
            AuthenticatorVersion::V2
        } else if value.version == v3::VERSION {
            AuthenticatorVersion::V3
        } else {
            AuthenticatorVersion::UNKNOWN
        }
    }
}

pub trait InitMessage {
    fn pub_key(&self) -> PeerPublicKey;
}

impl InitMessage for v1::registration::InitMessage {
    fn pub_key(&self) -> PeerPublicKey {
        self.pub_key
    }
}

impl InitMessage for v2::registration::InitMessage {
    fn pub_key(&self) -> PeerPublicKey {
        self.pub_key
    }
}

impl InitMessage for v3::registration::InitMessage {
    fn pub_key(&self) -> PeerPublicKey {
        self.pub_key
    }
}

pub trait FinalMessage {
    fn pub_key(&self) -> PeerPublicKey;
    fn verify(&self, private_key: &PrivateKey, nonce: u64) -> Result<(), Error>;
    fn private_ip(&self) -> IpAddr;
    fn credential(&self) -> Option<CredentialSpendingData>;
}

impl FinalMessage for v1::GatewayClient {
    fn pub_key(&self) -> PeerPublicKey {
        self.pub_key
    }

    fn verify(&self, private_key: &PrivateKey, nonce: u64) -> Result<(), Error> {
        self.verify(private_key, nonce)
    }

    fn private_ip(&self) -> IpAddr {
        self.private_ip
    }

    fn credential(&self) -> Option<CredentialSpendingData> {
        None
    }
}

impl FinalMessage for v2::registration::FinalMessage {
    fn pub_key(&self) -> PeerPublicKey {
        self.gateway_client.pub_key
    }

    fn verify(&self, private_key: &PrivateKey, nonce: u64) -> Result<(), Error> {
        self.gateway_client.verify(private_key, nonce)
    }

    fn private_ip(&self) -> IpAddr {
        self.gateway_client.private_ip
    }

    fn credential(&self) -> Option<CredentialSpendingData> {
        self.credential.clone()
    }
}

impl FinalMessage for v3::registration::FinalMessage {
    fn pub_key(&self) -> PeerPublicKey {
        self.gateway_client.pub_key
    }

    fn verify(&self, private_key: &PrivateKey, nonce: u64) -> Result<(), Error> {
        self.gateway_client.verify(private_key, nonce)
    }

    fn private_ip(&self) -> IpAddr {
        self.gateway_client.private_ip
    }

    fn credential(&self) -> Option<CredentialSpendingData> {
        self.credential.clone()
    }
}

pub trait QueryBandwidthMessage {
    fn pub_key(&self) -> PeerPublicKey;
}

impl QueryBandwidthMessage for PeerPublicKey {
    fn pub_key(&self) -> PeerPublicKey {
        *self
    }
}

pub trait TopUpMessage {
    fn pub_key(&self) -> PeerPublicKey;
    fn credential(&self) -> CredentialSpendingData;
}

impl TopUpMessage for v3::topup::TopUpMessage {
    fn pub_key(&self) -> PeerPublicKey {
        self.pub_key
    }

    fn credential(&self) -> CredentialSpendingData {
        self.credential.clone()
    }
}

pub enum AuthenticatorRequest {
    Initial {
        msg: Box<dyn InitMessage + Send + Sync + 'static>,
        protocol: Protocol,
        reply_to: Recipient,
        request_id: u64,
    },
    Final {
        msg: Box<dyn FinalMessage + Send + Sync + 'static>,
        protocol: Protocol,
        reply_to: Recipient,
        request_id: u64,
    },
    QueryBandwidth {
        msg: Box<dyn QueryBandwidthMessage + Send + Sync + 'static>,
        protocol: Protocol,
        reply_to: Recipient,
        request_id: u64,
    },
    TopUpBandwidth {
        msg: Box<dyn TopUpMessage + Send + Sync + 'static>,
        protocol: Protocol,
        reply_to: Recipient,
        request_id: u64,
    },
}

impl From<v1::request::AuthenticatorRequest> for AuthenticatorRequest {
    fn from(value: v1::request::AuthenticatorRequest) -> Self {
        match value.data {
            v1::request::AuthenticatorRequestData::Initial(init_message) => Self::Initial {
                msg: Box::new(init_message),
                protocol: Protocol {
                    version: value.version,
                    service_provider_type: ServiceProviderType::Authenticator,
                },
                reply_to: value.reply_to,
                request_id: value.request_id,
            },
            v1::request::AuthenticatorRequestData::Final(gateway_client) => Self::Final {
                msg: Box::new(gateway_client),
                protocol: Protocol {
                    version: value.version,
                    service_provider_type: ServiceProviderType::Authenticator,
                },
                reply_to: value.reply_to,
                request_id: value.request_id,
            },
            v1::request::AuthenticatorRequestData::QueryBandwidth(peer_public_key) => {
                Self::QueryBandwidth {
                    msg: Box::new(peer_public_key),
                    protocol: Protocol {
                        version: value.version,
                        service_provider_type: ServiceProviderType::Authenticator,
                    },
                    reply_to: value.reply_to,
                    request_id: value.request_id,
                }
            }
        }
    }
}

impl From<v2::request::AuthenticatorRequest> for AuthenticatorRequest {
    fn from(value: v2::request::AuthenticatorRequest) -> Self {
        match value.data {
            v2::request::AuthenticatorRequestData::Initial(init_message) => Self::Initial {
                msg: Box::new(init_message),
                protocol: value.protocol,
                reply_to: value.reply_to,
                request_id: value.request_id,
            },
            v2::request::AuthenticatorRequestData::Final(final_message) => Self::Final {
                msg: final_message,
                protocol: value.protocol,
                reply_to: value.reply_to,
                request_id: value.request_id,
            },
            v2::request::AuthenticatorRequestData::QueryBandwidth(peer_public_key) => {
                Self::QueryBandwidth {
                    msg: Box::new(peer_public_key),
                    protocol: value.protocol,
                    reply_to: value.reply_to,
                    request_id: value.request_id,
                }
            }
        }
    }
}

impl From<v3::request::AuthenticatorRequest> for AuthenticatorRequest {
    fn from(value: v3::request::AuthenticatorRequest) -> Self {
        match value.data {
            v3::request::AuthenticatorRequestData::Initial(init_message) => Self::Initial {
                msg: Box::new(init_message),
                protocol: value.protocol,
                reply_to: value.reply_to,
                request_id: value.request_id,
            },
            v3::request::AuthenticatorRequestData::Final(final_message) => Self::Final {
                msg: final_message,
                protocol: value.protocol,
                reply_to: value.reply_to,
                request_id: value.request_id,
            },
            v3::request::AuthenticatorRequestData::QueryBandwidth(peer_public_key) => {
                Self::QueryBandwidth {
                    msg: Box::new(peer_public_key),
                    protocol: value.protocol,
                    reply_to: value.reply_to,
                    request_id: value.request_id,
                }
            }
            v3::request::AuthenticatorRequestData::TopUpBandwidth(top_up_message) => {
                Self::TopUpBandwidth {
                    msg: top_up_message,
                    protocol: value.protocol,
                    reply_to: value.reply_to,
                    request_id: value.request_id,
                }
            }
        }
    }
}
