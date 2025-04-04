// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

/// Specifies the maximum validity of the issued ticketbooks.
pub const TICKETBOOK_VALIDITY_DAYS: u32 = 7;

/// Specifies the number of tickets in each issued ticketbook.
pub const TICKETBOOK_SIZE: u64 = 50;

/// Specifies the minimum request size each signer must support
pub const MINIMUM_TICKETBOOK_DATA_REQUEST_SIZE: usize = 50;

/// This type is defined mostly for the purposes of having constants (like sizes) associated with given variants
/// It's not meant to be serialised or have any fancy traits defined on it (in this crate)
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum TicketTypeRepr {
    V1MixnetEntry = 0,
    V1MixnetExit = 1,
    V1WireguardEntry = 2,
    V1WireguardExit = 3,
}

impl TicketTypeRepr {
    pub const WIREGUARD_ENTRY_TICKET_SIZE: u64 = 500 * 1000 * 1000; // 500 MB
    pub const WIREGUARD_EXIT_TICKET_SIZE: u64 = 500 * 1000 * 1000; // 500 MB
    pub const MIXNET_ENTRY_TICKET_SIZE: u64 = 200 * 1000 * 1000; // 200 MB
    pub const MIXNET_EXIT_TICKET_SIZE: u64 = 100 * 1000 * 1000; // 100 MB

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
