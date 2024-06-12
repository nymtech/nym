// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Order, StdError, StdResult, Storage};
use cw_storage_plus::{Bound, Item, Key, Map, Path, Prefix, PrimaryKey};
use nym_ecash_contract_common::deposit::DepositId;
use nym_ecash_contract_common::{deposit::Deposit, EcashContractError};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub(crate) struct DepositStorage<'a> {
    pub(crate) deposit_id_counter: Item<'a, DepositId>,
    pub(crate) deposits: StoredDeposits,

    // helper map for looking up the info string for the purposes of compressing deposit data
    pub(crate) deposit_info_lookup: DepositInfoLookup<'a>,
}

impl<'a> DepositStorage<'a> {
    pub const fn new() -> Self {
        DepositStorage {
            deposit_id_counter: Item::new("deposit_ids"),
            deposits: StoredDeposits,
            deposit_info_lookup: DepositInfoLookup {
                lookup: Map::new("deposit_lookup"),
                reverse_lookup: Map::new("deposit_re_lookup"),
                lookup_counter: Item::new("deposit_info_ids"),
            },
        }
    }
    fn next_id(&self, store: &mut dyn Storage) -> Result<DepositId, EcashContractError> {
        let id: DepositId = self.deposit_id_counter.may_load(store)?.unwrap_or_default();
        let next_id = id + 1;
        self.deposit_id_counter.save(store, &next_id)?;
        Ok(id)
    }

    pub fn save_deposit(
        &self,
        storage: &mut dyn Storage,
        amount: u128,
        info: String,
        bs58_encoded_ed25519: String,
    ) -> Result<DepositId, EcashContractError> {
        let id = self.next_id(storage)?;

        // 1 byte for type
        // 8 bytes for value
        // 32 bytes for key
        let mut bytes = [0u8; 41];
        bytes[0] = self.deposit_info_lookup.compress(storage, &info)?;

        // SAFETY: total supply of our tokens is 1e15 which is < u64::MAX
        // thus we can simply use u64 byte representation
        let value_bytes = (amount as u64).to_be_bytes();
        bytes[1..9].copy_from_slice(&value_bytes);

        // ed25519 key MUST be represented with valid bs58 representation
        // and after decoding it's always exactly 32 bytes which takes less
        // space than its string representation (~44 bytes)
        let pub_key_bytes = Deposit::get_ed25519_pubkey_bytes(&bs58_encoded_ed25519)?;
        bytes[9..].copy_from_slice(&pub_key_bytes);

        let storage_key = StoredDeposits::storage_key(id);
        storage.set(&storage_key, &bytes);
        Ok(id)
    }

    pub fn try_load_by_id(
        &self,
        storage: &dyn Storage,
        id: DepositId,
    ) -> Result<Option<Deposit>, EcashContractError> {
        let storage_key = StoredDeposits::storage_key(id);

        let Some(deposit_bytes) = storage.get(&storage_key) else {
            return Ok(None);
        };

        let compressed_deposit = SemiCompressedDeposit::from_bytes(deposit_bytes)?;

        let info = self
            .deposit_info_lookup
            .retrieve_from_reverse_lookup(storage, compressed_deposit.info_id)?;

        Ok(Some(Deposit {
            info,
            amount: compressed_deposit.amount.into(),
            bs58_encoded_ed25519: compressed_deposit.bs58_encoded_ed25519,
        }))
    }
    pub fn range(
        &'a self,
        store: &'a dyn Storage,
        min: Option<Bound<'a, DepositId>>,
        max: Option<Bound<'a, DepositId>>,
        order: Order,
    ) -> impl Iterator<Item = StdResult<(DepositId, Deposit)>> + 'a {
        let inner: HashMap<u8, String> = HashMap::new();
        let info_lookup_cache = Rc::new(RefCell::new(inner));

        self.deposits
            .no_prefix()
            .range(store, min, max, order)
            .map(move |maybe_record| (maybe_record, info_lookup_cache.clone()))
            .map(move |(maybe_record, info_lookup_cache)| {
                maybe_record.and_then(move |(deposit_id, compressed_deposit)| {
                    let mut lookup = info_lookup_cache.borrow_mut();
                    let maybe_info = match lookup.get(&compressed_deposit.info_id) {
                        Some(cached) => Ok(cached.clone()),

                        // first time we're retrieving this particular info
                        None => {
                            match self
                                .deposit_info_lookup
                                .retrieve_from_reverse_lookup(store, compressed_deposit.info_id)
                            {
                                Ok(retrieved) => {
                                    lookup.insert(compressed_deposit.info_id, retrieved.clone());
                                    Ok(retrieved)
                                }
                                Err(err) => Err(StdError::generic_err(err.to_string())),
                            }
                        }
                    };

                    maybe_info.map(|info| {
                        (
                            deposit_id,
                            Deposit {
                                info,
                                amount: compressed_deposit.amount.into(),
                                bs58_encoded_ed25519: compressed_deposit.bs58_encoded_ed25519,
                            },
                        )
                    })
                })
            })
    }
}

