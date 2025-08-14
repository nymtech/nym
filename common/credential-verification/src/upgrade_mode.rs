// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_upgrade_mode_check::UpgradeModeAttestation;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::time::Duration;
use time::OffsetDateTime;
use tokio::sync::RwLock;
use tracing::error;

#[derive(Clone)]
pub struct UpgradeModeState {
    inner: Arc<UpgradeModeStateInner>,
}

#[derive(Clone, Default)]
pub struct UpgradeModeStatus(Arc<AtomicBool>);

impl UpgradeModeStatus {
    pub fn enabled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }

    pub fn enable(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn disable(&self) {
        self.0.store(false, Ordering::Release);
    }
}

impl UpgradeModeState {
    pub fn new_empty() -> UpgradeModeState {
        UpgradeModeState {
            inner: Arc::new(UpgradeModeStateInner {
                expected_attestation: RwLock::new(None),
                last_queried_ts: AtomicI64::new(OffsetDateTime::UNIX_EPOCH.unix_timestamp()),
                status: UpgradeModeStatus(Arc::new(AtomicBool::new(false))),
            }),
        }
    }

    pub async fn attestation(&self) -> Option<UpgradeModeAttestation> {
        self.inner.expected_attestation.read().await.clone()
    }

    pub async fn set_expected_attestation(
        &self,
        expected_attestation: Option<UpgradeModeAttestation>,
    ) {
        let mut guard = self.inner.expected_attestation.write().await;
        // make sure to only enable upgrade mode flag AFTER we have written the expected value
        // (or still hold the exclusive lock as in this instance)
        if expected_attestation.is_some() {
            self.enable_upgrade_mode()
        } else {
            self.disable_upgrade_mode()
        }
        self.update_last_queried(OffsetDateTime::now_utc());
        *guard = expected_attestation;
    }

    pub fn upgrade_mode_status(&self) -> UpgradeModeStatus {
        self.inner.status.clone()
    }

    pub fn upgrade_mode_enabled(&self) -> bool {
        self.inner.status.enabled()
    }

    pub fn enable_upgrade_mode(&self) {
        self.inner.status.enable()
    }

    pub fn disable_upgrade_mode(&self) {
        self.inner.status.disable()
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

    pub fn since_last_query(&self) -> Duration {
        (OffsetDateTime::now_utc() - self.last_queried())
            .try_into()
            .unwrap_or_else(|_| {
                error!("somehow our last query for upgrade mode was in the future!");
                Duration::ZERO
            })
    }
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
    status: UpgradeModeStatus,
}
