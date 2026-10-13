// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The LtHash accumulator a contract commits its entry set to, stored raw so that clients can
//! prove it.

use cosmwasm_schema::cw_serde;
use cosmwasm_std::Binary;
use thiserror::Error;

#[cfg(feature = "lthash")]
use cosmwasm_std::Storage;
#[cfg(feature = "lthash")]
use nym_lthash::{DIGEST_LEN, LtHash16};

/// A digest-committed record's LtHash leaf, the contract's frozen encoding.
pub trait DigestLeaf {
    fn digest_leaf(&self) -> Vec<u8>;
}

/// The 32-byte collapse of the LtHash accumulator. Unproven: smart queries carry no proof, so
/// a client that needs one performs a raw store read at the digest key instead.
#[cw_serde]
pub struct DigestResponse {
    pub digest: Binary,
}

/// The stored accumulator is not `DIGEST_LEN` bytes long. The contract always writes exactly
/// that many, so this is state corruption.
#[derive(Error, Debug, Clone, Copy, PartialEq, Eq)]
#[error("the stored digest accumulator is corrupt (unexpected length)")]
pub struct CorruptDigestState;

/// The full LtHash accumulator, stored raw at a fixed key.
///
/// These bytes are what a client obtains an ICS23 proof for, since CosmWasm smart queries carry
/// none, so three things are load-bearing and must never change once a contract is deployed:
///
///   - the key is used **verbatim** through `store.set`, not through a `cw-storage-plus` `Item`,
///     so the proven key is exactly its bytes appended to the contract's storage prefix (see
///     `contract_storage_key` in `nym-validator-client`). An `Item` would not do anyway: serde
///     cannot (de)serialise a `[u8; DIGEST_LEN]`;
///   - the value is the accumulator itself, never its 32-byte collapse, since a verifier
///     reconstructs an `LtHash16` from `DIGEST_LEN` bytes and would reject anything else;
///   - the key itself, across migrations included: a client pins it in order to prove against
///     it, so renaming it silently breaks every verifier rather than failing loudly.
///
/// An absent key is the empty accumulator, which is also what a verifier concludes from a proof
/// of absence, so a contract need not write anything until its first entry.
#[cfg(feature = "lthash")]
pub struct DigestItem {
    key: &'static str,
}

#[cfg(feature = "lthash")]
impl DigestItem {
    pub const fn new(key: &'static str) -> Self {
        DigestItem { key }
    }

    /// The stored accumulator, or the empty one if nothing has been written yet.
    pub fn load(&self, store: &dyn Storage) -> Result<LtHash16, CorruptDigestState> {
        match store.get(self.key.as_bytes()) {
            Some(bytes) => {
                let raw: &[u8; DIGEST_LEN] = bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| CorruptDigestState)?;
                Ok(LtHash16::from_bytes(raw))
            }
            None => Ok(LtHash16::new()),
        }
    }

    pub fn save(&self, store: &mut dyn Storage, digest: &LtHash16) {
        store.set(self.key.as_bytes(), &digest.to_bytes());
    }

    /// The `Digest {}` query's answer: the collapse of the stored accumulator.
    pub fn query(&self, store: &dyn Storage) -> Result<DigestResponse, CorruptDigestState> {
        Ok(DigestResponse {
            digest: Binary::new(self.load(store)?.out().to_vec()),
        })
    }
}

#[cfg(all(test, feature = "lthash"))]
mod tests {
    use super::*;
    use cosmwasm_std::testing::MockStorage;

    const DIGEST: DigestItem = DigestItem::new("digest_state");

    #[test]
    fn an_absent_key_is_the_empty_accumulator() {
        let store = MockStorage::new();
        assert_eq!(DIGEST.load(&store), Ok(LtHash16::new()));
    }

    #[test]
    fn the_full_accumulator_round_trips_under_the_raw_key() {
        let mut store = MockStorage::new();
        let mut digest = LtHash16::new();
        digest.add(b"entry");

        DIGEST.save(&mut store, &digest);
        assert_eq!(store.get(b"digest_state"), Some(digest.to_bytes().to_vec()));
        assert_eq!(DIGEST.load(&store), Ok(digest));
    }

    #[test]
    fn a_value_of_any_other_length_is_corrupt() {
        let mut store = MockStorage::new();
        store.set(b"digest_state", &[0; 32]);
        assert_eq!(DIGEST.load(&store), Err(CorruptDigestState));
        assert_eq!(DIGEST.query(&store), Err(CorruptDigestState));
    }

    #[test]
    fn the_query_returns_the_collapse_of_the_stored_accumulator() {
        let mut store = MockStorage::new();
        let mut digest = LtHash16::new();
        digest.add(b"entry");
        DIGEST.save(&mut store, &digest);

        let response = DIGEST.query(&store).unwrap();
        assert_eq!(response.digest.to_vec(), digest.out().to_vec());
        assert_eq!(response.digest.len(), 32);
    }
}
