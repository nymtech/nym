// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ContractError;
use cosmwasm_std::{Addr, Order, Record, StdResult, Storage};
use cw_storage_plus::{Bound, Key, KeyDeserialize, Map, Path, Prefix, Prefixer, PrimaryKey};
use nym_coconut_dkg_common::types::{
    ChunkIndex, ContractSafeBytes, DealingIndex, DealingMetadata, EpochId, PartialContractDealing,
    PartialContractDealingData,
};

pub(crate) const DEALINGS_PAGE_MAX_LIMIT: u32 = 2;
pub(crate) const DEALINGS_PAGE_DEFAULT_LIMIT: u32 = 1;

type Dealer<'a> = &'a Addr;

/// Metadata for a dealing for given `EpochId`, submitted by particular `Dealer` for given `DealingIndex`.
const DEALINGS_METADATA: Map<(EpochId, Dealer, DealingIndex), DealingMetadata> =
    Map::new("dealings_metadata");

pub(crate) fn metadata_exists(
    storage: &dyn Storage,
    epoch_id: EpochId,
    dealer: Dealer,
    dealing_index: DealingIndex,
) -> bool {
    DEALINGS_METADATA.has(storage, (epoch_id, dealer, dealing_index))
}

pub(crate) fn may_read_metadata(
    storage: &dyn Storage,
    epoch_id: EpochId,
    dealer: Dealer,
    dealing_index: DealingIndex,
) -> Result<Option<DealingMetadata>, ContractError> {
    Ok(DEALINGS_METADATA.may_load(storage, (epoch_id, dealer, dealing_index))?)
}

pub(crate) fn must_read_metadata(
    storage: &dyn Storage,
    epoch_id: EpochId,
    dealer: Dealer,
    dealing_index: DealingIndex,
) -> Result<DealingMetadata, ContractError> {
    DEALINGS_METADATA
        .may_load(storage, (epoch_id, dealer, dealing_index))?
        .ok_or_else(|| ContractError::UnavailableDealingMetadata {
            epoch_id,
            dealer: dealer.to_owned(),
            dealing_index,
        })
}

pub(crate) fn store_metadata(
    storage: &mut dyn Storage,
    epoch_id: EpochId,
    dealer: Dealer,
    dealing_index: DealingIndex,
    metadata: &DealingMetadata,
) -> Result<(), ContractError> {
    Ok(DEALINGS_METADATA.save(storage, (epoch_id, dealer, dealing_index), metadata)?)
}

// dealings are stored in a multilevel map with the following hierarchy:
//  - epoch-id:
//      - issuer-address:
//          - dealing id:
//              - chunk_id:
//                  - dealing content
// NOTE: we're storing raw bytes bypassing serialization, so we can't use the `Map` type,
// thus make sure you always use the below methods for using the storage!

pub(crate) struct StoredDealing;

impl StoredDealing {
    const NAMESPACE: &'static [u8] = b"dealing";

    fn deserialize_dealing_record(
        kv: Record,
    ) -> StdResult<(DealingIndex, PartialContractDealingData)> {
        todo!()
        // let (k, v) = kv;
        // let index = <DealingIndex as KeyDeserialize>::from_vec(k)?;
        // let data = ContractSafeBytes(v);
        //
        // Ok((index, data))
    }

    fn storage_key(
        epoch_id: EpochId,
        dealer: Dealer,
        dealing_index: DealingIndex,
        chunk_index: ChunkIndex,
    ) -> Path<Vec<u8>> {
        // just replicate the behaviour from `Map::key`
        // note: `PrimaryKey` trait is not implemented for tuple (T, U, V, W), only for up to (T, U, V)
        // that's why we create a (T, U, (V, W)) tuple(s) instead
        let key = (epoch_id, dealer, (dealing_index, chunk_index));
        Path::new(
            Self::NAMESPACE,
            &key.key().iter().map(Key::as_ref).collect::<Vec<_>>(),
        )
    }

    // fn prefix(prefix: (EpochId, Dealer)) -> Prefix<DealingIndex, ContractSafeBytes, DealingIndex> {
    //     todo!()
    //     // Prefix::with_deserialization_functions(
    //     //     Self::NAMESPACE,
    //     //     &prefix.prefix(),
    //     //     &[],
    //     //     // explicitly panic to make sure we're never attempting to call an unexpected deserializer on our data
    //     //     |_, _, kv| Self::deserialize_dealing_record(kv),
    //     //     |_, _, _| panic!("attempted to call custom de_fn_v"),
    //     // )
    // }

