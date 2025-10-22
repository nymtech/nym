// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only
use nym_sphinx::addressing::clients::Recipient;
use nym_sphinx::addressing::nodes::NodeIdentity;
use nym_validator_client::models::NymNodeData;

use crate::{Error, error::Result};

#[derive(Debug, Copy, Clone)]
pub struct IpPacketRouterAddress(Recipient);

impl IpPacketRouterAddress {
    pub fn try_from_base58_string(ip_packet_router_nym_address: &str) -> Result<Self> {
        Ok(Self(
            Recipient::try_from_base58_string(ip_packet_router_nym_address).map_err(|source| {
                Error::RecipientFormattingError {
                    address: ip_packet_router_nym_address.to_string(),
                    source,
                }
            })?,
        ))
    }

    pub fn try_from_described_gateway(gateway: &NymNodeData) -> Result<Self> {
        let address = gateway
            .clone()
            .ip_packet_router
            .map(|ipr| ipr.address)
            .ok_or(Error::MissingIpPacketRouterAddress)?;
        Ok(Self(Recipient::try_from_base58_string(&address).map_err(
            |source| Error::RecipientFormattingError { address, source },
        )?))
    }

    pub fn gateway(&self) -> NodeIdentity {
        self.0.gateway()
    }
}

impl std::fmt::Display for IpPacketRouterAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Recipient> for IpPacketRouterAddress {
    fn from(recipient: Recipient) -> Self {
        Self(recipient)
    }
}

impl From<IpPacketRouterAddress> for Recipient {
    fn from(ipr_address: IpPacketRouterAddress) -> Self {
        ipr_address.0
    }
}
