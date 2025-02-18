// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt;

use nym_ip_packet_requests::v8::request::SentBy;
use nym_sdk::mixnet::{AnonymousSenderTag, Recipient};

use crate::error::{IpPacketRouterError, Result};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ConnectedClientId {
    AnonymousSenderTag(AnonymousSenderTag),
    NymAddress(Box<Recipient>),
}

impl ConnectedClientId {
    pub(crate) fn into_nym_address(self) -> Result<Recipient> {
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

impl TryFrom<(SentBy, Option<AnonymousSenderTag>)> for ConnectedClientId {
    type Error = IpPacketRouterError;

    fn try_from((sent_by, sender_tag): (SentBy, Option<AnonymousSenderTag>)) -> Result<Self> {
        match sent_by {
            SentBy::NymAddress(nym_address) => Ok(ConnectedClientId::NymAddress(nym_address)),
            SentBy::AnonymousSenderTag => sender_tag
                .map(ConnectedClientId::AnonymousSenderTag)
                .ok_or(IpPacketRouterError::InvalidReplyTo),
        }
    }
}
