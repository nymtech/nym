// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::anchor::TrustedDigest;
use crate::error::DirectoryClientError;
use crate::key::digest_state_key;
use crate::proof::{WASM_STORE_PATH, verify_wasm_store_membership};
use cosmrs::AccountId;
use nym_lthash::{DIGEST_LEN, LtHash16};
use nym_validator_client::nyxd::hash::AppHash;
use nym_validator_client::nyxd::{Height, TendermintRpcClientExt};

pub(crate) async fn get_trusted_directory_digest<C>(
    client: &C,
    directory_contract: &AccountId,
    height: Height,
    trusted_app_hash: AppHash,
) -> Result<TrustedDigest, DirectoryClientError>
where
    C: TendermintRpcClientExt + Send + Sync,
{
    // Reconstruct the raw key ourselves so a malicious RPC cannot substitute a
    // different key for the one we verify against.
    let key = digest_state_key(directory_contract);

    // 1. raw digest item + its ICS23 membership proof at H
    let res = client
        .make_raw_abci_query_with_proof(Some(WASM_STORE_PATH.to_owned()), key.clone(), Some(height))
        .await?;

    // 2. verify the proof against the trusted app_hash
    verify_wasm_store_membership(
        &res.proof.ops,
        trusted_app_hash.as_bytes(),
        &key,
        &res.response,
    )?;

    // 3. the proven raw value is the LtHash accumulator
    let bytes: [u8; DIGEST_LEN] = res
        .response
        .try_into()
        .map_err(|v: Vec<u8>| DirectoryClientError::BadDigestLength(v.len()))?;

    Ok(TrustedDigest {
        height,
        accumulator: LtHash16::from_bytes(&bytes),
    })
}
