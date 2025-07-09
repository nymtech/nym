// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_ecash_signer_check::SignerResult;

pub(crate) mod data;
pub(crate) mod refresher;

pub(crate) struct SignersCacheData {
    pub(crate) signer_results: Vec<SignerResult>,
}