    pub(crate) fn exists(
        storage: &dyn Storage,
        epoch_id: EpochId,
        dealer: &Addr,
        dealing_index: DealingIndex,
    ) -> bool {
        todo!()
        // StoredDealing::storage_key(epoch_id, dealer, dealing_index).has(storage)
    }

    pub(crate) fn save(
        storage: &mut dyn Storage,
        epoch_id: EpochId,
        dealer: Dealer,
        dealing: PartialContractDealing,
    ) {
        // NOTE: we're storing bytes directly here!
        let storage_key =
            Self::storage_key(epoch_id, dealer, dealing.dealing_index, dealing.chunk_index);
        storage.set(&storage_key, dealing.data.as_slice());
    }

    pub(crate) fn read(
        storage: &dyn Storage,
        epoch_id: EpochId,
        dealer: Dealer,
        dealing_index: DealingIndex,
        chunk_index: ChunkIndex,
    ) -> Option<PartialContractDealingData> {
        let storage_key = Self::storage_key(epoch_id, dealer, dealing_index, chunk_index);
        storage.get(&storage_key).map(ContractSafeBytes)
    }

    // pub(crate) fn prefix_range<'a>(
    //     storage: &'a dyn Storage,
    //     prefix: (EpochId, Dealer),
    //     start: Option<Bound<DealingIndex>>,
    // ) -> impl Iterator<Item = StdResult<PartialContractDealing>> + 'a {
    //     vec![].into_iter()
    //     // todo!()
    //     // Self::prefix(prefix)
    //     //     .range(storage, start, None, Order::Ascending)
    //     //     .map(|maybe_record| maybe_record.map(Into::into))
    // }