// a helper structure for looking up the deposit info string for the purposes of compressing data
// i.e. so you wouldn't need to store "BandwidthVoucher" alongside every single deposit.
// instead you'd use a single byte with the same meaning
pub(crate) struct DepositInfoLookup<'a> {
    lookup: Map<'a, String, u8>,
    reverse_lookup: Map<'a, u8, String>,

    lookup_counter: Item<'a, u8>,
}

impl<'a> DepositInfoLookup<'a> {
    // if we don't have an entry in our lookup, insert it into the storage
    fn compress(&self, store: &mut dyn Storage, value: &str) -> Result<u8, EcashContractError> {
        // see if we already have that value
        if let Some(byte) = self.lookup.may_load(store, value.to_string())? {
            return Ok(byte);
        }

        // generate and persist new lookup value
        let lookup_id = self.next_id(store)?;
        self.insert_into_lookup(store, value, lookup_id)?;

        Ok(lookup_id)
    }

    fn insert_into_lookup(
        &self,
        store: &mut dyn Storage,
        value: &str,
        key: u8,
    ) -> Result<(), EcashContractError> {
        self.lookup.save(store, value.to_string(), &key)?;
        self.reverse_lookup.save(store, key, &value.to_string())?;
        Ok(())
    }

    fn retrieve_from_reverse_lookup(
        &self,
        store: &dyn Storage,
        key: u8,
    ) -> Result<String, EcashContractError> {
        self.reverse_lookup
            .load(store, key)
            .map_err(|_| EcashContractError::UnknownCompressedDepositInfoType { typ: key })
    }

    fn next_id(&self, store: &mut dyn Storage) -> Result<u8, EcashContractError> {
        let id: u8 = self.lookup_counter.may_load(store)?.unwrap_or_default();
        let next_id = id
            .checked_add(1)
            .ok_or(EcashContractError::MaximumDepositTypesReached)?;
        self.lookup_counter.save(store, &next_id)?;
        Ok(id)
    }
}

// a helper structure for storing deposits to bypass json serialisation and use more efficient and compact representation
pub(crate) struct StoredDeposits;

// this actually does not need serde traits; the bounds on the Prefix functions are incorrect
#[cw_serde]
struct SemiCompressedDeposit {
    info_id: u8,
    amount: u128,
    bs58_encoded_ed25519: String,
}

impl SemiCompressedDeposit {
    fn from_bytes(deposit_bytes: Vec<u8>) -> StdResult<Self> {
        if deposit_bytes.len() != 41 {
            return Err(StdError::generic_err("malformed deposit data"));
        }

        let info_id = deposit_bytes[0];

        // safety: we're using 8 bytes here as expected by u64
        #[allow(clippy::unwrap_used)]
        let value = u64::from_be_bytes(deposit_bytes[1..9].try_into().unwrap());

        let bs58_encoded_ed25519 = Deposit::encode_pubkey_bytes(&deposit_bytes[9..]);

        Ok(SemiCompressedDeposit {
            info_id,
            amount: value as u128,
            bs58_encoded_ed25519,
        })
    }
}

impl StoredDeposits {
    const NAMESPACE: &'static [u8] = b"deposit";

    fn deserialize_deposit_record(
        kv: cosmwasm_std::Record,
    ) -> StdResult<(DepositId, SemiCompressedDeposit)> {
        let (k, deposit_bytes) = kv;
        let id = <DepositId as cw_storage_plus::KeyDeserialize>::from_vec(k)?;

        Ok((id, SemiCompressedDeposit::from_bytes(deposit_bytes)?))
    }

    fn no_prefix(&self) -> Prefix<DepositId, SemiCompressedDeposit, DepositId> {
        cw_storage_plus::Prefix::with_deserialization_functions(
            Self::NAMESPACE,
            &[],
            &[],
            // explicitly panic to make sure we're never attempting to call an unexpected deserializer on our data
            |_, _, kv| Self::deserialize_deposit_record(kv),
            |_, _, _| panic!("attempted to call custom de_fn_v"),
        )
    }

