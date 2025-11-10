// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::http::state::nyx_upgrade_mode::UpgradeModeState;
use nym_crypto::asymmetric::ed25519;
use nym_http_api_client::generate_user_agent;
use nym_upgrade_mode_check::attempt_retrieve_attestation;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};
use url::Url;

/// Specifies the threshold for retrieval failures that will trigger disabling upgrade mode.
/// This assumes the file has been removed incorrectly and has been replaced by some placeholder 404
/// page that does not deserialise correctly
const FAILURE_THRESHOLD: usize = 10;

pub struct AttestationWatcher {
    // default polling interval
    regular_polling_interval: Duration,

    // expedited polling interval once upgrade mode is detected
    expedited_poll_interval: Duration,

    attestation_url: Url,

    expected_attester_public_key: ed25519::PublicKey,

    jwt_signing_keys: ed25519::KeyPair,

    jwt_validity: Duration,

    upgrade_mode_state: UpgradeModeState,

    consecutive_retrieval_failures: usize,
}

impl AttestationWatcher {
    pub(crate) fn new(
        regular_polling_interval: Duration,
        expedited_poll_interval: Duration,
        expected_attester_public_key: ed25519::PublicKey,
        attestation_url: Url,
        jwt_signing_keys: ed25519::KeyPair,
        jwt_validity: Duration,
    ) -> Self {
        AttestationWatcher {
            regular_polling_interval,
            expedited_poll_interval,
            attestation_url,
            expected_attester_public_key,
            jwt_signing_keys,
            jwt_validity,
            upgrade_mode_state: UpgradeModeState {
                inner: Arc::new(Default::default()),
            },
            consecutive_retrieval_failures: 0,
        }
    }

    pub(crate) fn shared_state(&self) -> UpgradeModeState {
        self.upgrade_mode_state.clone()
    }

    async fn try_update_state(&mut self) {
        match attempt_retrieve_attestation(
            self.attestation_url.as_str(),
            Some(generate_user_agent!()),
        )
        .await
        {
            Err(err) => {
                self.consecutive_retrieval_failures += 1;
                info!("upgrade mode attestation is not available at this time");
                debug!("retrieval error: {err}");

                if self.upgrade_mode_state.has_attestation()
                    && self.consecutive_retrieval_failures > FAILURE_THRESHOLD
                {
                    self.upgrade_mode_state
                        .update(
                            None,
                            self.expected_attester_public_key,
                            &self.jwt_signing_keys,
                            self.jwt_validity,
                        )
                        .await
                }
            }
            Ok(attestation) => {
                self.consecutive_retrieval_failures = 0;

                self.upgrade_mode_state
                    .update(
                        attestation,
                        self.expected_attester_public_key,
                        &self.jwt_signing_keys,
                        self.jwt_validity,
                    )
                    .await
            }
        }
    }

    pub async fn run_forever(mut self, cancellation_token: CancellationToken) {
        info!("starting the attestation watcher task");

        // make sure the first check happens immediately
        let check_wait = tokio::time::sleep(Duration::new(0, 0));
        tokio::pin!(check_wait);

        loop {
            tokio::select! {
                biased;
                _ = cancellation_token.cancelled() => {
                    break
                }
                _ = &mut check_wait => {
                    self.try_update_state().await;
                    if self.upgrade_mode_state.has_attestation().await {
                        check_wait.as_mut().reset(Instant::now() + self.expedited_poll_interval);
                    } else {
                        check_wait.as_mut().reset(Instant::now() + self.regular_polling_interval)
                    }
                }
            }
        }
    }
}
