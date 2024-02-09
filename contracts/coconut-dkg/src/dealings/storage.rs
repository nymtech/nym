// Copyright 2022-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::error::ContractError;
use cosmwasm_std::{Addr, Storage};
use cw_storage_plus::{Key, Map, Path, PrimaryKey};
use nym_coconut_dkg_common::dealing::{DealingMetadata, PartialContractDealing};
use nym_coconut_dkg_common::types::{
    ChunkIndex, ContractSafeBytes, DealingIndex, EpochId, PartialContractDealingData,
};

type Dealer<'a> = &'a Addr;

/// Metadata for a dealing for given `EpochId`, submitted by particular `Dealer` for given `DealingIndex`.
pub(crate) const DEALINGS_METADATA: Map<(EpochId, Dealer, DealingIndex), DealingMetadata> =
    Map::new("dealings_metadata");

pub(crate) fn metadata_exists(
    storage: &dyn Storage,
    epoch_id: EpochId,
    dealer: Dealer,
    dealing_index: DealingIndex,
) -> bool {
    DEALINGS_METADATA.has(storage, (epoch_id, dealer, dealing_index))
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

// dealings data is stored in a multilevel map with the following hierarchy:
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

    // prefix-range related should we need it
    #[cfg(test)]
    fn deserialize_dealing_record(
        kv: cosmwasm_std::Record,
    ) -> cosmwasm_std::StdResult<(ChunkIndex, PartialContractDealingData)> {
        let (k, v) = kv;
        let index = <ChunkIndex as cw_storage_plus::KeyDeserialize>::from_vec(k)?;
        let data = ContractSafeBytes(v);

        Ok((index, data))
    }

    // prefix-range related should we need it
    #[cfg(test)]
    fn prefix(
        prefix: (EpochId, Dealer, DealingIndex),
    ) -> cw_storage_plus::Prefix<ChunkIndex, PartialContractDealingData, ChunkIndex> {
        use cw_storage_plus::Prefixer;

        cw_storage_plus::Prefix::with_deserialization_functions(
            Self::NAMESPACE,
            &prefix.prefix(),
            &[],
            // explicitly panic to make sure we're never attempting to call an unexpected deserializer on our data
            |_, _, kv| Self::deserialize_dealing_record(kv),
            |_, _, _| panic!("attempted to call custom de_fn_v"),
        )
    }

    // prefix-range related should we need it
    #[cfg(test)]
    pub(crate) fn prefix_range<'a>(
        storage: &'a dyn Storage,
        prefix: (EpochId, Dealer, DealingIndex),
        start: Option<cw_storage_plus::Bound<ChunkIndex>>,
    ) -> impl Iterator<Item = cosmwasm_std::StdResult<PartialContractDealing>> + 'a {
        let dealing_index = prefix.2;
        Self::prefix(prefix)
            .range(storage, start, None, cosmwasm_std::Order::Ascending)
            .map(move |maybe_record| {
                maybe_record.map(|(chunk_index, data)| PartialContractDealing {
                    dealing_index,
                    chunk_index,
                    data,
                })
            })
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

    // pub(crate) fn exists(
    //     storage: &dyn Storage,
    //     epoch_id: EpochId,
    //     dealer: &Addr,
    //     dealing_index: DealingIndex,
    //     chunk_index: ChunkIndex,
    // ) -> StdResult<bool> {
    //     // whenever the dealing is saved, the metadata is appropriately updated
    //     // reading metadata is way cheaper than the dealing chunk itself
    //     let Some(metadata) =
    //         DEALINGS_METADATA.may_load(storage, (epoch_id, dealer, dealing_index))?
    //     else {
    //         return Ok(false);
    //     };
    //     let Some(chunk_info) = metadata.submitted_chunks.get(&chunk_index) else {
    //         return Ok(false);
    //     };
    //     Ok(chunk_info.status.submitted())
    //     // StoredDealing::storage_key(epoch_id, dealer, dealing_index).has(storage)
    // }

    pub(crate) fn save(
        storage: &mut dyn Storage,
        epoch_id: EpochId,
        dealer: Dealer,
        dealng_chunk: PartialContractDealing,
    ) {
        // NOTE: we're storing bytes directly here!
        let storage_key = Self::storage_key(
            epoch_id,
            dealer,
            dealng_chunk.dealing_index,
            dealng_chunk.chunk_index,
        );
        storage.set(&storage_key, dealng_chunk.data.as_slice());
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

    // iterate over all values, only to be used in tests due to the amount of data being returned
    #[cfg(test)]
    #[allow(clippy::type_complexity)]
    pub(crate) fn unchecked_all_entries(
        storage: &dyn Storage,
    ) -> Vec<(
        (EpochId, Addr, (DealingIndex, ChunkIndex)),
        PartialContractDealingData,
    )> {
        use cw_storage_plus::KeyDeserialize;

        type StorageKey<'a> = (EpochId, Dealer<'a>, (DealingIndex, ChunkIndex));

        let empty_prefix: cw_storage_plus::Prefix<
            StorageKey,
            PartialContractDealingData,
            StorageKey,
        > = cw_storage_plus::Prefix::with_deserialization_functions(
            Self::NAMESPACE,
            &[],
            &[],
            |_, _, kv| StorageKey::from_vec(kv.0).map(|kt| (kt, ContractSafeBytes(kv.1))),
            |_, _, _| unimplemented!(),
        );

        empty_prefix
            .range(storage, None, None, cosmwasm_std::Order::Ascending)
            .collect::<cosmwasm_std::StdResult<_>>()
            .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::helpers::init_contract;
    use cw_storage_plus::Bound;
    use std::collections::HashMap;

    fn dealing_data(
        epoch_id: EpochId,
        dealer: Dealer,
        dealing_index: DealingIndex,
        chunk_index: ChunkIndex,
    ) -> PartialContractDealingData {
        ContractSafeBytes(
            format!("{epoch_id},{dealer},{dealing_index},{chunk_index}")
                .as_bytes()
                .to_vec(),
        )
    }

    #[test]
    fn saving_dealing_chunks() {
        let mut deps = init_contract();

        fn exists_in_storage(
            storage: &dyn Storage,
            epoch_id: EpochId,
            dealer: Dealer,
            dealing_index: DealingIndex,
            chunk_index: ChunkIndex,
        ) -> bool {
            StoredDealing::storage_key(epoch_id, dealer, dealing_index, chunk_index).has(storage)
        }

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
        let chunk_indices = [0, 1, 2, 3, 4];

        for epoch_id in &epochs {
            for dealer in &dealers {
                for dealing_index in &dealing_indices {
                    for chunk_index in &chunk_indices {
                        assert!(!exists_in_storage(
                            &deps.storage,
                            *epoch_id,
                            dealer,
                            *dealing_index,
                            *chunk_index
                        ));

                        StoredDealing::save(
                            deps.as_mut().storage,
                            *epoch_id,
                            dealer,
                            PartialContractDealing {
                                dealing_index: *dealing_index,
                                chunk_index: *chunk_index,
                                data: dealing_data(*epoch_id, dealer, *dealing_index, *chunk_index),
                            },
                        )
                    }
                }
            }
        }

        let all: HashMap<_, _> = StoredDealing::unchecked_all_entries(&deps.storage)
            .into_iter()
            .collect();
        assert_eq!(
            all.len(),
            epochs.len() * dealers.len() * dealing_indices.len() * chunk_indices.len()
        );

        for epoch_id in &epochs {
            for dealer in &dealers {
                for dealing_index in &dealing_indices {
                    for chunk_index in &chunk_indices {
                        assert!(exists_in_storage(
                            &deps.storage,
                            *epoch_id,
                            dealer,
                            *dealing_index,
                            *chunk_index
                        ));

                        let content = StoredDealing::read(
                            &deps.storage,
                            *epoch_id,
                            dealer,
                            *dealing_index,
                            *chunk_index,
                        )
                        .unwrap();
                        let expected =
                            dealing_data(*epoch_id, dealer, *dealing_index, *chunk_index);
                        assert_eq!(expected, content);
                        assert_eq!(
                            &expected,
                            all.get(&(*epoch_id, dealer.clone(), (*dealing_index, *chunk_index)))
                                .unwrap()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn iterating_over_dealing_chunks() {
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
        let chunk_indices = [0, 1, 2, 3, 4];

        for epoch_id in &epochs {
            for dealer in &dealers {
                for dealing_index in &dealing_indices {
                    for chunk_index in &chunk_indices {
                        StoredDealing::save(
                            deps.as_mut().storage,
                            *epoch_id,
                            dealer,
                            PartialContractDealing {
                                dealing_index: *dealing_index,
                                chunk_index: *chunk_index,
                                data: dealing_data(*epoch_id, dealer, *dealing_index, *chunk_index),
                            },
                        )
                    }
                }
            }
        }

        // remember, we're not testing the iterator implementation

        // nothing under epoch 0
        let dealings =
            StoredDealing::prefix_range(&deps.storage, (0, &dealers[0], dealing_indices[0]), None)
                .collect::<Vec<_>>();
        assert!(dealings.is_empty());

        // nothing for dealer "foo"
        let foo = Addr::unchecked("foo");
        let dealings =
            StoredDealing::prefix_range(&deps.storage, (epochs[0], &foo, dealing_indices[0]), None)
                .collect::<Vec<_>>();
        assert!(dealings.is_empty());

        // nothing for dealing index 99
        let dealings =
            StoredDealing::prefix_range(&deps.storage, (epochs[0], &dealers[0], 99), None)
                .collect::<Vec<_>>();
        assert!(dealings.is_empty());

        let all = StoredDealing::prefix_range(
            &deps.storage,
            (epochs[0], &dealers[0], dealing_indices[0]),
            None,
        )
        .collect::<Vec<_>>();
        assert_eq!(all.len(), chunk_indices.len());

        for (i, dealing) in all.iter().enumerate() {
            let expected =
                dealing_data(epochs[0], &dealers[0], dealing_indices[0], chunk_indices[i]);
            assert_eq!(expected, dealing.as_ref().unwrap().data);
            assert_eq!(chunk_indices[i], dealing.as_ref().unwrap().chunk_index);
        }

        // for sanity sake, check another dealer with different epoch and different dealing index
        let all_other = StoredDealing::prefix_range(
            &deps.storage,
            (epochs[2], &dealers[3], dealing_indices[4]),
            None,
        )
        .collect::<Vec<_>>();
        assert_eq!(all_other.len(), chunk_indices.len());

        for (i, dealing) in all_other.iter().enumerate() {
            let expected =
                dealing_data(epochs[2], &dealers[3], dealing_indices[4], chunk_indices[i]);
            assert_eq!(expected, dealing.as_ref().unwrap().data);
            assert_eq!(chunk_indices[i], dealing.as_ref().unwrap().chunk_index);
        }

        let without_first = StoredDealing::prefix_range(
            &deps.storage,
            (epochs[0], &dealers[0], dealing_indices[0]),
            Some(Bound::exclusive(chunk_indices[0])),
        )
        .collect::<Vec<_>>();
        assert_eq!(&all[1..], without_first);

        let mid = StoredDealing::prefix_range(
            &deps.storage,
            (epochs[0], &dealers[0], dealing_indices[0]),
            Some(Bound::inclusive(chunk_indices[3])),
        )
        .collect::<Vec<_>>();
        assert_eq!(&all[3..], mid);
    }
}
