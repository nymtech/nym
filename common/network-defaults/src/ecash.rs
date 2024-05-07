// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

/// How much bandwidth (in bytes) one ticket can buy
pub const TICKET_BANDWIDTH_VALUE: u64 = 100 * 1024 * 1024; // 100 MB

///Tickets to spend per payment
pub const SPEND_TICKETS: u64 = 1;
/// Threshold for claiming more bandwidth: 1 MB
pub const REMAINING_BANDWIDTH_THRESHOLD: i64 = 1024 * 1024;

// Constants for bloom filter for double spending detection
//Chosen for FP of
//Calculator at https://hur.st/bloomfilter/
pub const ECASH_DS_BLOOMFILTER_PARAMS: BloomfilterParameters = BloomfilterParameters {
    num_hashes: 13,
    bitmap_size: 250_000,
    sip_keys: [
        (12345678910111213141, 1415926535897932384),
        (7182818284590452353, 3571113171923293137),
    ],
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct BloomfilterParameters {
    pub num_hashes: u32,
    pub bitmap_size: u64,
    pub sip_keys: [(u64, u64); 2],
}

impl BloomfilterParameters {
    pub const fn byte_size(&self) -> u64 {
        self.bitmap_size / 8
    }

    pub const fn default_ecash() -> Self {
        ECASH_DS_BLOOMFILTER_PARAMS
    }
}
