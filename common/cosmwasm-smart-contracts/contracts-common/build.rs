// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use vergen::EmitBuilder;

fn main() {
    EmitBuilder::builder()
        .all_build()
        .rustc_semver()
        .git_branch()
        .git_commit_timestamp()
        .git_sha(false)
        .cargo_debug()
        .cargo_opt_level()
        .emit()
        .expect("failed to extract build metadata");
}
