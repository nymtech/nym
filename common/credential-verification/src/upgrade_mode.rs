// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use nym_upgrade_mode_check::{
    CREDENTIAL_PROXY_JWT_ISSUER, UpgradeModeAttestation, validate_upgrade_mode_jwt,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::time::Duration;
use thiserror::Error;
use time::OffsetDateTime;
use tokio::sync::{Notify, RwLock};
use tracing::{debug, error};

#[derive(Debug, Error)]
pub enum UpgradeModeEnableError {
    #[error("too soon to perform another upgrade mode attestation check")]
    TooManyRecheckRequests,

    #[error("provided upgrade mode JWT is invalid: {0}")]
    InvalidUpgradeModeJWT(#[from] nym_upgrade_mode_check::UpgradeModeCheckError),

    #[error("the upgrade mode attestation does not appear to have been published")]
    AttestationNotPublished,

    #[error("the provided upgrade mode attestation is different from the published one")]
    MismatchedUpgradeModeAttestation,
}

// the idea behind this is as follows:
// it's been relatively a long time since the watcher last performed its checks (since it's in 'regular' mode)
// and some client has just sent a JWT. we have to retrieve most recent information in case upgrade mode
// has just been enabled, and we haven't learned about it yet
#[derive(Clone)]
pub struct UpgradeModeCheckRequestSender(Option<UnboundedSender<CheckRequest>>);

impl UpgradeModeCheckRequestSender {
    pub fn new(sender: UnboundedSender<CheckRequest>) -> Self {
        UpgradeModeCheckRequestSender(Some(sender))
    }

    pub fn new_empty() -> Self {
        Self(None)
    }

    pub(crate) fn send_request(&self, on_done: Arc<Notify>) {
        let Some(ref inner) = self.0 else {
            // make sure the caller gets notified so it doesn't wait forever
            on_done.notify_waiters();
            return;
        };

        if let Err(not_sent) = inner.unbounded_send(CheckRequest { on_done }) {
            debug!("failed to send upgrade mode check request - {not_sent}");
            // make sure the caller gets notified so it doesn't wait forever
            not_sent.into_inner().on_done.notify_waiters();
        }
    }
}

pub type UpgradeModeCheckRequestReceiver = UnboundedReceiver<CheckRequest>;

pub struct CheckRequest {
    on_done: Arc<Notify>,
}

impl CheckRequest {
    pub fn finalize(self) {
        self.on_done.notify_waiters();
    }
}

#[derive(Clone, Copy)]
pub struct UpgradeModeCheckConfig {
    /// The minimum duration since the last explicit check to allow creation of separate request.
    pub min_staleness_recheck: Duration,
}

/// Full upgrade mode information, that apart from boolean flag indicating the state
/// and the attestation information, includes channel connection to relevant
/// attestation watcher to request state rechecks
#[derive(Clone)]
pub struct UpgradeModeDetails {
    pub(crate) config: UpgradeModeCheckConfig,
    pub(crate) request_checker: UpgradeModeCheckRequestSender,
    pub(crate) state: UpgradeModeState,
}

impl UpgradeModeDetails {
    pub fn new(
        config: UpgradeModeCheckConfig,
        request_checker: UpgradeModeCheckRequestSender,
        state: UpgradeModeState,
    ) -> Self {
        UpgradeModeDetails {
            config,
            request_checker,
            state,
        }
    }

    pub fn enabled(&self) -> bool {
        self.state.upgrade_mode_enabled()
    }

    fn since_last_query(&self) -> Duration {
        self.state.since_last_query()
    }

    pub fn can_request_recheck(&self) -> bool {
        self.since_last_query() > self.config.min_staleness_recheck
    }

    // explicitly request state update. this is only called when upgrade mode is NOT enabled,
    // and client has sent a JWT instead of ticket
    async fn request_recheck(&self) -> bool {
        // send request
        let on_done = Arc::new(Notify::new());
        self.request_checker.send_request(on_done.clone());

        // wait for response - note, if we fail to send, notification will be sent regardless,
        // so that we wouldn't get stuck in here
        on_done.notified().await;

        // check the state again
        self.enabled()
    }

    pub async fn try_enable_via_received_jwt(
        &self,
        token: String,
    ) -> Result<(), UpgradeModeEnableError> {
        // see if it's viable to perform another expedited check
        if !self.can_request_recheck() {
            return Err(UpgradeModeEnableError::TooManyRecheckRequests);
        }

        // first validate whether the received JWT is even valid
        // note: we expect the token has been signed by our credential proxy
        // (in the future, we won't care about it, and we'll have proper key discovery endpoint. 2026™️)
        let attestation = validate_upgrade_mode_jwt(&token, Some(CREDENTIAL_PROXY_JWT_ISSUER))?;

        // send request to revalidate internal state
        self.request_recheck().await;

        // not strictly necessary, but check if provided attestation actually matches the one retrieved
        // (if any)
        let Some(retrieved_attestation) = self.state.attestation().await else {
            return Err(UpgradeModeEnableError::AttestationNotPublished);
        };
        if retrieved_attestation != attestation {
            return Err(UpgradeModeEnableError::MismatchedUpgradeModeAttestation);
        }

        // note: if attestation has been returned, it means we're definitely in upgrade mode
        // (otherwise it wouldn't have existed in the state)

        Ok(())
    }
}

/// Detailed upgrade mode information, that apart from boolean flag,
/// also includes, if applicable, the associated attestation
#[derive(Clone)]
pub struct UpgradeModeState {
    inner: Arc<UpgradeModeStateInner>,
}

/// Just a shareable flag to indicate whether upgrade mode is enabled or disabled
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
