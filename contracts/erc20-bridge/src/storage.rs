// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use cosmwasm_std::Storage;
use cosmwasm_storage::{bucket, bucket_read, Bucket, ReadonlyBucket};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use erc20_bridge_contract::payment::Payment;

// buckets
const PREFIX_PAYMENTS: &[u8] = b"payments";
const PREFIX_STATUS: &[u8] = b"status";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize, JsonSchema)]
pub enum Status {
    Unchecked,
    Checked,
    Spent,
}

pub fn payments(storage: &mut dyn Storage) -> Bucket<Payment> {
    bucket(storage, PREFIX_PAYMENTS)
}

pub fn payments_read(storage: &dyn Storage) -> ReadonlyBucket<Payment> {
    bucket_read(storage, PREFIX_PAYMENTS)
}

pub fn status(storage: &mut dyn Storage) -> Bucket<Status> {
    bucket(storage, PREFIX_STATUS)
}
