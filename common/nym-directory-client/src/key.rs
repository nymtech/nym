// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Raw `x/wasm` storage keys for the directory contract that an ICS23 proof commits to.
//!
//! The generic `0x03 || canonical_addr || contract_key` layout lives in
//! [`nym_validator_client::nyxd::cosmwasm_client::contract_storage_key`] (shared with
//! `query_contract_raw_with_proof`); this module adds the directory-specific keys.

use cosmrs::AccountId;
use nym_directory_contract_common::constants::storage_keys;
use nym_mixnet_contract_common::NodeId;
use nym_validator_client::nyxd::cosmwasm_client::contract_storage_key;

/// Raw key for the directory contract's on-chain LtHash digest accumulator (`Item`).
pub fn digest_state_key(contract: &AccountId) -> Vec<u8> {
    contract_storage_key(contract, storage_keys::DIGEST_STATE.as_bytes())
}

/// Raw `x/wasm` key an ICS23 membership proof commits to for a node entry `(node_id,
/// label)`, reproducing the contract's `StoredNodeEntries` `cw-storage-plus` `Path`.
pub fn node_entry_key(contract: &AccountId, node_id: NodeId, label: &str) -> Vec<u8> {
    contract_storage_key(contract, &node_entry_contract_key(node_id, label))
}

/// Raw `x/wasm` key an ICS23 membership proof commits to for a curated entry, reproducing
/// the contract's `StoredCuratedEntries` `cw-storage-plus` `Path`.
pub fn curated_entry_key(contract: &AccountId, key: &str) -> Vec<u8> {
    contract_storage_key(contract, &curated_entry_contract_key(key))
}

/// The `cw-storage-plus` `Path` bytes for the `(NodeId, String)` node-entry map (the
/// bytes after `0x03 || addr`): `len(ns) || ns || len(node_id_be) || node_id_be || label`.
/// The composite key length-prefixes every component except the last (the `label`), and
/// `NodeId` (a `u32`) is encoded big-endian - matching `cw-storage-plus`'s `PrimaryKey`
/// for `(u32, String)` (cross-checked in tests).
fn node_entry_contract_key(node_id: NodeId, label: &str) -> Vec<u8> {
    let mut key = Vec::new();
    push_cw_length_prefixed(&mut key, storage_keys::NODE_ENTRIES.as_bytes());
    push_cw_length_prefixed(&mut key, &node_id.to_be_bytes());
    key.extend_from_slice(label.as_bytes());
    key
}

/// The `cw-storage-plus` `Path` bytes for the single-`String` curated-entry map:
/// `len(ns) || ns || key`.
fn curated_entry_contract_key(key: &str) -> Vec<u8> {
    let mut out = Vec::new();
    push_cw_length_prefixed(&mut out, storage_keys::CURATED_ENTRIES.as_bytes());
    out.extend_from_slice(key.as_bytes());
    out
}

/// Append `bytes` prefixed with its length as a 2-byte big-endian value - the framing
/// `cw-storage-plus` uses for every namespace / non-final composite-key component.
fn push_cw_length_prefixed(buf: &mut Vec<u8>, bytes: &[u8]) {
    // cw-storage-plus asserts the length fits in a u16; namespaces and keys are short.
    buf.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
    buf.extend_from_slice(bytes);
}

#[cfg(test)]
mod tests {
    use super::*;
    use cw_storage_plus::{Key, Path, PrimaryKey};

    fn contract() -> AccountId {
        "n17srjznxl9dvzdkpwpw24gg668wc73val88a6m5ajg6ankwvz9wtst0cznr"
            .parse()
            .unwrap()
    }

    /// The exact `cw-storage-plus` `Path` storage key the contract writes for a composite
    /// primary key under `namespace` - the authority we reproduce.
    fn cw_path<K: PrimaryKey<'static>>(namespace: &[u8], key: K) -> Vec<u8> {
        let parts = key.key();
        Path::<Vec<u8>>::new(
            namespace,
            &parts.iter().map(Key::as_ref).collect::<Vec<_>>(),
        )
        .to_vec()
    }

    #[test]
    fn digest_state_key_wraps_the_digest_storage_key() {
        let contract = contract();
        let key = digest_state_key(&contract);

        // 0x03 prefix, then the 32-byte address, then the raw "digest_state" key
        assert_eq!(key[0], 0x03);
        assert_eq!(&key[1..33], contract.to_bytes().as_slice());
        assert_eq!(&key[33..], storage_keys::DIGEST_STATE.as_bytes());
    }

    #[test]
    fn node_entry_key_matches_the_contract_storage_path() {
        let contract = contract();
        let key = node_entry_key(&contract, 7, "sphinx_key");

        assert_eq!(key[0], 0x03);
        assert_eq!(&key[1..33], contract.to_bytes().as_slice());
        // the contract key equals cw-storage-plus's Path for (u32, String) under the namespace
        assert_eq!(
            &key[33..],
            cw_path(
                storage_keys::NODE_ENTRIES.as_bytes(),
                (7u32, "sphinx_key".to_string())
            )
        );
    }

    #[test]
    fn curated_entry_key_matches_the_contract_storage_path() {
        let contract = contract();
        let key = curated_entry_key(&contract, "nym-api-1");

        assert_eq!(key[0], 0x03);
        assert_eq!(&key[1..33], contract.to_bytes().as_slice());
        // the contract key equals cw-storage-plus's Path for a single String under the namespace
        assert_eq!(
            &key[33..],
            cw_path(
                storage_keys::CURATED_ENTRIES.as_bytes(),
                "nym-api-1".to_string()
            )
        );
    }
}
