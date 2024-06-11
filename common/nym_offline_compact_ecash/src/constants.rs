// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use bls12_381::Scalar;

pub const PUBLIC_ATTRIBUTES_LEN: usize = 1; //expiration date
pub const PRIVATE_ATTRIBUTES_LEN: usize = 2; //user and wallet secret
pub const ATTRIBUTES_LEN: usize = PUBLIC_ATTRIBUTES_LEN + PRIVATE_ATTRIBUTES_LEN; // number of attributes encoded in a single zk-nym credential

pub const CRED_VALIDITY_PERIOD_DAYS: u64 = 30;
pub(crate) const SECONDS_PER_DAY: u64 = 86400;

/// Total number of tickets in each issued ticket book.
pub const NB_TICKETS: u64 = 1000;

pub const TYPE_EXP: Scalar = Scalar::from_raw([
    u64::from_le_bytes(*b"ZKNYMEXP"),
    u64::from_le_bytes(*b"IRATIOND"),
    u64::from_le_bytes(*b"ATE4llCB"),
    u64::from_le_bytes(*b"MEypAxr3"),
]);
pub const TYPE_IDX: Scalar = Scalar::from_raw([
    u64::from_le_bytes(*b"ZKNYMSIN"),
    u64::from_le_bytes(*b"DICESh^7"),
    u64::from_le_bytes(*b"gTYbhnap"),
    u64::from_le_bytes(*b"*12n5GG6"),
]);
