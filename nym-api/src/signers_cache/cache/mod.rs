// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_ecash_signer_check::SignersTestResult;
use serde::{Deserialize, Serialize};

pub(crate) mod data;
pub(crate) mod refresher;

#[derive(Serialize, Deserialize)]
pub(crate) struct SignersCacheData {
    pub(crate) signers_results: SignersTestResult,
}
