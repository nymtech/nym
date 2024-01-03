// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, Order, Record, StdResult, Storage};
use cw_storage_plus::{
    range_with_prefix, Bound, Key, KeyDeserialize, Path, Prefix, Prefixer, PrimaryKey,
};
use nym_coconut_dkg_common::types::{
    ContractDealing, ContractSafeBytes, DealingIndex, EpochId, PartialContractDealing,
};

pub(crate) const DEALINGS_PAGE_MAX_LIMIT: u32 = 2;
pub(crate) const DEALINGS_PAGE_DEFAULT_LIMIT: u32 = 1;

type Dealer<'a> = &'a Addr;

// dealings are stored in a multilevel map with the following hierarchy:
//  - epoch-id:
//      - issuer-address:
//          - dealing id:
//              - dealing content
// NOTE: we're storing raw bytes bypassing serialization, so we can't use the `Map` type,
// thus make sure you always use the below methods for using the storage!

pub(crate) struct StoredDealing;

impl StoredDealing {
    const NAMESPACE: &'static [u8] = b"dealing";

    fn deserialize_raw_dealing(kv: Record) -> StdResult<PartialContractDealing> {
        let (k, v) = kv;
        let index = <DealingIndex as KeyDeserialize>::from_vec(k)?;
        let data = ContractSafeBytes(v);

        Ok(PartialContractDealing { index, data })
    }

    fn storage_key(
        epoch_id: EpochId,
        dealer: Dealer,
        dealing_index: DealingIndex,
    ) -> Path<Vec<u8>> {
        // just replicate the behaviour from `Map::key`
        let key = (epoch_id, dealer, dealing_index);
        Path::new(
            Self::NAMESPACE,
            &key.key().iter().map(Key::as_ref).collect::<Vec<_>>(),
        )
    }

    fn prefix(prefix: (EpochId, Dealer)) -> Prefix<DealingIndex, ContractSafeBytes> {
        Prefix::with_deserialization_functions(
            Self::NAMESPACE,
            &prefix.prefix(),
            &[],
            // explicitly panic to make sure we're never attempting to call an unexpected deserializer on our data
            |_, _, _| panic!("attempted to call custom de_fn_kv"),
            |_, _, _| panic!("attempted to call custom de_fn_v"),
        )
    }

    pub(crate) fn exists(
        storage: &dyn Storage,
        epoch_id: EpochId,
        dealer: &Addr,
        dealing_index: DealingIndex,
    ) -> bool {
        StoredDealing::storage_key(epoch_id, dealer, dealing_index).has(storage)
    }

    pub(crate) fn save(
        storage: &mut dyn Storage,
        epoch_id: EpochId,
        dealer: Dealer,
        dealing: PartialContractDealing,
    ) {
        // NOTE: we're storing bytes directly here!
        let storage_key = StoredDealing::storage_key(epoch_id, dealer, dealing.index);
        storage.set(&storage_key, dealing.data.as_slice());
    }

    pub(crate) fn read(
        storage: &dyn Storage,
        epoch_id: EpochId,
        dealer: Dealer,
        dealing_index: DealingIndex,
    ) -> Option<ContractDealing> {
        let storage_key = StoredDealing::storage_key(epoch_id, dealer, dealing_index);
        let raw_dealing = storage.get(&storage_key);
        raw_dealing.map(ContractSafeBytes)
    }

    pub(crate) fn prefix_range<'a>(
        storage: &'a dyn Storage,
        prefix: (EpochId, Dealer),
        start: Option<Bound<DealingIndex>>,
    ) -> Box<dyn Iterator<Item = StdResult<PartialContractDealing>> + 'a> {
        // replicate the behaviour of `Prefix::range_raw` but without the fallible deserialization on the data
        // (since we're reading from the storage directly)
        // and whilst combining the data on the spot
        let prefix = Self::prefix(prefix);

        let mapped = range_with_prefix(
            storage,
            // note: Prefix's Deref implementation gives back its storage_prefix
            &prefix,
            start.map(|b| b.to_raw_bound()),
            None,
            Order::Ascending,
        )
        .map(Self::deserialize_raw_dealing);

        Box::new(mapped)
    }
}
