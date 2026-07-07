// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmrs::AccountId;
use cosmrs::tendermint::chain;
use nym_lthash::LtHash16;
use nym_validator_client::nyxd::Height;
use nym_validator_client::nyxd::hash::AppHash;
use serde::{Deserialize, Serialize};

/// Domain-separation tag for [`digest_snapshot_signing_payload`], so a snapshot
/// signature can never be interpreted as a `node_signing_payload` signature (which
/// carries no tag of its own), even for a signer whose identity key is used for both.
const DIGEST_SNAPSHOT_DOMAIN_TAG: &[u8] = b"nym-directory-digest-snapshot-v1";

#[derive(Serialize, Deserialize)]
pub struct DigestSnapshot {
    /// The chain this attestation is scoped to, so a signature cannot be replayed
    /// against a different chain.
    chain_id: chain::Id,

    /// The directory contract this attestation is scoped to, so a signature cannot be
    /// replayed against a different contract instance.
    directory_contract: AccountId,

    /// The block height every other field attests to.
    height: Height,

    /// The block `app_hash` at `height` - the ICS23 fallback root for single-entry reads.
    #[serde(with = "cosmrs::tendermint::serializers::apphash")]
    app_hash: AppHash,

    /// The directory contract's LtHash accumulator at `height`.
    accumulator: LtHash16,

    /// Hash over the current `NodeId -> ed25519 identity` mapping at `height`
    /// (see [`crate::verify::node_identities_hash`]).
    node_identities_hash: [u8; 32],
}

impl DigestSnapshot {
    pub(crate) fn signing_payload(&self) -> Vec<u8> {
        digest_snapshot_signing_payload(
            self.chain_id.as_ref(),
            &self.directory_contract,
            self.height,
            &self.app_hash,
            &self.accumulator,
            &self.node_identities_hash,
        )
    }
}

/// Append `bytes` prefixed with its u32 little-endian length, so adjacent
/// variable-length fields cannot be confused with one another. Mirrors
/// `nym_directory_contract_common::helpers::push_len_prefixed`'s framing (private to
/// that crate); reproduced here since it is the only encoder in this crate that needs it.
fn push_len_prefixed(buf: &mut Vec<u8>, bytes: &[u8]) {
    buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    buf.extend_from_slice(bytes);
}

/// The exact bytes a nym-api signs when attesting a directory snapshot: the block
/// `app_hash`, the directory's LtHash `accumulator`, and a hash over the current
/// `NodeId -> ed25519 identity` mapping (see
/// [`crate::verify::node_identities_hash`]), all bound to a chain-id, contract address,
/// and height so a signature cannot be replayed across chains, contract instances, or
/// heights.
pub(crate) fn digest_snapshot_signing_payload(
    chain_id: &str,
    contract: &AccountId,
    height: Height,
    app_hash: &AppHash,
    accumulator: &LtHash16,
    node_identities_hash: &[u8; 32],
) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(DIGEST_SNAPSHOT_DOMAIN_TAG);
    push_len_prefixed(&mut buf, chain_id.as_bytes());
    push_len_prefixed(&mut buf, &contract.to_bytes());
    buf.extend_from_slice(&height.value().to_le_bytes());
    push_len_prefixed(&mut buf, app_hash.as_bytes());
    push_len_prefixed(&mut buf, &accumulator.to_bytes());
    buf.extend_from_slice(node_identities_hash);
    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn contract() -> AccountId {
        AccountId::from_str("n17srjznxl9dvzdkpwpw24gg668wc73val88a6m5ajg6ankwvz9wtst0cznr").unwrap()
    }

    fn other_contract() -> AccountId {
        AccountId::from_str("n1jw6mp7d5xqc7w6xm79lha27glmd0vdt3l9artf").unwrap()
    }

    fn app_hash(byte: u8) -> AppHash {
        AppHash::try_from(vec![byte; 32]).unwrap()
    }

    #[test]
    fn digest_snapshot_payload_is_deterministic_and_field_sensitive() {
        let contract = contract();
        let acc = LtHash16::new();
        let node_hash = [9u8; 32];
        let base = digest_snapshot_signing_payload(
            "nyx-testnet",
            &contract,
            Height::from(100u32),
            &app_hash(1),
            &acc,
            &node_hash,
        );
        assert_eq!(
            base,
            digest_snapshot_signing_payload(
                "nyx-testnet",
                &contract,
                Height::from(100u32),
                &app_hash(1),
                &acc,
                &node_hash,
            )
        );
        assert_ne!(
            base,
            digest_snapshot_signing_payload(
                "nyx-mainnet",
                &contract,
                Height::from(100u32),
                &app_hash(1),
                &acc,
                &node_hash,
            )
        );
        assert_ne!(
            base,
            digest_snapshot_signing_payload(
                "nyx-testnet",
                &other_contract(),
                Height::from(100u32),
                &app_hash(1),
                &acc,
                &node_hash,
            )
        );
        assert_ne!(
            base,
            digest_snapshot_signing_payload(
                "nyx-testnet",
                &contract,
                Height::from(101u32),
                &app_hash(1),
                &acc,
                &node_hash,
            )
        );
        assert_ne!(
            base,
            digest_snapshot_signing_payload(
                "nyx-testnet",
                &contract,
                Height::from(100u32),
                &app_hash(2),
                &acc,
                &node_hash,
            )
        );
        let mut other_acc = LtHash16::new();
        other_acc.add(b"leaf");
        assert_ne!(
            base,
            digest_snapshot_signing_payload(
                "nyx-testnet",
                &contract,
                Height::from(100u32),
                &app_hash(1),
                &other_acc,
                &node_hash,
            )
        );
        let mut other_node_hash = node_hash;
        other_node_hash[0] ^= 1;
        assert_ne!(
            base,
            digest_snapshot_signing_payload(
                "nyx-testnet",
                &contract,
                Height::from(100u32),
                &app_hash(1),
                &acc,
                &other_node_hash,
            )
        );
    }

    #[test]
    fn digest_snapshot_payload_length_prefix_disambiguates() {
        // (chain-id "ab", contract-derived bytes) framing must not let adjacent
        // variable-length fields bleed into one another; exercised here via chain-id
        // vs. the contract's encoded bytes rather than two strings of our own choosing,
        // since `contract` is a real bech32 address.
        let acc = LtHash16::new();
        let node_hash = [0u8; 32];
        assert_ne!(
            digest_snapshot_signing_payload(
                "ab",
                &contract(),
                Height::from(0u32),
                &app_hash(0),
                &acc,
                &node_hash,
            ),
            digest_snapshot_signing_payload(
                "a",
                &other_contract(),
                Height::from(0u32),
                &app_hash(0),
                &acc,
                &node_hash,
            ),
        );
    }

    #[test]
    fn digest_snapshot_payload_is_domain_tagged() {
        let payload = digest_snapshot_signing_payload(
            "chain",
            &contract(),
            Height::from(1u32),
            &app_hash(7),
            &LtHash16::new(),
            &[7u8; 32],
        );
        assert!(payload.starts_with(DIGEST_SNAPSHOT_DOMAIN_TAG));

        // a representative node-entry payload never starts with the snapshot's domain
        // tag, so the two signature domains cannot be confused
        let node_payload = nym_directory_contract_common::node_signing_payload(1, "x", 1, b"y");
        assert!(!node_payload.starts_with(DIGEST_SNAPSHOT_DOMAIN_TAG));
    }
}
