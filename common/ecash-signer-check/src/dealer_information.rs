// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::SignerCheckError;
use nym_crypto::asymmetric::ed25519;
use nym_validator_client::nyxd::contract_traits::dkg_query_client::{
    ContractVKShare, DealerDetails, VerificationKeyShare,
};
use url::Url;

#[derive(Debug)]
pub struct RawDealerInformation {
    pub announce_address: String,
    pub owner_address: String,
    pub node_index: u64,
    pub public_key: String,
    pub verification_key_share: Option<VerificationKeyShare>,
    pub share_verified: bool,
}

impl RawDealerInformation {
    pub fn new(
        dealer_details: &DealerDetails,
        contract_share: Option<&ContractVKShare>,
    ) -> RawDealerInformation {
        RawDealerInformation {
            announce_address: dealer_details.announce_address.clone(),
            owner_address: dealer_details.address.to_string(),
            node_index: dealer_details.assigned_index,
            public_key: dealer_details.ed25519_identity.clone(),
            verification_key_share: contract_share.map(|s| s.share.clone()),
            share_verified: contract_share.map(|s| s.verified).unwrap_or(false),
        }
    }

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
            verification_key_share: self.verification_key_share.clone(),
            share_verified: self.share_verified,
        })
    }
}

#[derive(Debug)]
pub struct DealerInformation {
    pub announce_address: Url,
    pub owner_address: String,
    pub node_index: u64,
    pub public_key: ed25519::PublicKey,
    // no need to parse it into the full type as it doesn't get us anything
    pub verification_key_share: Option<VerificationKeyShare>,
    pub share_verified: bool,
}

impl From<DealerInformation> for RawDealerInformation {
    fn from(d: DealerInformation) -> Self {
        RawDealerInformation {
            announce_address: d.announce_address.to_string(),
            owner_address: d.owner_address,
            node_index: d.node_index,
            public_key: d.public_key.to_base58_string(),
            verification_key_share: d.verification_key_share,
            share_verified: d.share_verified,
        }
    }
}
