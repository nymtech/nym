// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::contract_traits::{DkgQueryClient, PagedDkgQueryClient};
use crate::nyxd::error::NyxdError;
use crate::NymApiClient;
use nym_coconut_dkg_common::types::{EpochId, NodeIndex};
use nym_coconut_dkg_common::verification_key::ContractVKShare;
use nym_compact_ecash::error::CompactEcashError;
use nym_compact_ecash::{Base58, VerificationKeyAuth};
use std::fmt::{Display, Formatter};
use thiserror::Error;
use url::Url;

// TODO: it really doesn't feel like this should live in this crate.
#[derive(Clone)]
pub struct EcashApiClient {
    pub api_client: NymApiClient,
    pub verification_key: VerificationKeyAuth,
    pub node_id: NodeIndex,
    pub cosmos_address: cosmrs::AccountId,
}

impl Display for EcashApiClient {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[id: {}] {} @ {}",
            self.node_id,
            self.cosmos_address,
            self.api_client.api_url()
        )
    }
}

// TODO: this should be using the coconut error
// (which is in different crate; perhaps this client should be moved there?)

#[derive(Debug, Error)]
pub enum EcashApiError {
    // TODO: ask @BN whether this is a correct error message
    #[error("the provided key share hasn't been verified")]
    UnverifiedShare,

    #[error("failed to query the contract: {source}")]
    ContractQueryFailure {
        #[from]
        source: NyxdError,
    },

    #[error("the provided announce address is malformed: {source}")]
    MalformedAnnounceAddress {
        #[from]
        source: url::ParseError,
    },

    #[error("the provided verification key is malformed: {source}")]
    MalformedVerificationKey {
        #[from]
        source: CompactEcashError,
    },

    #[error("the provided account address is malformed: {source}")]
    MalformedAccountAddress {
        #[from]
        source: cosmrs::ErrorReport,
    },
}

impl TryFrom<ContractVKShare> for EcashApiClient {
    type Error = EcashApiError;

    fn try_from(share: ContractVKShare) -> Result<Self, Self::Error> {
        if !share.verified {
            return Err(EcashApiError::UnverifiedShare);
        }

        let url_address = Url::parse(&share.announce_address)?;

        Ok(EcashApiClient {
            api_client: NymApiClient::new(url_address),
            verification_key: VerificationKeyAuth::try_from_bs58(&share.share)?,
            node_id: share.node_index,
            cosmos_address: share.owner.as_str().parse()?,
        })
    }
}

pub async fn all_ecash_api_clients<C>(
    client: &C,
    epoch_id: EpochId,
) -> Result<Vec<EcashApiClient>, EcashApiError>
where
    C: DkgQueryClient + Sync + Send,
{
    // TODO: this will error out if there's an invalid share out there. is that what we want?
    client
        .get_all_verification_key_shares(epoch_id)
        .await?
        .into_iter()
        .map(TryInto::try_into)
        .collect::<Result<Vec<_>, _>>()

    // ... if not, let's switch to the below:
    // client
    //     .get_all_verification_key_shares(epoch_id)
    //     .await?
    //     .into_iter()
    //     .filter_map(TryInto::try_into)
    //     .collect::<Result<Vec<_>, _>>()
}
