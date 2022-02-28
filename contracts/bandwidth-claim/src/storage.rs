// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Storage;
use cosmwasm_storage::{bucket, bucket_read, Bucket, ReadonlyBucket};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use bandwidth_claim_contract::payment::Payment;

// buckets
const PREFIX_PAYMENTS: &[u8] = b"payments";
const PREFIX_STATUS: &[u8] = b"status";
const PREFIX_COCONUT: &[u8] = b"coconut";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub enum Status {
    Unchecked,
    Checked,
    Spent,
}

pub fn payments(storage: &mut dyn Storage) -> Bucket<'_, Payment> {
    bucket(storage, PREFIX_PAYMENTS)
}

pub fn payments_read(storage: &dyn Storage) -> ReadonlyBucket<'_, Payment> {
    bucket_read(storage, PREFIX_PAYMENTS)
}

pub fn status(storage: &mut dyn Storage) -> Bucket<'_, Status> {
    bucket(storage, PREFIX_STATUS)
}

pub fn coconut(storage: &mut dyn Storage) -> Bucket<'_, Payment> {
    bucket(storage, PREFIX_COCONUT)
}

pub fn coconut_read(storage: &dyn Storage) -> ReadonlyBucket<'_, Payment> {
    bucket_read(storage, PREFIX_COCONUT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support::tests::helpers;
    use bandwidth_claim_contract::keys::PublicKey;
    use cosmwasm_std::testing::MockStorage;

    #[test]
    fn payments_single_read_retrieval() {
        let mut storage = MockStorage::new();
        let key1 = PublicKey::new([1; 32]);
        let key2 = PublicKey::new([2; 32]);
        let payment1 = helpers::payment_fixture();
        let payment2 = helpers::payment_fixture();
        payments(&mut storage)
            .save(key1.as_ref(), &payment1)
            .unwrap();
        payments(&mut storage)
            .save(key2.as_ref(), &payment2)
            .unwrap();

        let res1 = payments_read(&storage).load(key1.as_ref()).unwrap();
        let res2 = payments_read(&storage).load(key2.as_ref()).unwrap();
        assert_eq!(payment1, res1);
        assert_eq!(payment2, res2);
    }

    #[test]
    fn status_single_read_retrieval() {
        let mut storage = MockStorage::new();
        let key1 = PublicKey::new([1; 32]);
        let key2 = PublicKey::new([2; 32]);
        let status_value = Status::Unchecked;
        status(&mut storage)
            .save(key1.as_ref(), &status_value)
            .unwrap();
        status(&mut storage)
            .save(key2.as_ref(), &status_value)
            .unwrap();

        let res1 = status(&mut storage).load(key1.as_ref()).unwrap();
        assert_eq!(status_value, res1);
        let res2 = status(&mut storage).load(key2.as_ref()).unwrap();
        assert_eq!(status_value, res2);
    }
}