    // iterate over all values, only to be used in tests due to the amount of data being returned
    #[cfg(test)]
    pub(crate) fn unchecked_all_entries(
        storage: &dyn Storage,
    ) -> Vec<((EpochId, Addr, DealingIndex), PartialContractDealingData)> {
        todo!()
        // type StorageKey<'a> = (EpochId, Dealer<'a>, DealingIndex);
        //
        // let empty_prefix: Prefix<StorageKey, ContractDealing, StorageKey> =
        //     Prefix::with_deserialization_functions(
        //         Self::NAMESPACE,
        //         &[],
        //         &[],
        //         |_, _, kv| StorageKey::from_vec(kv.0).map(|kt| (kt, ContractSafeBytes(kv.1))),
        //         |_, _, _| unimplemented!(),
        //     );
        //
        // empty_prefix
        //     .range(storage, None, None, Order::Ascending)
        //     .collect::<StdResult<_>>()
        //     .unwrap()
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::support::tests::helpers::init_contract;
//     use std::collections::HashMap;
//
//     fn dealing_data(
//         epoch_id: EpochId,
//         dealer: Dealer,
//         dealing_index: DealingIndex,
//     ) -> PartialContractDealingData {
//         ContractSafeBytes(
//             format!("{epoch_id},{dealer},{dealing_index}")
//                 .as_bytes()
//                 .to_vec(),
//         )
//     }
//
//     #[test]
//     fn saving_dealing() {
//         let mut deps = init_contract();
//
//         // make sure to check all combinations of epoch id, dealer address and dealing index to ensure nothing overlaps
//         let epochs = [54, 423, 754];
//         let dealers = [
//             Addr::unchecked("dealer1"),
//             Addr::unchecked("dealer2"),
//             Addr::unchecked("dealer3"),
//             Addr::unchecked("dealer4"),
//             Addr::unchecked("dealer5"),
//         ];
//         let dealing_indices = [0, 1, 2, 3, 4, 5, 6, 7];
//
//         for epoch_id in &epochs {
//             for dealer in &dealers {
//                 for dealing_index in &dealing_indices {
//                     assert!(!StoredDealing::exists(
//                         &deps.storage,
//                         *epoch_id,
//                         dealer,
//                         *dealing_index
//                     ));
//
//                     StoredDealing::save(
//                         deps.as_mut().storage,
//                         *epoch_id,
//                         dealer,
//                         PartialContractDealing {
//                             dealing_index: *dealing_index,
//                             data: dealing_data(*epoch_id, dealer, *dealing_index),
//                         },
//                     )
//                 }
//             }
//         }
//
//         let all: HashMap<_, _> = StoredDealing::unchecked_all_entries(&deps.storage)
//             .into_iter()
//             .collect();
//         assert_eq!(
//             all.len(),
//             epochs.len() * dealers.len() * dealing_indices.len()
//         );
//
//         for epoch_id in &epochs {
//             for dealer in &dealers {
//                 for dealing_index in &dealing_indices {
//                     assert!(StoredDealing::exists(
//                         &deps.storage,
//                         *epoch_id,
//                         dealer,
//                         *dealing_index
//                     ));
//
//                     let content =
//                         StoredDealing::read(&deps.storage, *epoch_id, dealer, *dealing_index)
//                             .unwrap();
//                     let expected = dealing_data(*epoch_id, dealer, *dealing_index);
//                     assert_eq!(expected, content);
//                     assert_eq!(
//                         &expected,
//                         all.get(&(*epoch_id, dealer.clone(), *dealing_index))
//                             .unwrap()
//                     );
//                 }
//             }
//         }
//     }
//
//     #[test]
//     fn iterating_over_dealings() {
//         let mut deps = init_contract();
//
//         let epochs = [54, 423, 754];
//         let dealers = [
//             Addr::unchecked("dealer1"),
//             Addr::unchecked("dealer2"),
//             Addr::unchecked("dealer3"),
//             Addr::unchecked("dealer4"),
//             Addr::unchecked("dealer5"),
//         ];
//         let dealing_indices = [0, 1, 2, 3, 4, 5, 6, 7];
//
//         for epoch_id in &epochs {
//             for dealer in &dealers {
//                 for dealing_index in &dealing_indices {
//                     StoredDealing::save(
//                         deps.as_mut().storage,
//                         *epoch_id,
//                         dealer,
//                         PartialContractDealing {
//                             dealing_index: *dealing_index,
//                             data: dealing_data(*epoch_id, dealer, *dealing_index),
//                         },
//                     )
//                 }
//             }
//         }
//
//         // remember, we're not testing the iterator implementation
//
//         // nothing under epoch 0
//         let dealings =
//             StoredDealing::prefix_range(&deps.storage, (0, &dealers[0]), None).collect::<Vec<_>>();
//         assert!(dealings.is_empty());
//
//         // nothing for dealer "foo"
//         let foo = Addr::unchecked("foo");
//         let dealings =
//             StoredDealing::prefix_range(&deps.storage, (epochs[0], &foo), None).collect::<Vec<_>>();
//         assert!(dealings.is_empty());
//
//         let all = StoredDealing::prefix_range(&deps.storage, (epochs[0], &dealers[0]), None)
//             .collect::<Vec<_>>();
//         assert_eq!(all.len(), dealing_indices.len());
//
//         for (i, dealing) in all.iter().enumerate() {
//             let expected = dealing_data(epochs[0], &dealers[0], dealing_indices[i]);
//             assert_eq!(expected, dealing.as_ref().unwrap().data);
//             assert_eq!(dealing_indices[i], dealing.as_ref().unwrap().dealing_index);
//         }
//
//         // for sanity sake, check another dealer with different epoch
//         let all_other = StoredDealing::prefix_range(&deps.storage, (epochs[2], &dealers[3]), None)
//             .collect::<Vec<_>>();
//         assert_eq!(all_other.len(), dealing_indices.len());
//
//         for (i, dealing) in all_other.iter().enumerate() {
//             let expected = dealing_data(epochs[2], &dealers[3], dealing_indices[i]);
//             assert_eq!(expected, dealing.as_ref().unwrap().data);
//             assert_eq!(dealing_indices[i], dealing.as_ref().unwrap().dealing_index);
//         }
//
//         let without_first = StoredDealing::prefix_range(
//             &deps.storage,
//             (epochs[0], &dealers[0]),
//             Some(Bound::exclusive(dealing_indices[0])),
//         )
//         .collect::<Vec<_>>();
//         assert_eq!(&all[1..], without_first);
//
//         let mid = StoredDealing::prefix_range(
//             &deps.storage,
//             (epochs[0], &dealers[0]),
//             Some(Bound::inclusive(dealing_indices[3])),
//         )
//         .collect::<Vec<_>>();
//         assert_eq!(&all[3..], mid);
//     }
// }
