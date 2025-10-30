// Copyright 2025 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_sphinx::addressing::Recipient;
use nym_wireguard_types::PeerPublicKey;

use crate::{
    AuthenticatorVersion, Error,
    traits::{FinalMessage, InitMessage, QueryBandwidthMessage, TopUpMessage, Versionable},
    v2, v3, v4, v5, v6,
};

// This is very redundant with AuthenticatorRequest and I reckon they could be smooshed.
// It is a bit out of scope for me at the moment though
#[derive(Debug)]
pub enum ClientMessage {
    Initial(Box<dyn InitMessage + Send + Sync + 'static>),
    Final(Box<dyn FinalMessage + Send + Sync + 'static>),
    Query(Box<dyn QueryBandwidthMessage + Send + Sync + 'static>),
    TopUp(Box<dyn TopUpMessage + Send + Sync + 'static>),
}

pub struct SerialisedRequest {
    pub bytes: Vec<u8>,
    pub request_id: u64,
}

impl SerialisedRequest {
    pub fn new(bytes: Vec<u8>, request_id: u64) -> Self {
        Self { bytes, request_id }
    }
}

impl ClientMessage {
    fn serialise_v1(&self) -> Result<SerialisedRequest, Error> {
        Err(Error::UnsupportedVersion)
    }

    fn serialise_v2(&self, reply_to: Recipient) -> Result<SerialisedRequest, Error> {
        use v2::{
            registration::{ClientMac, FinalMessage, GatewayClient, InitMessage},
            request::AuthenticatorRequest,
        };
        match self {
            ClientMessage::Initial(init_message) => {
                let (req, id) = AuthenticatorRequest::new_initial_request(
                    InitMessage {
                        pub_key: init_message.pub_key(),
                    },
                    reply_to,
                );
                Ok(SerialisedRequest::new(req.to_bytes()?, id))
            }
            ClientMessage::Final(final_message) => {
                let (req, id) = AuthenticatorRequest::new_final_request(
                    FinalMessage {
                        gateway_client: GatewayClient {
                            pub_key: final_message.gateway_client_pub_key(),
                            private_ip: final_message
                                .gateway_client_ipv4()
                                .ok_or(Error::UnsupportedMessage)?
                                .into(),
                            mac: ClientMac::new(final_message.gateway_client_mac()),
                        },
                        credential: final_message
                            .credential()
                            .and_then(|c| c.credential.into_zk_nym())
                            .map(|c| *c),
                    },
                    reply_to,
                );
                Ok(SerialisedRequest::new(req.to_bytes()?, id))
            }
            ClientMessage::Query(query_message) => {
                let (req, id) =
                    AuthenticatorRequest::new_query_request(query_message.pub_key(), reply_to);
                Ok(SerialisedRequest::new(req.to_bytes()?, id))
            }
            _ => Err(Error::UnsupportedMessage),
        }
    }

    fn serialise_v3(&self, reply_to: Recipient) -> Result<SerialisedRequest, Error> {
        use v3::{
            registration::{ClientMac, FinalMessage, GatewayClient, InitMessage},
            request::AuthenticatorRequest,
            topup::TopUpMessage,
        };
        match self {
            ClientMessage::Initial(init_message) => {
                let (req, id) = AuthenticatorRequest::new_initial_request(
                    InitMessage {
                        pub_key: init_message.pub_key(),
                    },
                    reply_to,
                );
                Ok(SerialisedRequest::new(req.to_bytes()?, id))
            }
            ClientMessage::Final(final_message) => {
                let (req, id) = AuthenticatorRequest::new_final_request(
                    FinalMessage {
                        gateway_client: GatewayClient {
                            pub_key: final_message.gateway_client_pub_key(),
                            private_ip: final_message
                                .gateway_client_ipv4()
                                .ok_or(Error::UnsupportedMessage)?
                                .into(),
                            mac: ClientMac::new(final_message.gateway_client_mac()),
                        },
                        credential: final_message
                            .credential()
                            .and_then(|c| c.credential.into_zk_nym())
                            .map(|c| *c),
                    },
                    reply_to,
                );
                Ok(SerialisedRequest::new(req.to_bytes()?, id))
            }
            ClientMessage::Query(query_message) => {
                let (req, id) =
                    AuthenticatorRequest::new_query_request(query_message.pub_key(), reply_to);
                Ok(SerialisedRequest::new(req.to_bytes()?, id))
            }
            ClientMessage::TopUp(top_up_message) => {
                let (req, id) = AuthenticatorRequest::new_topup_request(
                    TopUpMessage {
                        pub_key: top_up_message.pub_key(),
                        credential: top_up_message.credential(),
                    },
                    reply_to,
                );
                Ok(SerialisedRequest::new(req.to_bytes()?, id))
            }
        }
    }

    fn serialise_v4(&self, reply_to: Recipient) -> Result<SerialisedRequest, Error> {
        use v4::{
            registration::{ClientMac, FinalMessage, GatewayClient, InitMessage, IpPair},
            request::AuthenticatorRequest,
            topup::TopUpMessage,
        };
        match self {
            ClientMessage::Initial(init_message) => {
                let (req, id) = AuthenticatorRequest::new_initial_request(
                    InitMessage {
                        pub_key: init_message.pub_key(),
                    },
                    reply_to,
                );
                Ok(SerialisedRequest::new(req.to_bytes()?, id))
            }
            ClientMessage::Final(final_message) => {
                let (req, id) = AuthenticatorRequest::new_final_request(
                    FinalMessage {
                        gateway_client: GatewayClient {
                            pub_key: final_message.gateway_client_pub_key(),
                            private_ips: IpPair {
                                ipv4: final_message
                                    .gateway_client_ipv4()
                                    .ok_or(Error::UnsupportedMessage)?,
                                ipv6: final_message
                                    .gateway_client_ipv6()
                                    .ok_or(Error::UnsupportedMessage)?,
                            },
                            mac: ClientMac::new(final_message.gateway_client_mac()),
                        },
                        credential: final_message
                            .credential()
                            .and_then(|c| c.credential.into_zk_nym())
                            .map(|c| *c),
                    },
                    reply_to,
                );
                Ok(SerialisedRequest::new(req.to_bytes()?, id))
            }
            ClientMessage::Query(query_message) => {
                let (req, id) =
                    AuthenticatorRequest::new_query_request(query_message.pub_key(), reply_to);
                Ok(SerialisedRequest::new(req.to_bytes()?, id))
            }
            ClientMessage::TopUp(top_up_message) => {
                let (req, id) = AuthenticatorRequest::new_topup_request(
                    TopUpMessage {
                        pub_key: top_up_message.pub_key(),
                        credential: top_up_message.credential(),
                    },
                    reply_to,
                );
                Ok(SerialisedRequest::new(req.to_bytes()?, id))
            }
        }
    }

    fn serialise_v5(&self) -> Result<SerialisedRequest, Error> {
        use v5::{
            registration::{ClientMac, FinalMessage, GatewayClient, InitMessage, IpPair},
            request::AuthenticatorRequest,
            topup::TopUpMessage,
        };
        match self {
            ClientMessage::Initial(init_message) => {
                let (req, id) = AuthenticatorRequest::new_initial_request(InitMessage {
                    pub_key: init_message.pub_key(),
                });
                Ok(SerialisedRequest::new(req.to_bytes()?, id))
            }
            ClientMessage::Final(final_message) => {
                let (req, id) = AuthenticatorRequest::new_final_request(FinalMessage {
                    gateway_client: GatewayClient {
                        pub_key: final_message.gateway_client_pub_key(),
                        private_ips: IpPair {
                            ipv4: final_message
                                .gateway_client_ipv4()
                                .ok_or(Error::UnsupportedMessage)?,
                            ipv6: final_message
                                .gateway_client_ipv6()
                                .ok_or(Error::UnsupportedMessage)?,
                        },
                        mac: ClientMac::new(final_message.gateway_client_mac()),
                    },
                    credential: final_message
                        .credential()
                        .and_then(|c| c.credential.into_zk_nym())
                        .map(|c| *c),
                });
                Ok(SerialisedRequest::new(req.to_bytes()?, id))
            }
            ClientMessage::Query(query_message) => {
                let (req, id) = AuthenticatorRequest::new_query_request(query_message.pub_key());
                Ok(SerialisedRequest::new(req.to_bytes()?, id))
            }
            ClientMessage::TopUp(top_up_message) => {
                let (req, id) = AuthenticatorRequest::new_topup_request(TopUpMessage {
                    pub_key: top_up_message.pub_key(),
                    credential: top_up_message.credential(),
                });
                Ok(SerialisedRequest::new(req.to_bytes()?, id))
            }
        }
    }

    fn serialise_v6(&self) -> Result<SerialisedRequest, Error> {
        use v6::{
            registration::{ClientMac, FinalMessage, GatewayClient, InitMessage, IpPair},
            request::AuthenticatorRequest,
            topup::TopUpMessage,
        };
        match self {
            ClientMessage::Initial(init_message) => {
                let (req, id) = AuthenticatorRequest::new_initial_request(InitMessage {
                    pub_key: init_message.pub_key(),
                });
                Ok(SerialisedRequest::new(req.to_bytes()?, id))
            }
            ClientMessage::Final(final_message) => {
                let (req, id) = AuthenticatorRequest::new_final_request(FinalMessage {
                    gateway_client: GatewayClient {
                        pub_key: final_message.gateway_client_pub_key(),
                        private_ips: IpPair {
                            ipv4: final_message
                                .gateway_client_ipv4()
                                .ok_or(Error::UnsupportedMessage)?,
                            ipv6: final_message
                                .gateway_client_ipv6()
                                .ok_or(Error::UnsupportedMessage)?,
                        },
                        mac: ClientMac::new(final_message.gateway_client_mac()),
                    },
                    credential: final_message.credential(),
                });
                Ok(SerialisedRequest::new(req.to_bytes()?, id))
            }
            ClientMessage::Query(query_message) => {
                let (req, id) = AuthenticatorRequest::new_query_request(query_message.pub_key());
                Ok(SerialisedRequest::new(req.to_bytes()?, id))
            }
            ClientMessage::TopUp(top_up_message) => {
                let (req, id) = AuthenticatorRequest::new_topup_request(TopUpMessage {
                    pub_key: top_up_message.pub_key(),
                    credential: top_up_message.credential(),
                });
                Ok(SerialisedRequest::new(req.to_bytes()?, id))
            }
        }
    }
}

impl ClientMessage {
    // check if message is wasteful e.g. contains a credential
    pub fn is_wasteful(&self) -> bool {
        match self {
            Self::Final(msg) => msg.credential().is_some(),
            Self::TopUp(_) => true,
            Self::Initial(_) | Self::Query(_) => false,
        }
    }

    fn version(&self) -> AuthenticatorVersion {
        match self {
            ClientMessage::Initial(msg) => msg.version(),
            ClientMessage::Final(msg) => msg.version(),
            ClientMessage::Query(msg) => msg.version(),
            ClientMessage::TopUp(msg) => msg.version(),
        }
    }

    pub fn bytes(&self, reply_to: Recipient) -> Result<SerialisedRequest, Error> {
        match self.version() {
            AuthenticatorVersion::V1 => self.serialise_v1(),
            AuthenticatorVersion::V2 => self.serialise_v2(reply_to),
            AuthenticatorVersion::V3 => self.serialise_v3(reply_to),
            AuthenticatorVersion::V4 => self.serialise_v4(reply_to),
            AuthenticatorVersion::V5 => self.serialise_v5(),
            AuthenticatorVersion::V6 => self.serialise_v6(),
            AuthenticatorVersion::UNKNOWN => Err(Error::UnknownVersion),
        }
    }

    pub fn use_surbs(&self) -> bool {
        use AuthenticatorVersion::*;
        match self.version() {
            V1 | V2 | V3 | V4 => false,
            V5 | V6 => true,
            UNKNOWN => true,
        }
    }
}

// Same comment as above struct
#[derive(Debug)]
pub struct QueryMessageImpl {
    pub pub_key: PeerPublicKey,
    pub version: AuthenticatorVersion,
}

impl Versionable for QueryMessageImpl {
    fn version(&self) -> AuthenticatorVersion {
        self.version
    }
}

impl QueryBandwidthMessage for QueryMessageImpl {
    fn pub_key(&self) -> PeerPublicKey {
        self.pub_key
    }
}
