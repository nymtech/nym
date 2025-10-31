// Copyright 2025 Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_sphinx::addressing::Recipient;
use nym_wireguard_types::PeerPublicKey;

use crate::{
    AuthenticatorVersion, Error,
    latest::registration::IpPair,
    traits::{FinalMessage, InitMessage, QueryBandwidthMessage, TopUpMessage, Versionable},
    v2, v3, v4, v5,
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

    pub fn bytes(&self, reply_to: Recipient) -> Result<(Vec<u8>, u64), Error> {
        match self.version() {
            AuthenticatorVersion::V1 => Err(Error::UnsupportedVersion),
            AuthenticatorVersion::V2 => {
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
                        Ok((req.to_bytes()?, id))
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
                                credential: final_message.credential(),
                            },
                            reply_to,
                        );
                        Ok((req.to_bytes()?, id))
                    }
                    ClientMessage::Query(query_message) => {
                        let (req, id) = AuthenticatorRequest::new_query_request(
                            query_message.pub_key(),
                            reply_to,
                        );
                        Ok((req.to_bytes()?, id))
                    }
                    _ => Err(Error::UnsupportedMessage),
                }
            }
            AuthenticatorVersion::V3 => {
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
                        Ok((req.to_bytes()?, id))
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
                                credential: final_message.credential(),
                            },
                            reply_to,
                        );
                        Ok((req.to_bytes()?, id))
                    }
                    ClientMessage::Query(query_message) => {
                        let (req, id) = AuthenticatorRequest::new_query_request(
                            query_message.pub_key(),
                            reply_to,
                        );
                        Ok((req.to_bytes()?, id))
                    }
                    ClientMessage::TopUp(top_up_message) => {
                        let (req, id) = AuthenticatorRequest::new_topup_request(
                            TopUpMessage {
                                pub_key: top_up_message.pub_key(),
                                credential: top_up_message.credential(),
                            },
                            reply_to,
                        );
                        Ok((req.to_bytes()?, id))
                    }
                }
            }
            AuthenticatorVersion::V4 => {
                use v4::{
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
                        Ok((req.to_bytes()?, id))
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
                                    }
                                    .into(),
                                    mac: ClientMac::new(final_message.gateway_client_mac()),
                                },
                                credential: final_message.credential(),
                            },
                            reply_to,
                        );
                        Ok((req.to_bytes()?, id))
                    }
                    ClientMessage::Query(query_message) => {
                        let (req, id) = AuthenticatorRequest::new_query_request(
                            query_message.pub_key(),
                            reply_to,
                        );
                        Ok((req.to_bytes()?, id))
                    }
                    ClientMessage::TopUp(top_up_message) => {
                        let (req, id) = AuthenticatorRequest::new_topup_request(
                            TopUpMessage {
                                pub_key: top_up_message.pub_key(),
                                credential: top_up_message.credential(),
                            },
                            reply_to,
                        );
                        Ok((req.to_bytes()?, id))
                    }
                }
            }
            AuthenticatorVersion::V5 => {
                use v5::{
                    registration::{ClientMac, FinalMessage, GatewayClient, InitMessage},
                    request::AuthenticatorRequest,
                    topup::TopUpMessage,
                };
                match self {
                    ClientMessage::Initial(init_message) => {
                        let (req, id) = AuthenticatorRequest::new_initial_request(InitMessage {
                            pub_key: init_message.pub_key(),
                        });
                        Ok((req.to_bytes()?, id))
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
                        Ok((req.to_bytes()?, id))
                    }
                    ClientMessage::Query(query_message) => {
                        let (req, id) =
                            AuthenticatorRequest::new_query_request(query_message.pub_key());
                        Ok((req.to_bytes()?, id))
                    }
                    ClientMessage::TopUp(top_up_message) => {
                        let (req, id) = AuthenticatorRequest::new_topup_request(TopUpMessage {
                            pub_key: top_up_message.pub_key(),
                            credential: top_up_message.credential(),
                        });
                        Ok((req.to_bytes()?, id))
                    }
                }
            }
            AuthenticatorVersion::UNKNOWN => Err(Error::UnknownVersion),
        }
    }

    pub fn use_surbs(&self) -> bool {
        use AuthenticatorVersion::*;
        match self.version() {
            V1 | V2 | V3 | V4 => false,
            V5 => true,
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
