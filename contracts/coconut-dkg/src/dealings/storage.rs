// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Addr, Order, Record, StdResult, Storage};
use cw_storage_plus::{Bound, Key, KeyDeserialize, Path, Prefix, Prefixer, PrimaryKey};
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

// part of `StoredDealing` to make existence lookup cheaper
// TODO: do it later since we need to chunk the dealings anyway
// pub(crate) struct UNIMPLEMENTED_DealingLookup;

impl StoredDealing {
    const NAMESPACE: &'static [u8] = b"dealing";

    fn deserialize_dealing_record(kv: Record) -> StdResult<(DealingIndex, ContractDealing)> {
        let (k, v) = kv;
        let index = <DealingIndex as KeyDeserialize>::from_vec(k)?;
        let data = ContractSafeBytes(v);

        Ok((index, data))
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

    fn prefix(prefix: (EpochId, Dealer)) -> Prefix<DealingIndex, ContractSafeBytes, DealingIndex> {
        Prefix::with_deserialization_functions(
            Self::NAMESPACE,
            &prefix.prefix(),
            &[],
            // explicitly panic to make sure we're never attempting to call an unexpected deserializer on our data
            |_, _, kv| Self::deserialize_dealing_record(kv),
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
    ) -> impl Iterator<Item = StdResult<PartialContractDealing>> + 'a {
        Self::prefix(prefix)
            .range(storage, start, None, Order::Ascending)
            .map(|maybe_record| maybe_record.map(Into::into))
    }

    // iterate over all values, only to be used in tests due to the amount of data being returned
    #[cfg(test)]
    pub(crate) fn unchecked_all_entries(
        storage: &dyn Storage,
    ) -> Vec<((EpochId, Addr, DealingIndex), ContractDealing)> {
        type StorageKey<'a> = (EpochId, Dealer<'a>, DealingIndex);

        let empty_prefix: Prefix<StorageKey, ContractDealing, StorageKey> =
            Prefix::with_deserialization_functions(
                Self::NAMESPACE,
                &[],
                &[],
                |_, _, kv| StorageKey::from_vec(kv.0).map(|kt| (kt, ContractSafeBytes(kv.1))),
                |_, _, _| unimplemented!(),
            );

        empty_prefix
            .range(storage, None, None, Order::Ascending)
            .collect::<StdResult<_>>()
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::helpers::init_contract;
    use std::collections::HashMap;

    fn dealing_data(
        epoch_id: EpochId,
        dealer: Dealer,
        dealing_index: DealingIndex,
    ) -> ContractDealing {
        ContractSafeBytes(
            format!("{epoch_id},{dealer},{dealing_index}")
                .as_bytes()
                .to_vec(),
        )
    }

    #[test]
    fn saving_dealing() {
        let mut deps = init_contract();

        // make sure to check all combinations of epoch id, dealer address and dealing index to ensure nothing overlaps
        let epochs = [54, 423, 754];
        let dealers = [
            Addr::unchecked("dealer1"),
            Addr::unchecked("dealer2"),
            Addr::unchecked("dealer3"),
            Addr::unchecked("dealer4"),
            Addr::unchecked("dealer5"),
        ];
        let dealing_indices = [0, 1, 2, 3, 4, 5, 6, 7];

        for epoch_id in &epochs {
            for dealer in &dealers {
                for dealing_index in &dealing_indices {
                    assert!(!StoredDealing::exists(
                        &deps.storage,
                        *epoch_id,
                        dealer,
                        *dealing_index
                    ));

                    StoredDealing::save(
                        deps.as_mut().storage,
                        *epoch_id,
                        dealer,
                        PartialContractDealing {
                            index: *dealing_index,
                            data: dealing_data(*epoch_id, dealer, *dealing_index),
                        },
                    )
                }
            }
        }

        let all: HashMap<_, _> = StoredDealing::unchecked_all_entries(&deps.storage)
            .into_iter()
            .collect();
        assert_eq!(
            all.len(),
            epochs.len() * dealers.len() * dealing_indices.len()
        );

        for epoch_id in &epochs {
            for dealer in &dealers {
                for dealing_index in &dealing_indices {
                    assert!(StoredDealing::exists(
                        &deps.storage,
                        *epoch_id,
                        dealer,
                        *dealing_index
                    ));

                    let content =
                        StoredDealing::read(&deps.storage, *epoch_id, dealer, *dealing_index)
                            .unwrap();
                    let expected = dealing_data(*epoch_id, dealer, *dealing_index);
                    assert_eq!(expected, content);
                    assert_eq!(
                        &expected,
                        all.get(&(*epoch_id, dealer.clone(), *dealing_index))
                            .unwrap()
                    );
                }
            }
        }
    }

    #[test]
    fn iterating_over_dealings() {
        let mut deps = init_contract();

        let epochs = [54, 423, 754];
        let dealers = [
            Addr::unchecked("dealer1"),
            Addr::unchecked("dealer2"),
            Addr::unchecked("dealer3"),
            Addr::unchecked("dealer4"),
            Addr::unchecked("dealer5"),
        ];
        let dealing_indices = [0, 1, 2, 3, 4, 5, 6, 7];

        for epoch_id in &epochs {
            for dealer in &dealers {
                for dealing_index in &dealing_indices {
                    StoredDealing::save(
                        deps.as_mut().storage,
                        *epoch_id,
                        dealer,
                        PartialContractDealing {
                            index: *dealing_index,
                            data: dealing_data(*epoch_id, dealer, *dealing_index),
                        },
                    )
                }
            }
        }

        // remember, we're not testing the iterator implementation

        // nothing under epoch 0
        let dealings =
            StoredDealing::prefix_range(&deps.storage, (0, &dealers[0]), None).collect::<Vec<_>>();
        assert!(dealings.is_empty());

        // nothing for dealer "foo"
        let foo = Addr::unchecked("foo");
        let dealings =
            StoredDealing::prefix_range(&deps.storage, (epochs[0], &foo), None).collect::<Vec<_>>();
        assert!(dealings.is_empty());

        let all = StoredDealing::prefix_range(&deps.storage, (epochs[0], &dealers[0]), None)
            .collect::<Vec<_>>();
        assert_eq!(all.len(), dealing_indices.len());

        for (i, dealing) in all.iter().enumerate() {
            let expected = dealing_data(epochs[0], &dealers[0], dealing_indices[i]);
            assert_eq!(expected, dealing.as_ref().unwrap().data);
            assert_eq!(dealing_indices[i], dealing.as_ref().unwrap().index);
        }

        // for sanity sake, check another dealer with different epoch
        let all_other = StoredDealing::prefix_range(&deps.storage, (epochs[2], &dealers[3]), None)
            .collect::<Vec<_>>();
        assert_eq!(all_other.len(), dealing_indices.len());

        for (i, dealing) in all_other.iter().enumerate() {
            let expected = dealing_data(epochs[2], &dealers[3], dealing_indices[i]);
            assert_eq!(expected, dealing.as_ref().unwrap().data);
            assert_eq!(dealing_indices[i], dealing.as_ref().unwrap().index);
        }

        let without_first = StoredDealing::prefix_range(
            &deps.storage,
            (epochs[0], &dealers[0]),
            Some(Bound::exclusive(dealing_indices[0])),
        )
        .collect::<Vec<_>>();
        assert_eq!(&all[1..], without_first);

        let mid = StoredDealing::prefix_range(
            &deps.storage,
            (epochs[0], &dealers[0]),
            Some(Bound::inclusive(dealing_indices[3])),
        )
        .collect::<Vec<_>>();
        assert_eq!(&all[3..], mid);
    }
}
