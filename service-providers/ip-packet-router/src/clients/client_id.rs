// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::fmt;

use nym_sdk::mixnet::{AnonymousSenderTag, Recipient};

use crate::error::{IpPacketRouterError, Result};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ConnectedClientId {
    NymAddress(Box<Recipient>),
    SenderTag(AnonymousSenderTag),
}

impl ConnectedClientId {
    pub(crate) fn into_nym_address(self) -> Result<Recipient> {
        match self {
            ConnectedClientId::NymAddress(nym_address) => Ok(*nym_address),
            ConnectedClientId::SenderTag(_) => Err(IpPacketRouterError::InvalidReplyTo),
        }
    }
}

impl fmt::Display for ConnectedClientId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectedClientId::NymAddress(nym_address) => write!(f, "{nym_address}"),
            ConnectedClientId::SenderTag(tag) => write!(f, "{tag}"),
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
        ConnectedClientId::SenderTag(tag)
    }
}
