// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

// Wait time for the verification to take place (currently 24h)
pub(crate) const BLOCK_TIME_FOR_VERIFICATION_SECS: u64 = 86400;

/// The longest any single ceremony phase may be configured to last: 30 days.
///
/// A ceiling on fat fingers rather than a design number. `Timestamp::plus_seconds` multiplies
/// into nanoseconds under overflow checks, so an absurd duration would not make a long phase
/// but panic at the next transition, taking every advance and the forced reset with it, and
/// only another migrate could get the ceremony moving again.
pub(crate) const MAX_PHASE_DURATION_SECS: u64 = 30 * 24 * 60 * 60;

pub(crate) const VK_SHARES_PK_NAMESPACE: &str = "vksp";
pub(crate) const VK_SHARES_EPOCH_ID_IDX_NAMESPACE: &str = "vkse";
