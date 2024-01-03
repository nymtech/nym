// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ContractError;
use cosmwasm_std::{Addr, Storage};
use cw_storage_plus::Map;
use nym_coconut_dkg_common::types::{
    ContractSafeBytes, DealingIndex, EpochId, PartialContractDealing, TOTAL_DEALINGS,
};

pub(crate) const DEALINGS_PAGE_MAX_LIMIT: u32 = 2;
pub(crate) const DEALINGS_PAGE_DEFAULT_LIMIT: u32 = 1;

type Dealer<'a> = &'a Addr;

#[deprecated]
type DealingKey<'a> = &'a Addr;

// dealings are stored in a multilevel map with the following hierarchy:
//  - epoch-id:
//      - issuer-address:
//          - dealing id:
//              - dealing content
// NOTE: we're storing raw bytes bypassing serialization, so make sure you always use the below methods for using the storage!
pub(crate) const DEALINGS: Map<(EpochId, Dealer, DealingIndex), ContractSafeBytes> =
    Map::new("dealing");

pub fn has_committed_dealing(
    storage: &dyn Storage,
    epoch_id: EpochId,
    dealer: &Addr,
    dealing_index: DealingIndex,
) -> bool {
    DEALINGS.has(storage, (epoch_id, dealer, dealing_index))
}

pub fn save_dealing(
    storage: &mut dyn Storage,
    epoch_id: EpochId,
    dealer: &Addr,
    dealing: PartialContractDealing,
) {
    // NOTE: we're storing bytes directly here!
    let storage_key = DEALINGS.key((epoch_id, dealer, dealing.index));
    storage.set(&storage_key, dealing.data.as_slice());
}

// Note to whoever is looking at this implementation and is thinking of using something similar
// for storing small commitments/hashes of data on chain:
// If there's a lot of entries you want to store thinking, "oh, this digest is only 32 bytes, it's not that much",
// the default cosmwasm' serializer will bloat it to around ~100B. So you really don't want to be using
// Buckets/Maps, etc. for that purpose. Instead you want to use `storage` directly (look into the actual implementation of
// `Map` or `Bucket` to see what I mean. Instead of using the `to_vec` method on serde_json_wasm, you'd
// provide your data directly yourself.
// but you must be extremely careful when doing so, as you might end up overwriting some existing data
// if you don't choose your prefixes wisely.
// I didn't have to do it here as I'm storing relatively little data and after just base58-encoding
// my bytes, I was fine with the json overhead.

// if TOTAL_DEALINGS is modified to anything other then current value (5), this part will also need
// to be modified
pub(crate) const DEALINGS_BYTES: [Map<'_, DealingKey<'_>, ContractSafeBytes>; TOTAL_DEALINGS] = [
    Map::new("dbyt1"),
    Map::new("dbyt2"),
    Map::new("dbyt3"),
    Map::new("dbyt4"),
    Map::new("dbyt5"),
];
