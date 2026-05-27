// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Event names and attribute keys emitted by the ecash contract. Renaming any
//! of these is a breaking change for indexers and downstream tooling.

/// Event type emitted by every successful `DepositTicketBookFunds`. Carries a
/// single `deposit-id` attribute with the assigned id as a decimal string.
pub const DEPOSITED_FUNDS_EVENT_TYPE: &str = "deposited-funds";

/// Attribute key on the `deposited-funds` event: the newly assigned deposit id.
pub const DEPOSIT_ID: &str = "deposit-id";

/// Name of the cosmwasm-std auto-generated event that carries handler
/// attributes (`updated_deposit`, `action`, `address`, `deposit`,
/// `proposal_id`).
pub const WASM_EVENT_NAME: &str = "wasm";

/// Attribute key carrying the multisig-issued `proposal_id` on the `wasm`
/// event from the redemption-proposal reply handler.
pub const PROPOSAL_ID_ATTRIBUTE_NAME: &str = "proposal_id";
