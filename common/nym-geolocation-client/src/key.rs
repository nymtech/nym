// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The raw `x/wasm` storage key an ICS23 proof commits to for the geolocation contract's
//! digest accumulator.
//!
//! The generic `0x03 || canonical_addr || contract_key` layout lives in
//! [`nym_validator_client::nyxd::cosmwasm_client::contract_storage_key`]; this module names
//! the geolocation contract's own item key.

use cosmrs::AccountId;
use nym_contract_anchor::anchor::helpers::digest_storage_key;
use nym_geolocation_contract_common::constants::storage_keys;

/// The geolocation contract's own item key for its LtHash digest accumulator, un-prefixed.
/// This is what a trust anchor is constructed with: the anchor prefixes the contract itself,
/// so that it reconstructs the proven key locally rather than trusting an RPC's.
pub fn digest_item_key() -> Vec<u8> {
    storage_keys::DIGEST_STATE.as_bytes().to_vec()
}

/// Raw key for the geolocation contract's on-chain LtHash digest accumulator.
pub fn digest_state_key(contract: &AccountId) -> Vec<u8> {
    digest_storage_key(contract, storage_keys::DIGEST_STATE.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract(seed: u8) -> AccountId {
        #[allow(clippy::unwrap_used)]
        AccountId::new("n", &[seed; 32]).unwrap()
    }

    /// The contract writes the accumulator with `store.set(DIGEST_STATE.as_bytes(), ..)`
    /// rather than through a `cw-storage-plus` `Item`, so the proven key is exactly those
    /// bytes appended to the contract's storage prefix. Built here from wasmd's layout by
    /// hand rather than by calling the helper again, so this actually pins the encoding:
    /// if a length prefix or a namespace ever crept in, this fails.
    #[test]
    fn the_digest_key_is_the_item_key_appended_to_the_contract_prefix() {
        let contract = contract(7);

        // 0x03 is wasmd's ContractStorePrefix
        let mut expected = vec![0x03u8];
        expected.extend_from_slice(&contract.to_bytes());
        expected.extend_from_slice(b"digest_state");

        assert_eq!(digest_state_key(&contract), expected);
    }

    /// The item key carries no contract binding of its own - the anchor supplies that - so
    /// it must be exactly the contract's published constant and nothing more.
    #[test]
    fn the_item_key_is_the_bare_constant() {
        assert_eq!(digest_item_key(), b"digest_state".to_vec());
    }

    /// Two deployments of the contract must not share a proven key, or a proof against one
    /// would verify against the other.
    #[test]
    fn the_key_binds_the_contract_address() {
        assert_ne!(
            digest_state_key(&contract(0)),
            digest_state_key(&contract(1))
        );
    }
}
