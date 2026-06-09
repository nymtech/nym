// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

/// Title used for the cw3 `Propose` message dispatched by `RequestRedemption`.
/// nym-api signers cross-check this exact string when validating that an
/// in-flight proposal originated from the ecash contract.
// TODO: to be moved to multisig
pub const BATCH_REDEMPTION_PROPOSAL_TITLE: &str = "ecash-redemption";
