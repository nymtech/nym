// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::contract_traits::{DkgQueryClient, PagedDkgQueryClient};
use crate::nyxd::error::NyxdError;
use nym_coconut_dkg_common::types::{EpochId, NodeIndex};
use nym_coconut_dkg_common::verification_key::ContractVKShare;
use nym_compact_ecash::error::CompactEcashError;
use nym_compact_ecash::{Base58, VerificationKeyAuth};
use std::fmt::{Display, Formatter};
use thiserror::Error;
use tracing::warn;
use url::Url;

// TODO: it really doesn't feel like this should live in this crate.
#[derive(Clone)]
pub struct EcashApiClient {
    pub api_client: nym_http_api_client::Client,
    pub verification_key: VerificationKeyAuth,
    pub node_id: NodeIndex,
    pub cosmos_address: cosmrs::AccountId,
}

impl Display for EcashApiClient {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[id: {}] {} @ ({})",
            self.node_id,
            self.cosmos_address,
            self.api_client
                .base_urls()
                .iter()
                .map(|url| url.to_string())
                .collect::<Vec<String>>()
                .join(", ")
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

    #[error("failed to create API client: {0}")]
    ClientError(String),

    #[error("the provided account address is malformed: {source}")]
    MalformedAccountAddress {
        #[from]
        source: cosmrs::ErrorReport,
    },

    #[error("nym api error")]
    NymApi {
        #[from]
        source: crate::ValidatorClientError,
    },
}

impl TryFrom<ContractVKShare> for EcashApiClient {
    type Error = EcashApiError;

    fn try_from(share: ContractVKShare) -> Result<Self, Self::Error> {
        if !share.verified {
            return Err(EcashApiError::UnverifiedShare);
        }

        let url_address = Url::parse(&share.announce_address)?;

        // The NymApiClient constructed here uses the default (hickory DoT/DoH) resolver because
        // this EcashApiClient is used by both client and non-client applications.
        //
        // In non-client applications this resolver can cause warning logs about H2 connection
        // failure. This indicates that the long lived https connection was closed by the remote
        // peer and the resolver will have to reconnect. It should not impact actual functionality
        let api_client = nym_http_api_client::Client::builder(url_address)
            .map_err(|e| EcashApiError::ClientError(e.to_string()))?
            .build()
            .map_err(|e| EcashApiError::ClientError(e.to_string()))?;

        Ok(EcashApiClient {
            api_client,
            verification_key: VerificationKeyAuth::try_from_bs58(&share.share)?,
            node_id: share.node_index,
            cosmos_address: share.owner.as_str().parse()?,
        })
    }
}

/// Turn an epoch's key shares into usable API clients, skipping any share that can't be
/// used and logging why.
///
/// A single bad share must not take the epoch down with it. Beyond the obvious case of a
/// share that was never verified, the contract stores `announce_address` verbatim and
/// share validation never looks at it, so a share can be marked verified on chain and
/// still fail to convert here. Failing the whole batch would then deny every caller
/// signer discovery for that epoch, recoverable only if the offending dealer updates its
/// own announce address.
///
/// Callers must check the result still meets the epoch threshold - skipping is only safe
/// because too few usable signers is a condition they detect themselves.
pub fn usable_ecash_api_clients(shares: Vec<ContractVKShare>) -> Vec<EcashApiClient> {
    let mut clients = Vec::with_capacity(shares.len());

    for share in shares {
        let owner = share.owner.clone();
        let epoch_id = share.epoch_id;

        match EcashApiClient::try_from(share) {
            Ok(client) => clients.push(client),
            Err(err) => {
                warn!("ignoring the key share of {owner} for epoch {epoch_id}: {err}")
            }
        }
    }

    clients
}

pub async fn all_ecash_api_clients<C>(
    client: &C,
    epoch_id: EpochId,
) -> Result<Vec<EcashApiClient>, EcashApiError>
where
    C: DkgQueryClient,
{
    Ok(usable_ecash_api_clients(
        client.get_all_verification_key_shares(epoch_id).await?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nyxd::contract_traits::dkg_query_client::{DkgQueryMsg, PagedVKSharesResponse};
    use async_trait::async_trait;
    use cosmrs::AccountId;
    use cosmwasm_std::Addr;
    use nym_compact_ecash::ttp_keygen;
    use serde::{Deserialize, Serialize};

    /// Serves a fixed set of shares; every other query is out of scope for these tests.
    struct StubDkgQueryClient {
        shares: Vec<ContractVKShare>,
    }

    fn respond<S, T>(value: S) -> Result<T, NyxdError>
    where
        S: Serialize,
        for<'a> T: Deserialize<'a>,
    {
        let raw = serde_json::to_vec(&value).expect("failed to serialise the stub response");
        Ok(serde_json::from_slice(&raw).expect("the stub returned the wrong response type"))
    }

    #[async_trait]
    impl DkgQueryClient for StubDkgQueryClient {
        async fn query_dkg_contract<T>(&self, query: DkgQueryMsg) -> Result<T, NyxdError>
        where
            for<'a> T: Deserialize<'a>,
        {
            match query {
                DkgQueryMsg::GetVerificationKeys { epoch_id, .. } => {
                    respond(PagedVKSharesResponse {
                        shares: self
                            .shares
                            .iter()
                            .filter(|share| share.epoch_id == epoch_id)
                            .cloned()
                            .collect(),
                        per_page: self.shares.len(),
                        start_next_after: None,
                    })
                }
                other => panic!("the stub does not serve {other:?}"),
            }
        }
    }

    fn share(index: NodeIndex, key: &VerificationKeyAuth, verified: bool) -> ContractVKShare {
        let owner = AccountId::new("n", &[index as u8; 32]).unwrap();

        ContractVKShare {
            share: key.to_bs58(),
            announce_address: format!("http://localhost:{}", 8080 + index),
            node_index: index,
            owner: Addr::unchecked(owner.to_string()),
            epoch_id: 0,
            verified,
        }
    }

    #[tokio::test]
    async fn unverified_shares_are_skipped_rather_than_failing_the_whole_epoch() {
        let keys = ttp_keygen(2, 3).unwrap();

        // one dealer never got its share verified on chain - it missed the finalisation
        // window, say. the epoch still concluded, and the other two shares are usable.
        let shares = vec![
            share(1, &keys[0].verification_key(), true),
            share(2, &keys[1].verification_key(), false),
            share(3, &keys[2].verification_key(), true),
        ];

        let client = StubDkgQueryClient { shares };
        let clients = all_ecash_api_clients(&client, 0)
            .await
            .expect("a single unverified share must not brick signer discovery for the epoch");

        // the verified signers remain discoverable, so callers can still apply their own
        // threshold check - today the unverified share aborts the conversion before any
        // threshold is ever considered
        assert_eq!(clients.len(), 2);
        assert_eq!(
            clients.iter().map(|c| c.node_id).collect::<Vec<_>>(),
            vec![1, 3]
        );
    }
}
