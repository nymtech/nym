// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::SignerCheckError;
use nym_crypto::asymmetric::ed25519;
use nym_validator_client::nyxd::contract_traits::dkg_query_client::DealerDetails;
use url::Url;

#[derive(Debug)]
pub struct RawDealerInformation {
    pub announce_address: String,
    pub owner_address: String,
    pub node_index: u64,
    pub public_key: String,
}

impl RawDealerInformation {
    pub fn parse(&self) -> Result<DealerInformation, SignerCheckError> {
        Ok(DealerInformation {
            announce_address: self.announce_address.parse().map_err(|source| {
                SignerCheckError::InvalidDealerAddress {
                    dealer_url: self.announce_address.clone(),
                    source,
                }
            })?,
            owner_address: self.owner_address.clone(),
            node_index: self.node_index,
            public_key: self.announce_address.parse().map_err(|source| {
                SignerCheckError::InvalidDealerPubkey {
                    dealer_url: self.announce_address.clone(),
                    source,
                }
            })?,
        })
    }
}

impl From<&DealerDetails> for RawDealerInformation {
    fn from(d: &DealerDetails) -> Self {
        RawDealerInformation {
            announce_address: d.announce_address.clone(),
            owner_address: d.address.to_string(),
            node_index: d.assigned_index,
            public_key: d.ed25519_identity.clone(),
        }
    }
}

#[derive(Debug)]
pub struct DealerInformation {
    pub announce_address: Url,
    pub owner_address: String,
    pub node_index: u64,
    pub public_key: ed25519::PublicKey,
}

impl From<DealerInformation> for RawDealerInformation {
    fn from(d: DealerInformation) -> Self {
        RawDealerInformation {
            announce_address: d.announce_address.to_string(),
            owner_address: d.owner_address,
            node_index: d.node_index,
            public_key: d.public_key.to_base58_string(),
        }
    }
}
