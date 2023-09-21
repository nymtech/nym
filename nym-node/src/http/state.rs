// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_bin_common::bin_info_owned;
use nym_bin_common::build_information::BinaryBuildInformationOwned;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

impl AppState {
    pub fn new_dummy() -> Self {
        AppState {
            inner: Arc::new(AppStateInner {
                build_information: bin_info_owned!(),
            }),
        }
    }
}

impl Deref for AppState {
    type Target = AppStateInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Debug)]
pub struct AppStateInner {
    // TODO: split it based on routes?
    pub(crate) build_information: BinaryBuildInformationOwned,
}
