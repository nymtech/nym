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
    max_retries: u32,
    retry_initial_delay: Duration,
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
            max_retries: 3,
            retry_initial_delay: Duration::from_secs(2),
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

    fn is_retryable_error(&self, err: &CredentialProxyError) -> bool {
        let err_str = err.to_string().to_lowercase();

        // Check for DNS-related errors
        if err_str.contains("dns")
            || err_str.contains("lookup")
            || err_str.contains("name resolution")
            || err_str.contains("temporary failure")
            || err_str.contains("failed to lookup address")
        {
            return true;
        }

        // Check if it's a Tendermint RPC error (which could be DNS/timeout related)
        if let CredentialProxyError::NyxdFailure { source: nyxd_err } = err {
            let nyxd_err_str = nyxd_err.to_string().to_lowercase();
            if nyxd_err_str.contains("tendermint rpc request failed") {
                return true;
            }

            if nyxd_err.is_tendermint_response_timeout() {
                return true;
            }
        }

        false
    }

    async fn check_quorum_state(&self) -> Result<bool, CredentialProxyError> {
        self.check_quorum_state_with_retry().await
    }

    async fn check_quorum_state_with_retry(&self) -> Result<bool, CredentialProxyError> {
        let mut last_error_msg = None;
        let delay = self.retry_initial_delay;

        for attempt in 0..=self.max_retries {
            match self.check_quorum_state_once().await {
                Ok(result) => {
                    if attempt > 0 {
                        info!("quorum check succeeded after {} retry attempt(s)", attempt);
                    }
                    return Ok(result);
                }
                Err(err) => {
                    let err_msg = err.to_string();

                    // Check if this error is retryable
                    if !self.is_retryable_error(&err) {
                        return Err(err);
                    }

                    last_error_msg = Some(err_msg.clone());

                    if attempt >= self.max_retries {
                        break;
                    }

                    // Log the retry attempt
                    warn!(
                        "quorum check failed (attempt {}/{}): {}. Retrying in {:?}...",
                        attempt + 1,
                        self.max_retries + 1,
                        err_msg,
                        delay
                    );

                    // Wait before retrying with exponential backoff
                    tokio::time::sleep(delay).await;
                }
            }
        }

        // try one final time to get the actual error
        match self.check_quorum_state_once().await {
            Ok(result) => {
                warn!(
                    "quorum check succeeded on final attempt after {} retries",
                    self.max_retries
                );
                Ok(result)
            }
            Err(err) => {
                if let Some(error_msg) = last_error_msg {
                    error!(
                        "quorum check failed after {} retry attempts. Last error: {}",
                        self.max_retries + 1,
                        error_msg
                    );
                }
                Err(err)
            }
        }
    }

    async fn check_quorum_state_once(&self) -> Result<bool, CredentialProxyError> {
        let client_guard = self.client.query_chain().await;

        // split the operation as we only need to hold the reference to chain client for the first part
        // and the second half doesn't rely on it (and takes way longer)
        let dkg_details = dkg_details_with_client(client_guard.deref()).await?;
        drop(client_guard);

        let res = check_known_dealers(dkg_details).await?;

        let Some(signing_threshold) = res.threshold else {
            warn!(
                "signing threshold is currently unavailable and we have not yet implemented credential issuance during DKG transition"
            );
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
                    match self.check_quorum_state_with_retry().await {
                        Ok(available) => self.quorum_state.available.store(available, Ordering::SeqCst),
                        Err(err) => error!("failed to check current quorum state: {err}"),
                    }
                }
            }
        }
    }
}
