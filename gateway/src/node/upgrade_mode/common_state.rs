// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::node::upgrade_mode::watcher::UpgradeModeCheckRequestSender;
use crate::node::upgrade_mode::UpgradeModeEnableError;
use nym_credential_verification::upgrade_mode::UpgradeModeState;
use nym_upgrade_mode_check::{validate_upgrade_mode_jwt, CREDENTIAL_PROXY_JWT_ISSUER};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;

#[derive(Clone, Copy)]
pub(crate) struct Config {
    /// The minimum duration since the last explicit check to allow creation of separate request.
    pub min_staleness_recheck: Duration,
}

#[derive(Clone)]
pub struct UpgradeModeCommon {
    pub(crate) config: Config,
    pub(crate) request_checker: UpgradeModeCheckRequestSender,
    pub(crate) state: UpgradeModeState,
}

impl UpgradeModeCommon {
    pub(crate) fn new(
        config: Config,
        request_checker: UpgradeModeCheckRequestSender,
        state: UpgradeModeState,
    ) -> Self {
        UpgradeModeCommon {
            config,
            request_checker,
            state,
        }
    }

    pub(crate) fn enabled(&self) -> bool {
        self.state.upgrade_mode_enabled()
    }

    pub(crate) fn since_last_query(&self) -> Duration {
        self.state.since_last_query()
    }

    pub(crate) fn can_request_recheck(&self) -> bool {
        self.since_last_query() > self.config.min_staleness_recheck
    }

    // explicitly request state update. this is only called when upgrade mode is NOT enabled,
    // and client has sent a JWT instead of ticket
    pub(crate) async fn request_recheck(&self) -> bool {
        // send request
        let on_done = Arc::new(Notify::new());
        self.request_checker.send_request(on_done.clone());

        // wait for response - note, if we fail to send, notification will be sent regardless,
        // so that we wouldn't get stuck in here
        on_done.notified().await;

        // check the state again
        self.enabled()
    }

    pub(crate) async fn try_enable_via_received_jwt(
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
