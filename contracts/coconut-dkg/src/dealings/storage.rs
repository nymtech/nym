// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use coconut_dkg_common::types::{ContractSafeCommitment, EpochId};
use cosmwasm_std::Addr;
use cw_storage_plus::Map;

pub(crate) const COMMITMENTS_PAGE_MAX_LIMIT: u32 = 75;
pub(crate) const COMMITMENTS_PAGE_DEFAULT_LIMIT: u32 = 50;

type CommitmentKey<'a> = (EpochId, &'a Addr);

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
pub(crate) const DEALING_COMMITMENTS: Map<'_, CommitmentKey<'_>, ContractSafeCommitment> =
    Map::new("dcmt");
