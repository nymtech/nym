// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::service_providers::ip_packet_router::error::IpPacketRouterError;
use nym_sdk::mixnet::{AnonymousSenderTag, Recipient};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ConnectedClientId {
    AnonymousSenderTag(AnonymousSenderTag),
    NymAddress(Box<Recipient>),
}

impl ConnectedClientId {
    pub(crate) fn into_nym_address(self) -> Result<Recipient, IpPacketRouterError> {
        match self {
            ConnectedClientId::NymAddress(nym_address) => Ok(*nym_address),
            ConnectedClientId::AnonymousSenderTag(_) => Err(IpPacketRouterError::InvalidReplyTo),
        }
    }
}

impl fmt::Display for ConnectedClientId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectedClientId::NymAddress(nym_address) => write!(f, "{nym_address}"),
            ConnectedClientId::AnonymousSenderTag(tag) => write!(f, "{tag}"),
        }
    }
}

impl From<Recipient> for ConnectedClientId {
    fn from(nym_address: Recipient) -> Self {
        ConnectedClientId::NymAddress(Box::new(nym_address))
    }
}

impl From<AnonymousSenderTag> for ConnectedClientId {
    fn from(tag: AnonymousSenderTag) -> Self {
        ConnectedClientId::AnonymousSenderTag(tag)
    }
}
