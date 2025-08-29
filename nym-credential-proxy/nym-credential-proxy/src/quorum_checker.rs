// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::CredentialProxyError;
use crate::http::state::ChainClient;
use nym_ecash_signer_check::{check_known_dealers, dkg_details_with_client};
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

#[derive(Clone)]
pub(crate) struct QuorumState {
    available: Arc<AtomicBool>,
}

impl QuorumState {
    pub(crate) fn available(&self) -> bool {
        self.available.load(Ordering::Acquire)
    }
}

pub(crate) struct QuorumStateChecker {
    client: ChainClient,
    cancellation_token: CancellationToken,
    check_interval: Duration,
    quorum_state: QuorumState,
}

impl QuorumStateChecker {
    pub async fn new(
        client: ChainClient,
        check_interval: Duration,
        cancellation_token: CancellationToken,
    ) -> Result<Self, CredentialProxyError> {
        let this = QuorumStateChecker {
            client,
            cancellation_token,
            check_interval,
            quorum_state: QuorumState {
                available: Arc::new(Default::default()),
            },
        };

        // first check MUST succeed, otherwise we shouldn't start
        let quorum_available = this.check_quorum_state().await?;
        this.quorum_state
            .available
            .store(quorum_available, Ordering::Relaxed);
        Ok(this)
    }

    pub fn quorum_state_ref(&self) -> QuorumState {
        self.quorum_state.clone()
    }

    async fn check_quorum_state(&self) -> Result<bool, CredentialProxyError> {
        let client_guard = self.client.query_chain().await;

        // split the operation as we only need to hold the reference to chain client for the first part
        // and the second half doesn't rely on it (and takes way longer)
        let dkg_details = dkg_details_with_client(client_guard.deref()).await?;
        drop(client_guard);

        let res = check_known_dealers(dkg_details).await?;

        let Some(signing_threshold) = res.threshold else {
            warn!("signing threshold is currently unavailable and we have not yet implemented credential issuance during DKG transition");
            return Ok(false);
        };

        let mut working_issuer = 0;

        for result in res.results {
            if result.chain_available() && result.signing_available() {
                working_issuer += 1;
            }
        }

        Ok((working_issuer as u64) >= signing_threshold)
    }

    pub async fn run_forever(self) {
        info!("starting quorum state checker");
        loop {
            tokio::select! {
                biased;
                _ = self.cancellation_token.cancelled() => {
                    break
                }
                _ = tokio::time::sleep(self.check_interval) => {
                    match self.check_quorum_state().await {
                        Ok(available) => self.quorum_state.available.store(available, Ordering::SeqCst),
                        Err(err) => error!("failed to check current quorum state: {err}"),
                    }
                }
            }
        }
    }
}
