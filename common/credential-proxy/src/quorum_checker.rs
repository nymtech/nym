// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::CredentialProxyError;
use crate::shared_state::nyxd_client::ChainClient;
use nym_ecash_signer_check::{check_known_dealers, dkg_details_with_client};
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct QuorumState {
    available: Arc<AtomicBool>,
}

impl QuorumState {
    pub fn available(&self) -> bool {
        self.available.load(Ordering::Acquire)
    }
}

pub struct QuorumStateChecker {
    client: ChainClient,
    cancellation_token: CancellationToken,
    check_interval: Duration,
    quorum_state: QuorumState,

    /// indicates whether the last check has been a failure
    last_failed: bool,
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
            last_failed: false,
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
        info!("checking the current quorum state");
        let client_guard = self.client.query_chain().await;

        // split the operation as we only need to hold the reference to chain client for the first part
        // and the second half doesn't rely on it (and takes way longer)
        let dkg_details = dkg_details_with_client(client_guard.deref()).await?;
        drop(client_guard);

        let res = check_known_dealers(dkg_details, 4).await?;
        info!("there are {} known DKG dealers", res.results.len());

        let Some(signing_threshold) = res.threshold else {
            warn!(
                "signing threshold is currently unavailable and we have not yet implemented credential issuance during DKG transition"
            );
            return Ok(false);
        };

        let mut working_issuer = 0;

        for result in res.results {
            let dealer = &result.information;
            let info = format!("[id: {}] @ {}", dealer.node_index, dealer.announce_address);
            if result.chain_available() && result.signing_available() {
                info!("✅ {info} is fully available");
                working_issuer += 1;
            } else if !result.chain_available() && !result.signing_available() {
                warn!("❌ {info} is not available for both chain and signing");
            } else if !result.chain_available() {
                warn!("❌ {info} is not available for chain");
            } else {
                warn!("❌ {info} is not available for signing");
            }
        }

        let available = (working_issuer as u64) >= signing_threshold;

        if available {
            info!(
                "✅ Quorum state is available with {working_issuer} out of {signing_threshold} issuers"
            )
        } else {
            error!(
                "❌ Quorum state is not available with {working_issuer} out of {signing_threshold} issuers"
            )
        }

        Ok(available)
    }

    pub async fn run_forever(mut self) {
        info!("starting quorum state checker");
        loop {
            tokio::select! {
                biased;
                _ = self.cancellation_token.cancelled() => {
                    break
                }
                _ = tokio::time::sleep(self.check_interval) => {
                    match self.check_quorum_state().await {
                        Ok(available) => {
                            let previous = self.quorum_state.available.load(Ordering::SeqCst);
                            // only update the quorum state to a failed state if we've had two consecutive failures
                            if available {
                                if !previous {
                                    info!("quorum recovered");
                                }
                                self.quorum_state.available.store(true, Ordering::SeqCst);
                            } else if self.last_failed {
                                if previous {
                                    warn!("quorum became unavailable after 2 consecutive failed checks");
                                }
                                self.quorum_state.available.store(false, Ordering::SeqCst);
                            }

                            self.last_failed = !available;
                        },
                        Err(err) => error!("failed to check current quorum state: {err}"),
                    }
                }
            }
        }
    }
}
