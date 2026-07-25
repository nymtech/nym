// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! A `DkgQueryClient` test double (design D1): answers the verification-key
//! share and threshold queries from fabricated data, so the REAL
//! `NyxdGlobalDataFetcher` resolves its ecash-API list to local mock servers
//! through its real discovery path — no chain involved.
//!
//! `DkgQueryClient` has exactly one required method (`query_dkg_contract`);
//! every discovery entry point is a default method over it. The double matches
//! the query variants discovery actually issues and fails loud on anything
//! else, so silent contract-protocol drift surfaces immediately.

#![allow(clippy::expect_used, clippy::unwrap_used, dead_code)]

use async_trait::async_trait;
use nym_coconut_dkg_common::types::EpochId as DkgEpochId;
use nym_coconut_dkg_common::verification_key::{ContractVKShare, PagedVKSharesResponse};
use nym_validator_client::nyxd::contract_traits::dkg_query_client::DkgQueryMsg;
use nym_validator_client::nyxd::contract_traits::DkgQueryClient;
use nym_validator_client::nyxd::error::NyxdError;
use serde::Deserialize;

/// A fabricated DKG state: the signer set (each announcing a mock server URL)
/// plus the epoch threshold.
pub struct FakeDkg {
    pub shares: Vec<ContractVKShare>,
    pub threshold: u64,
}

/// Syntactically valid nyx bech32 owner addresses (discovery parses `owner`
/// into a cosmrs `AccountId`, so it must decode; the value is otherwise
/// unused by these tests). Cycled when more signers than entries are built.
const OWNERS: &[&str] = &[
    "n168t66xf3qp2pv9x4ufv42z08d4jwvg0fa9zhzf",
    "n1mc5dkv2mkp8ae5numtcqycq7n8xdq5s8mqhnt5",
    "n1qk2cqty0mlu6s8ewqtuf5e6fna0yp3nks5ttlr",
];

impl FakeDkg {
    /// Build a signer set from `(announce_address, vk_share_bs58)` pairs, all
    /// verified (unverified shares are filtered by the real discovery).
    pub fn new(
        epoch_id: DkgEpochId,
        threshold: u64,
        signers: impl IntoIterator<Item = (String, String)>,
    ) -> Self {
        let shares = signers
            .into_iter()
            .enumerate()
            .map(|(i, (announce_address, share))| ContractVKShare {
                share,
                announce_address,
                node_index: i as u64 + 1,
                owner: cosmwasm_std::Addr::unchecked(OWNERS[i % OWNERS.len()]),
                epoch_id,
                verified: true,
            })
            .collect();
        FakeDkg { shares, threshold }
    }
}

#[async_trait]
impl DkgQueryClient for FakeDkg {
    async fn query_dkg_contract<T>(&self, query: DkgQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        let response = match query {
            DkgQueryMsg::GetVerificationKeys { epoch_id, .. } => {
                let shares: Vec<ContractVKShare> = self
                    .shares
                    .iter()
                    .filter(|s| s.epoch_id == epoch_id)
                    .cloned()
                    .collect();
                serde_json::to_value(PagedVKSharesResponse {
                    shares,
                    per_page: 100,
                    // single page: discovery's paging loop terminates here
                    start_next_after: None,
                })
            }
            DkgQueryMsg::GetEpochThreshold { .. } | DkgQueryMsg::GetCurrentEpochThreshold {} => {
                serde_json::to_value(Some(self.threshold))
            }
            // Fail loud: an unexpected query means discovery's contract
            // protocol changed under us — surface it, don't shrug.
            other => {
                return Err(NyxdError::SerializationError(format!(
                    "FakeDkg: unexpected DKG query {other:?}"
                )))
            }
        }
        .map_err(|e| NyxdError::SerializationError(format!("FakeDkg response: {e}")))?;
        serde_json::from_value(response)
            .map_err(|e| NyxdError::DeserializationError(format!("FakeDkg response: {e}")))
    }
}
