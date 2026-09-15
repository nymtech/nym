// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::anchor::TrustedDigest;
use crate::error::AnchorError;
use crate::proof::{ProvenPresence, WASM_STORE_PATH, verify_wasm_store_presence};
use cosmrs::AccountId;
use nym_lthash::{DIGEST_LEN, LtHash16};
use nym_validator_client::nyxd::cosmwasm_client::contract_storage_key;
use nym_validator_client::nyxd::hash::AppHash;
use nym_validator_client::nyxd::{Height, TendermintRpcClientExt};
use nym_validator_client::rpc::types::ProvableAbciQueryResponse;

/// The raw `x/wasm` key an ICS23 proof commits to for a contract's digest accumulator:
/// `contract_key` appended to the contract's storage prefix, with no further namespacing.
/// Domain-neutral - the caller supplies whichever item key its contract writes the
/// accumulator under.
pub fn digest_storage_key(contract: &AccountId, contract_key: &[u8]) -> Vec<u8> {
    contract_storage_key(contract, contract_key)
}

/// The digest `contract` commits at `height`, proven against `trusted_app_hash`.
///
/// `digest_key` is the contract-side item key (not the raw storage key): this reconstructs
/// the raw key locally, so a malicious RPC cannot substitute a different key for the one
/// the proof is checked against.
pub async fn get_trusted_digest<C>(
    client: &C,
    contract: &AccountId,
    digest_key: &[u8],
    height: Height,
    trusted_app_hash: AppHash,
) -> Result<TrustedDigest, AnchorError>
where
    C: TendermintRpcClientExt + Send + Sync,
{
    // Reconstruct the raw key ourselves so a malicious RPC cannot substitute a
    // different key for the one we verify against.
    let key = digest_storage_key(contract, digest_key);

    // raw digest item + its ICS23 proof at H
    let res = client
        .make_raw_abci_query_with_proof(Some(WASM_STORE_PATH.to_owned()), key.clone(), Some(height))
        .await?;

    proven_contract_digest(res, &trusted_app_hash, &key, height)
}

/// The digest a proof-carrying raw read of the digest item establishes at `height`,
/// verified against the trusted app hash.
///
/// A contract only writes the item on the first entry mutation, so one with no entries yet
/// has no digest item: the read is proven ABSENT (still against the trusted app hash), which
/// means the empty accumulator - exactly the contract's own `load_digest`.
fn proven_contract_digest(
    res: ProvableAbciQueryResponse<Vec<u8>>,
    trusted_app_hash: &AppHash,
    key: &[u8],
    height: Height,
) -> Result<TrustedDigest, AnchorError> {
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
                .map_err(|v: Vec<u8>| AnchorError::BadDigestLength(v.len()))?;
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

        let trusted = proven_contract_digest(res, &app_hash, &key, height)?;

        assert_eq!(trusted.height, height);
        assert_eq!(trusted.accumulator, LtHash16::new());
        Ok(())
    }

    // The digest location is a parameter rather than a per-domain constant, so the same
    // helper must address two contracts (and two item keys) distinctly.
    #[test]
    fn the_digest_key_varies_with_both_the_contract_and_the_item_key() {
        let first = AccountId::new("n", &[0u8; 32]).unwrap();
        let second = AccountId::new("n", &[1u8; 32]).unwrap();

        // same item key, different contracts
        assert_ne!(
            digest_storage_key(&first, b"digest_state"),
            digest_storage_key(&second, b"digest_state")
        );
        // same contract, different item keys
        assert_ne!(
            digest_storage_key(&first, b"digest_state"),
            digest_storage_key(&first, b"other_state")
        );
    }

    // The contract's item key is appended to its storage prefix verbatim - no length
    // prefix, no extra namespacing - which is what lets a client rebuild the proven key
    // from the contract address and the key alone.
    #[test]
    fn the_item_key_is_appended_to_the_contract_prefix_verbatim() {
        let contract = AccountId::new("n", &[7u8; 32]).unwrap();
        let item_key = b"digest_state";

        let key = digest_storage_key(&contract, item_key);

        assert!(key.ends_with(item_key));
        assert_eq!(
            key.len(),
            digest_storage_key(&contract, b"").len() + item_key.len()
        );
    }
}
