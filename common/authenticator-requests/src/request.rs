// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_service_provider_requests_common::{Protocol, ServiceProviderType};
use nym_sphinx::addressing::Recipient;

use crate::traits::{FinalMessage, InitMessage, QueryBandwidthMessage, TopUpMessage};
use crate::{v1, v2, v3, v4, v5};

#[derive(Debug)]
pub enum AuthenticatorRequest {
    Initial {
        msg: Box<dyn InitMessage + Send + Sync + 'static>,
        protocol: Protocol,
        reply_to: Option<Recipient>,
        request_id: u64,
    },
    Final {
        msg: Box<dyn FinalMessage + Send + Sync + 'static>,
        protocol: Protocol,
        reply_to: Option<Recipient>,
        request_id: u64,
    },
    QueryBandwidth {
        msg: Box<dyn QueryBandwidthMessage + Send + Sync + 'static>,
        protocol: Protocol,
        reply_to: Option<Recipient>,
        request_id: u64,
    },
    TopUpBandwidth {
        msg: Box<dyn TopUpMessage + Send + Sync + 'static>,
        protocol: Protocol,
        reply_to: Option<Recipient>,
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
                reply_to: Some(value.reply_to),
                request_id: value.request_id,
            },
            v1::request::AuthenticatorRequestData::Final(gateway_client) => Self::Final {
                msg: Box::new(gateway_client),
                protocol: Protocol {
                    version: value.version,
                    service_provider_type: ServiceProviderType::Authenticator,
                },
                reply_to: Some(value.reply_to),
                request_id: value.request_id,
            },
            v1::request::AuthenticatorRequestData::QueryBandwidth(peer_public_key) => {
                Self::QueryBandwidth {
                    msg: Box::new(peer_public_key),
                    protocol: Protocol {
                        version: value.version,
                        service_provider_type: ServiceProviderType::Authenticator,
                    },
                    reply_to: Some(value.reply_to),
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
                reply_to: Some(value.reply_to),
                request_id: value.request_id,
            },
            v2::request::AuthenticatorRequestData::Final(final_message) => Self::Final {
                msg: final_message,
                protocol: value.protocol,
                reply_to: Some(value.reply_to),
                request_id: value.request_id,
            },
            v2::request::AuthenticatorRequestData::QueryBandwidth(peer_public_key) => {
                Self::QueryBandwidth {
                    msg: Box::new(peer_public_key),
                    protocol: value.protocol,
                    reply_to: Some(value.reply_to),
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
                reply_to: Some(value.reply_to),
                request_id: value.request_id,
            },
            v3::request::AuthenticatorRequestData::Final(final_message) => Self::Final {
                msg: final_message,
                protocol: value.protocol,
                reply_to: Some(value.reply_to),
                request_id: value.request_id,
            },
            v3::request::AuthenticatorRequestData::QueryBandwidth(peer_public_key) => {
                Self::QueryBandwidth {
                    msg: Box::new(peer_public_key),
                    protocol: value.protocol,
                    reply_to: Some(value.reply_to),
                    request_id: value.request_id,
                }
            }
            v3::request::AuthenticatorRequestData::TopUpBandwidth(top_up_message) => {
                Self::TopUpBandwidth {
                    msg: top_up_message,
                    protocol: value.protocol,
                    reply_to: Some(value.reply_to),
                    request_id: value.request_id,
                }
            }
        }
    }
}

impl From<v4::request::AuthenticatorRequest> for AuthenticatorRequest {
    fn from(value: v4::request::AuthenticatorRequest) -> Self {
        match value.data {
            v4::request::AuthenticatorRequestData::Initial(init_message) => Self::Initial {
                msg: Box::new(init_message),
                protocol: value.protocol,
                reply_to: Some(value.reply_to),
                request_id: value.request_id,
            },
            v4::request::AuthenticatorRequestData::Final(final_message) => Self::Final {
                msg: final_message,
                protocol: value.protocol,
                reply_to: Some(value.reply_to),
                request_id: value.request_id,
            },
            v4::request::AuthenticatorRequestData::QueryBandwidth(peer_public_key) => {
                Self::QueryBandwidth {
                    msg: Box::new(peer_public_key),
                    protocol: value.protocol,
                    reply_to: Some(value.reply_to),
                    request_id: value.request_id,
                }
            }
            v4::request::AuthenticatorRequestData::TopUpBandwidth(top_up_message) => {
                Self::TopUpBandwidth {
                    msg: top_up_message,
                    protocol: value.protocol,
                    reply_to: Some(value.reply_to),
                    request_id: value.request_id,
                }
            }
        }
    }
}

impl From<v5::request::AuthenticatorRequest> for AuthenticatorRequest {
    fn from(value: v5::request::AuthenticatorRequest) -> Self {
        match value.data {
            v5::request::AuthenticatorRequestData::Initial(init_message) => Self::Initial {
                msg: Box::new(init_message),
                protocol: value.protocol,
                reply_to: None,
                request_id: value.request_id,
            },
            v5::request::AuthenticatorRequestData::Final(final_message) => Self::Final {
                msg: final_message,
                protocol: value.protocol,
                reply_to: None,
                request_id: value.request_id,
            },
            v5::request::AuthenticatorRequestData::QueryBandwidth(peer_public_key) => {
                Self::QueryBandwidth {
                    msg: Box::new(peer_public_key),
                    protocol: value.protocol,
                    reply_to: None,
                    request_id: value.request_id,
                }
            }
            v5::request::AuthenticatorRequestData::TopUpBandwidth(top_up_message) => {
                Self::TopUpBandwidth {
                    msg: top_up_message,
                    protocol: value.protocol,
                    reply_to: None,
                    request_id: value.request_id,
                }
            }
        }
    }
}
