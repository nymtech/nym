// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::error::CredentialProxyError;
use crate::shared_state::nyxd_client::ChainClient;
use nym_ecash_signer_check::{check_known_dealers, dkg_details_with_client};
use nym_validator_client::nym_api::EpochId;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct QuorumState {
    available: Arc<AtomicBool>,
    checked_epoch: Arc<AtomicU64>,
}

impl QuorumState {
    pub fn rules_out(&self, epoch_id: EpochId) -> bool {
        !self.available.load(Ordering::Acquire)
            && self.checked_epoch.load(Ordering::Acquire) == epoch_id
    }

    fn record(&self, epoch_id: EpochId, available: bool) {
        self.available.store(available, Ordering::Release);
        self.checked_epoch.store(epoch_id, Ordering::Release);
    }
}

#[cfg(test)]
impl QuorumState {
    /// A fixed answer, for tests exercising code that carries this state without reading it.
    pub(crate) fn fixed(available: bool) -> Self {
        Self::checked(0, available)
    }

    pub(crate) fn checked(epoch_id: EpochId, available: bool) -> Self {
        QuorumState {
            available: Arc::new(AtomicBool::new(available)),
            checked_epoch: Arc::new(AtomicU64::new(epoch_id)),
        }
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
                checked_epoch: Arc::new(Default::default()),
            },
            last_failed: false,
        };

        // first check MUST succeed, otherwise we shouldn't start
        let (epoch_id, quorum_available) = this.check_quorum_state().await?;
        this.quorum_state.record(epoch_id, quorum_available);
        Ok(this)
    }

    pub fn quorum_state_ref(&self) -> QuorumState {
        self.quorum_state.clone()
    }

    /// The epoch the check was made against, and whether it found a quorum for it.
    async fn check_quorum_state(&self) -> Result<(EpochId, bool), CredentialProxyError> {
        info!("checking the current quorum state");
        let client_guard = self.client.query_chain().await;

        // split the operation as we only need to hold the reference to chain client for the first part
        // and the second half doesn't rely on it (and takes way longer)
        let dkg_details = dkg_details_with_client(client_guard.deref()).await?;
        drop(client_guard);

        let epoch_id = dkg_details.dkg_epoch.epoch_id;
        let res = check_known_dealers(dkg_details, 4).await?;
        info!("there are {} known DKG dealers", res.results.len());

        let Some(signing_threshold) = res.threshold else {
            warn!(
                "signing threshold is currently unavailable and we have not yet implemented credential issuance during DKG transition"
            );
            return Ok((epoch_id, false));
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

        Ok((epoch_id, available))
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
                        Ok((epoch_id, available)) => {
                            let previous = self.quorum_state.available.load(Ordering::SeqCst);
                            // only update the quorum state to a failed state if we've had two consecutive failures
                            if available {
                                if !previous {
                                    info!("quorum recovered");
                                }
                                self.quorum_state.record(epoch_id, true);
                            } else if self.last_failed {
                                if previous {
                                    warn!("quorum became unavailable after 2 consecutive failed checks");
                                }
                                self.quorum_state.record(epoch_id, false);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failed_check_rules_out_the_epoch_it_was_made_against() {
        assert!(QuorumState::checked(5, false).rules_out(5));
    }

    #[test]
    fn a_check_made_during_a_ceremony_rules_out_nothing_for_the_epoch_in_service() {
        assert!(!QuorumState::checked(6, false).rules_out(5));
    }

    #[test]
    fn an_available_quorum_rules_out_nothing() {
        assert!(!QuorumState::checked(5, true).rules_out(5));
    }
}
