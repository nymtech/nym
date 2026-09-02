// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::anchor::TrustedDigest;
use crate::error::DirectoryClientError;
use crate::key::digest_state_key;
use crate::proof::{ProvenPresence, WASM_STORE_PATH, verify_wasm_store_presence};
use cosmrs::AccountId;
use nym_lthash::{DIGEST_LEN, LtHash16};
use nym_validator_client::nyxd::hash::AppHash;
use nym_validator_client::nyxd::{Height, TendermintRpcClientExt};
use nym_validator_client::rpc::types::ProvableAbciQueryResponse;

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

    // raw digest item + its ICS23 proof at H
    let res = client
        .make_raw_abci_query_with_proof(Some(WASM_STORE_PATH.to_owned()), key.clone(), Some(height))
        .await?;

    proven_directory_digest(res, &trusted_app_hash, &key, height)
}

/// The digest a proof-carrying raw read of the `DIGEST_STATE` item establishes at `height`,
/// verified against the trusted app hash.
///
/// The contract only writes the item on the first entry mutation, so a directory with no
/// entries yet has no digest item: the read is proven ABSENT (still against the trusted app
/// hash), which means the empty accumulator - exactly the contract's own `load_digest`.
fn proven_directory_digest(
    res: ProvableAbciQueryResponse<Vec<u8>>,
    trusted_app_hash: &AppHash,
    key: &[u8],
    height: Height,
) -> Result<TrustedDigest, DirectoryClientError> {
    // 1. verify the proof against the trusted app_hash, whichever shape it has
    let presence = verify_wasm_store_presence(
        &res.proof.ops,
        trusted_app_hash.as_bytes(),
        key,
        &res.response,
    )?;

    // 2. the proven raw value is the LtHash accumulator (or there is none yet)
    let accumulator = match presence {
        ProvenPresence::Absent => LtHash16::new(),
        ProvenPresence::Present => {
            let bytes: [u8; DIGEST_LEN] = res
                .response
                .try_into()
                .map_err(|v: Vec<u8>| DirectoryClientError::BadDigestLength(v.len()))?;
            LtHash16::from_bytes(&bytes)
        }
    };

    Ok(TrustedDigest {
        height,
        accumulator,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proof::tests::{LiveNonMembershipFixture, live_non_membership_fixture};

    // The contract only writes the digest item on the first entry mutation, so a directory
    // with no entries yet has NO digest item on chain: the raw read is proven ABSENT. The
    // reader must treat that as the empty accumulator (mirroring the contract's own
    // `load_digest`), not as a verification failure.
    #[test]
    fn a_proven_absent_digest_item_is_the_empty_accumulator() -> anyhow::Result<()> {
        let LiveNonMembershipFixture {
            height,
            res,
            key,
            app_hash,
            ..
        } = live_non_membership_fixture();

        let trusted = proven_directory_digest(res, &app_hash, &key, height)?;

        assert_eq!(trusted.height, height);
        assert_eq!(trusted.accumulator, LtHash16::new());
        Ok(())
    }
}
