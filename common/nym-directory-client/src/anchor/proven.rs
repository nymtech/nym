// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::anchor::{AnchorError, DirectoryTrustAnchor, TrustedDigest, WASM_STORE_PATH};
use crate::key::digest_state_key;
use crate::proof::verify_wasm_store_membership;
use async_trait::async_trait;
use cosmrs::AccountId;
use nym_lthash::{DIGEST_LEN, LtHash16};
use nym_validator_client::nyxd::{Height, TendermintRpcClientExt};

/// Proven anchor: proves the on-chain `digest_state` item via an ICS23 membership
/// proof against the block `app_hash`. Phase 1a takes the `app_hash` from a configured
/// RPC's `header[H+1]`; a light client can replace that source behind the same trait.
pub struct ProvenTrustAnchor<C> {
    client: C,
    directory_contract: AccountId,
}

impl<C> ProvenTrustAnchor<C> {
    pub fn new(client: C, directory_contract: AccountId) -> Self {
        Self {
            client,
            directory_contract,
        }
    }
}

#[async_trait]
impl<C> DirectoryTrustAnchor for ProvenTrustAnchor<C>
where
    C: TendermintRpcClientExt + Send + Sync,
{
    async fn trusted_digest(&self, height: Height) -> Result<TrustedDigest, AnchorError> {
        // Reconstruct the raw key ourselves so a malicious RPC cannot substitute a
        // different key for the one we verify against.
        let key = digest_state_key(&self.directory_contract);

        // 1. raw digest item + its ICS23 membership proof at H
        let res = self
            .client
            .make_raw_abci_query_with_proof(
                Some(WASM_STORE_PATH.to_owned()),
                key.clone(),
                Some(height),
            )
            .await
            .map_err(|e| AnchorError::Query(e.to_string()))?;

        // 2. the app_hash committing state at H lives in header[H+1] (CometBFT off-by-one)
        let next: Height = (height.value() as u32 + 1).into();
        let app_hash = self
            .client
            .header(next)
            .await
            .map_err(|e| AnchorError::Query(e.to_string()))?
            .header
            .app_hash;

        // 3. verify the proof against the trusted app_hash
        verify_wasm_store_membership(&res.proof.ops, app_hash.as_bytes(), &key, &res.response)?;

        // 4. the proven raw value is the LtHash accumulator
        let bytes: [u8; DIGEST_LEN] = res
            .response
            .try_into()
            .map_err(|v: Vec<u8>| AnchorError::BadDigestLength(v.len()))?;

        Ok(TrustedDigest {
            height,
            accumulator: LtHash16::from_bytes(&bytes),
        })
    }
}
