// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

pub const NODE_INDEX: &str = "node_index";
pub const DKG_PROPOSAL_ID: &str = "proposal_id";

/// Emitted (with the held epoch's id as the value) when an epoch-state advance is held
/// because nobody has registered as a dealer - without it, the held transaction succeeds
/// looking exactly like a real advance, and a ceremony stuck waiting for dealers can only
/// be noticed by diffing successive epoch queries.
pub const AWAITING_DEALERS: &str = "awaiting_dealers";
