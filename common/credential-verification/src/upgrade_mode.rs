// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_upgrade_mode_check::UpgradeModeAttestation;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use time::OffsetDateTime;
use tokio::sync::RwLock;

struct UpgradeModeState {
    // very much tbd
    inner: Arc<UpgradeModeStateInner>,
}

impl UpgradeModeState {
    pub fn upgrade_mode_enabled(&self) -> bool {
        self.inner.upgrade_mode_enabled.load(Ordering::Acquire)
    }

    pub fn enable_upgrade_mode(&self) {
        self.inner
            .upgrade_mode_enabled
            .store(true, Ordering::Release);
    }

    pub fn disable_upgrade_mode(&self) {
        self.inner
            .upgrade_mode_enabled
            .store(false, Ordering::Release);
    }

    pub fn last_queried(&self) -> OffsetDateTime {
        // SAFETY: the stored value here is always a valid unix timestamp
        #[allow(clippy::unwrap_used)]
        OffsetDateTime::from_unix_timestamp(self.inner.last_queried_ts.load(Ordering::Acquire))
            .unwrap()
    }

    pub fn update_last_queried(&self, queried_at: OffsetDateTime) {
        self.inner
            .last_queried_ts
            .store(queried_at.unix_timestamp(), Ordering::Release);
    }
}

struct UpgradeModeDetails {
    expected_attestation: Option<UpgradeModeAttestation>,
}

struct UpgradeModeStateInner {
    /// Contents of the published upgrade mode attestation, as queried by this node
    expected_attestation: RwLock<Option<UpgradeModeAttestation>>,

    /// timestamp indicating last time this node has queried for the current upgrade mode attestation
    /// it is used to determine if an additional expedited query should be made in case client sends a JWT
    /// whilst this node is not aware of the upgrade mode
    last_queried_ts: AtomicI64,

    /// flag indicating whether upgrade mode is currently enabled. this is to perform cheap checks
    /// that avoid having to acquire the lock
    // (and dealing with the async consequences of that)
    upgrade_mode_enabled: AtomicBool,
}
