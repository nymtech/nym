// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::http::state::nyx_upgrade_mode::UpgradeModeState;
use nym_crypto::asymmetric::ed25519;
use nym_http_api_client::generate_user_agent;
use nym_upgrade_mode_check::attempt_retrieve_attestation;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};
use url::Url;

pub struct AttestationWatcher {
    // default polling interval
    regular_polling_interval: Duration,

    // expedited polling interval once upgrade mode is detected
    expedited_poll_interval: Duration,

    attestation_url: Url,

    jwt_signing_keys: ed25519::KeyPair,

    jwt_validity: Duration,

    cancellation_token: CancellationToken,

    upgrade_mode_state: UpgradeModeState,
}

impl AttestationWatcher {
    pub(crate) fn new(
        regular_polling_interval: Duration,
        expedited_poll_interval: Duration,
        attestation_url: Url,
        jwt_signing_keys: ed25519::KeyPair,
        jwt_validity: Duration,
        cancellation_token: CancellationToken,
    ) -> Self {
        AttestationWatcher {
            regular_polling_interval,
            expedited_poll_interval,
            attestation_url,
            jwt_signing_keys,
            jwt_validity,
            cancellation_token,
            upgrade_mode_state: UpgradeModeState {
                inner: Arc::new(Default::default()),
            },
        }
    }

    pub(crate) fn shared_state(&self) -> UpgradeModeState {
        self.upgrade_mode_state.clone()
    }

    async fn try_update_state(&self) {
        match attempt_retrieve_attestation(
            self.attestation_url.as_str(),
            Some(generate_user_agent!()),
        )
        .await
        {
            Err(err) => error!("failed to retrieve attestation information: {err}"),
            Ok(attestation) => {
                self.upgrade_mode_state
                    .update(attestation, &self.jwt_signing_keys, self.jwt_validity)
                    .await
            }
        }
    }

    pub async fn run_forever(self) {
        info!("starting the attestation watcher task");

        let check_wait = tokio::time::sleep(self.regular_polling_interval);
        tokio::pin!(check_wait);

        loop {
            tokio::select! {
                biased;
                _ = self.cancellation_token.cancelled() => {
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
