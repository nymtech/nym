// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::{Order, StdResult, Storage};
use cw_storage_plus::{Bound, Item, Key, Path, Prefix, PrimaryKey};
use nym_ecash_contract_common::deposit::DepositId;
use nym_ecash_contract_common::{deposit::Deposit, EcashContractError};

pub(crate) struct DepositStorage<'a> {
    pub(crate) deposit_id_counter: Item<'a, DepositId>,
    pub(crate) deposits: StoredDeposits,
}

impl<'a> DepositStorage<'a> {
    pub const fn new() -> Self {
        DepositStorage {
            deposit_id_counter: Item::new("deposit_ids"),
            deposits: StoredDeposits,
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
        bs58_encoded_ed25519: String,
    ) -> Result<DepositId, EcashContractError> {
        let id = self.next_id(storage)?;

        // ed25519 key MUST be represented with valid bs58 representation
        // and after decoding it's always exactly 32 bytes which takes less
        // space than its string representation (~44 bytes)
        let bytes = Deposit::new(bs58_encoded_ed25519).to_bytes()?;

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

        Ok(Some(Deposit::try_from_bytes(&deposit_bytes)?))
    }
    pub fn range(
        &'a self,
        store: &'a dyn Storage,
        min: Option<Bound<'a, DepositId>>,
        max: Option<Bound<'a, DepositId>>,
        order: Order,
    ) -> impl Iterator<Item = StdResult<(DepositId, Deposit)>> + 'a {
        self.deposits.no_prefix().range(store, min, max, order)
    }
}

// a helper structure for storing deposits to bypass json serialisation and use more efficient and compact representation
pub(crate) struct StoredDeposits;

impl StoredDeposits {
    const NAMESPACE: &'static [u8] = b"deposit";

    fn deserialize_deposit_record(kv: cosmwasm_std::Record) -> StdResult<(DepositId, Deposit)> {
        let (k, deposit_bytes) = kv;
        let id = <DepositId as cw_storage_plus::KeyDeserialize>::from_vec(k)?;

        Ok((id, Deposit::try_from_bytes(&deposit_bytes)?))
    }

    fn no_prefix(&self) -> Prefix<DepositId, Deposit, DepositId> {
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
        for _ in 0..count {
            let ed25519_keypair = ed25519::KeyPair::new(&mut rng);

            let bs58_encoded_ed25519 = ed25519_keypair.public_key().to_base58_string();
            expected.push(Deposit {
                bs58_encoded_ed25519_pubkey: bs58_encoded_ed25519.clone(),
            });

            storage
                .save_deposit(deps.as_mut().storage, bs58_encoded_ed25519)
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
