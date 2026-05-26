// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Reply-id constants for the contract's reply dispatcher. Both ids are part
//! of the public contract surface - changing them between versions invalidates
//! any in-flight submessage.

/// Reply id for the cw3 propose dispatched by the (stubbed) blacklist flow.
/// Wired but unreachable from the public ExecuteMsg surface today.
pub const BLACKLIST_PROPOSAL_REPLY_ID: u64 = 7759;

/// Reply id for the cw3 propose dispatched by `RequestRedemption`. The handler
/// captures the multisig-issued `proposal_id` and re-exposes it as the
/// response data.
pub const REDEMPTION_PROPOSAL_REPLY_ID: u64 = 2137;