    fn storage_key(deposit_id: u32) -> Path<Vec<u8>> {
        let key = deposit_id;
        Path::new(
            Self::NAMESPACE,
            &key.key().iter().map(Key::as_ref).collect::<Vec<_>>(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::test_rng;
    use cosmwasm_std::testing::mock_dependencies;
    use nym_crypto::asymmetric::ed25519;

    #[test]
    fn iterating_over_deposits() {
        let mut deps = mock_dependencies();
        let mut rng = test_rng();

        let storage = DepositStorage::new();

        let count = 10;
        let mut expected = Vec::new();
        for i in 0..count {
            let ed25519_keypair = ed25519::KeyPair::new(&mut rng);
            let info_id = i % 3;
            let info = format!("info-string{info_id}");
            let amount = 10000 + i as u128;
            let bs58_encoded_ed25519 = ed25519_keypair.public_key().to_base58_string();
            expected.push(Deposit {
                info: info.clone(),
                amount: amount.into(),
                bs58_encoded_ed25519: bs58_encoded_ed25519.clone(),
            });

            storage
                .save_deposit(deps.as_mut().storage, amount, info, bs58_encoded_ed25519)
                .unwrap();
        }

        // just first entry
        let res = storage
            .range(
                deps.as_ref().storage,
                None,
                Some(Bound::inclusive(1u32)),
                Order::Ascending,
            )
            .collect::<Vec<_>>();
        let (id, deposit) = res[0].as_ref().unwrap();
        assert_eq!(0, *id);
        assert_eq!(deposit, &expected[0]);

        // first three entries
        let res = storage
            .range(
                deps.as_ref().storage,
                None,
                Some(Bound::exclusive(4u32)),
                Order::Ascending,
            )
            .collect::<Vec<_>>();
        for i in 0..3 {
            let (id, deposit) = res[i].as_ref().unwrap();
            assert_eq!(i as u32, *id);
            assert_eq!(deposit, &expected[i]);
        }

        // two entries in the middle
        let res = storage
            .range(
                deps.as_ref().storage,
                Some(Bound::inclusive(5u32)),
                Some(Bound::inclusive(6u32)),
                Order::Ascending,
            )
            .collect::<Vec<_>>();
        let (id1, deposit1) = res[0].as_ref().unwrap();
        let (id2, deposit2) = res[1].as_ref().unwrap();
        assert_eq!(5, *id1);
        assert_eq!(deposit1, &expected[5]);
        assert_eq!(6, *id2);
        assert_eq!(deposit2, &expected[6]);

        // last 2 entries
        let res = storage
            .range(
                deps.as_ref().storage,
                Some(Bound::inclusive(8u32)),
                Some(Bound::inclusive(9u32)),
                Order::Ascending,
            )
            .collect::<Vec<_>>();
        let (id1, deposit1) = res[0].as_ref().unwrap();
        let (id2, deposit2) = res[1].as_ref().unwrap();
        assert_eq!(8, *id1);
        assert_eq!(deposit1, &expected[8]);
        assert_eq!(9, *id2);
        assert_eq!(deposit2, &expected[9]);

        // last 2 entries but with iterator going beyond
        let res = storage
            .range(
                deps.as_ref().storage,
                Some(Bound::inclusive(8u32)),
                Some(Bound::inclusive(42u32)),
                Order::Ascending,
            )
            .collect::<Vec<_>>();
        let (id1, deposit1) = res[0].as_ref().unwrap();
        let (id2, deposit2) = res[1].as_ref().unwrap();
        assert_eq!(8, *id1);
        assert_eq!(deposit1, &expected[8]);
        assert_eq!(9, *id2);
        assert_eq!(deposit2, &expected[9]);

        // outside the saved range
        let res = storage
            .range(
                deps.as_ref().storage,
                Some(Bound::inclusive(42u32)),
                Some(Bound::inclusive(666u32)),
                Order::Ascending,
            )
            .collect::<Vec<_>>();
        assert!(res.is_empty());

        // all entries
        let res = storage
            .range(deps.as_ref().storage, None, None, Order::Ascending)
            .collect::<Vec<_>>();
        assert_eq!(res.len(), count as usize);
        for (i, val) in res.into_iter().enumerate() {
            let (id, deposit) = val.unwrap();
            assert_eq!(id, i as u32);
            assert_eq!(&deposit, &expected[i])
        }
    }
}
