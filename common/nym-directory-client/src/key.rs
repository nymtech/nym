// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Raw `x/wasm` storage keys for the directory contract that an ICS23 proof commits to.
//!
//! The generic `0x03 || canonical_addr || contract_key` layout lives in
//! [`nym_validator_client::nyxd::cosmwasm_client::contract_storage_key`] (shared with
//! `query_contract_raw_with_proof`); this module adds the directory-specific keys.

use cosmrs::AccountId;
use nym_directory_contract_common::constants::storage_keys;
use nym_validator_client::nyxd::cosmwasm_client::contract_storage_key;

/// Raw key for the directory contract's on-chain LtHash digest accumulator (`Item`).
pub fn digest_state_key(contract: &AccountId) -> Vec<u8> {
    contract_storage_key(contract, storage_keys::DIGEST_STATE.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_state_key_wraps_the_digest_storage_key() {
        let contract: AccountId = "n17srjznxl9dvzdkpwpw24gg668wc73val88a6m5ajg6ankwvz9wtst0cznr"
            .parse()
            .unwrap();

        let key = digest_state_key(&contract);

        // 0x03 prefix, then the 32-byte address, then the raw "digest_state" key
        assert_eq!(key[0], 0x03);
        assert_eq!(&key[1..33], contract.to_bytes().as_slice());
        assert_eq!(&key[33..], storage_keys::DIGEST_STATE.as_bytes());
    }
}
