// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// event types
pub const DEPOSITED_FUNDS_EVENT_TYPE: &str = "deposited-funds";

// a 'wasm-' prefix is added to all cosmwasm events
pub const COSMWASM_DEPOSITED_FUNDS_EVENT_TYPE: &str = "wasm-deposited-funds";

pub const DEPOSIT_ID: &str = "deposit-id";

pub const TICKET_BOOK_VALUE: u128 = 50_000_000;
pub const TICKET_VALUE: u128 = 50_000;

pub const WASM_EVENT_NAME: &str = "wasm";
pub const BLACKLIST_PROPOSAL_ID: &str = "proposal_id";
pub const BLACKLIST_PROPOSAL_REPLY_ID: u64 = 7759;
