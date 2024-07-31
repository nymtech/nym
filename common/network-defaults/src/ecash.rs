// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

/// Specifies the maximum validity of the issued ticketbooks.
pub const TICKETBOOK_VALIDITY_DAYS: u32 = 7;

/// Specifies the number of tickets in each issued ticketbook.
pub const TICKETBOOK_SIZE: u64 = 50;

/// This type is defined mostly for the purposes of having constants (like sizes) associated with given variants
/// It's not meant to be serialised or have any fancy traits defined on it (in this crate)
#[derive(Default, Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum TicketTypeRepr {
    #[default]
    V1MixnetEntry = 0,
    V1MixnetExit = 1,
    V1WireguardEntry = 2,
    V1WireguardExit = 3,
}

impl TicketTypeRepr {
    pub const WIREGUARD_ENTRY_TICKET_SIZE: u64 = 500 * 1024 * 1024; // 500 MB

    // TBD:
    pub const WIREGUARD_EXIT_TICKET_SIZE: u64 = Self::WIREGUARD_ENTRY_TICKET_SIZE;
    pub const MIXNET_ENTRY_TICKET_SIZE: u64 = Self::WIREGUARD_ENTRY_TICKET_SIZE;
    pub const MIXNET_EXIT_TICKET_SIZE: u64 = Self::WIREGUARD_ENTRY_TICKET_SIZE;

    /// How much bandwidth (in bytes) one ticket can grant
    pub const fn bandwidth_value(&self) -> u64 {
        match self {
            TicketTypeRepr::V1MixnetEntry => Self::MIXNET_ENTRY_TICKET_SIZE,
            TicketTypeRepr::V1MixnetExit => Self::MIXNET_EXIT_TICKET_SIZE,
            TicketTypeRepr::V1WireguardEntry => Self::WIREGUARD_ENTRY_TICKET_SIZE,
            TicketTypeRepr::V1WireguardExit => Self::WIREGUARD_EXIT_TICKET_SIZE,
        }
    }
}

// Constants for bloom filter for double spending detection
//Chosen for FP of
//Calculator at https://hur.st/bloomfilter/
pub const ECASH_DS_BLOOMFILTER_PARAMS: BloomfilterParameters = BloomfilterParameters {
    num_hashes: 10,
    bitmap_size: 1_500_000_000,
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
