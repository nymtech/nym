// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_bin_common::build_information::BinaryBuildInformationOwned;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(crate) struct AppState {
    inner: Arc<AppStateInner>,
}

#[derive(Debug)]
struct AppStateInner {
    // TODO: split it based on routes?
    pub(crate) build_information: BinaryBuildInformationOwned,
}
