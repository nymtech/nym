// Copyright 2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::ecash;
use nym_bin_common::bin_info;
use nym_bin_common::build_information::BinaryBuildInformation;
use std::ops::Deref;
use std::sync::Arc;
use tokio::time::Instant;

pub(crate) mod handlers;

#[derive(Clone)]
pub(crate) struct ApiStatusState {
    inner: Arc<ApiStatusStateInner>,
}

impl Deref for ApiStatusState {
    type Target = ApiStatusStateInner;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub(crate) struct ApiStatusStateInner {
    startup_time: Instant,
    build_information: BinaryBuildInformation,
    signer_information: Option<SignerState>,
}

pub(crate) struct SignerState {
    // static information
    pub cosmos_address: String,

    pub identity: String,

    pub announce_address: String,

    pub(crate) ecash_keypair: ecash::keys::KeyPair,
}

impl ApiStatusState {
    pub fn new(signer_information: Option<SignerState>) -> Self {
        ApiStatusState {
            inner: Arc::new(ApiStatusStateInner {
                startup_time: Instant::now(),
                build_information: bin_info!(),
                signer_information,
            }),
        }
    }
}
