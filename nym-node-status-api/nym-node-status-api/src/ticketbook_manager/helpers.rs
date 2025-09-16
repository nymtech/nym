// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use nym_bin_common::bin_info;
use tracing::error;

#[allow(clippy::panic)]
pub(crate) fn build_sha_short() -> &'static str {
    let bin_info = bin_info!();
    if bin_info.commit_sha.len() < 7 {
        panic!("unavailable build commit sha")
    }

    if bin_info.commit_sha == "VERGEN_IDEMPOTENT_OUTPUT" {
        error!("the binary hasn't been built correctly. it doesn't have a commit sha information");
        return "unknown";
    }

    &bin_info.commit_sha[..7]
}
