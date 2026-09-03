// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Signer-failure tolerance for the session's credential fetcher.
//!
//! The distributed ecash signers (and the nym-apis aggregating them) can be
//! unresponsive for long stretches — observed on mainnet as an endpoint that
//! accepts the connection and never responds. Without a bound, that hang
//! propagates into the bandwidth controller's run loop: the freshly issued
//! (paid-for) ticketbook is never persisted and provisioning blocks forever.
//!
//! [`TimeoutFetcher`] decorates any [`CredentialFetcher`] so the three
//! read-only global-signing-data fetches (master verification key, coin-index
//! signatures, expiration-date signatures) are bounded by a per-call timeout.
//! A bounded failure is recoverable: the controller's ticketbook-store path is
//! best-effort per step, so the ticketbook is persisted anyway and the missing
//! signing data is fetched later (background reconciliation or spend time) —
//! without a new deposit.
//!
//! The ticketbook-issuance call ([`CredentialFetcher::fetch_ticketbooks`]) is
//! deliberately NOT timed: it deposits funds on-chain, and interrupting it is
//! governed by the existing cancellation-safety + pending-request recovery
//! guarantees, not by a fetch timeout.

use std::time::Duration;

use async_trait::async_trait;
use nym_bandwidth_controller::error::FetcherErrorKind;
use nym_bandwidth_controller::{
    CredentialFetcher, CredentialFetcherError, CredentialPublicDataFetcher, FetcherError,
    NymCredential, TicketType,
};
use nym_credentials::{
    AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures, EpochVerificationKey,
};
use nym_ecash_time::Date;
use nym_validator_client::nym_api::EpochId;

/// Default per-call bound for the read-only global-signing-data fetches. A
/// healthy signer answers in well under a second; this is ~50x that, so it only
/// ever fires on genuinely unresponsive infrastructure while turning an
/// infinite hang into a bounded delay.
pub const DEFAULT_PUBLIC_DATA_TIMEOUT: Duration = Duration::from_secs(15);

/// A global-signing-data fetch exceeded its per-call bound. Surfaced through
/// the controller as a fetch failure so readiness reporting names the cause.
#[derive(Debug, thiserror::Error)]
#[error("ecash signers unresponsive: fetching {what} did not complete within {timeout:?}")]
pub struct SignerTimeout {
    what: &'static str,
    timeout: Duration,
}

impl FetcherError for SignerTimeout {
    fn kind(&self) -> FetcherErrorKind {
        // a nym-api / ecash query failure — transient from the caller's view
        FetcherErrorKind::Api
    }
}

/// Decorator over a [`CredentialFetcher`] bounding each read-only public-data
/// fetch with a per-call timeout (see module docs for why issuance is exempt).
pub struct TimeoutFetcher<F> {
    inner: F,
    per_call: Duration,
}

impl<F> TimeoutFetcher<F> {
    /// Wrap `inner` with the [default per-call bound](DEFAULT_PUBLIC_DATA_TIMEOUT).
    pub fn new(inner: F) -> Self {
        Self::with_timeout(inner, DEFAULT_PUBLIC_DATA_TIMEOUT)
    }

    /// Wrap `inner` with a custom per-call bound.
    pub fn with_timeout(inner: F, per_call: Duration) -> Self {
        TimeoutFetcher { inner, per_call }
    }

    /// Run `fut` bounded by the per-call timeout, mapping elapse to [`SignerTimeout`].
    async fn bounded<T>(
        &self,
        what: &'static str,
        fut: impl std::future::Future<Output = Result<T, CredentialFetcherError>>,
    ) -> Result<T, CredentialFetcherError> {
        match tokio::time::timeout(self.per_call, fut).await {
            Ok(res) => res,
            Err(_elapsed) => Err(SignerTimeout {
                what,
                timeout: self.per_call,
            }
            .into()),
        }
    }
}

#[async_trait]
impl<F: CredentialPublicDataFetcher> CredentialPublicDataFetcher for TimeoutFetcher<F> {
    async fn fetch_master_verification_key(
        &self,
        epoch_id: EpochId,
    ) -> Result<EpochVerificationKey, CredentialFetcherError> {
        self.bounded(
            "the master verification key",
            self.inner.fetch_master_verification_key(epoch_id),
        )
        .await
    }

    async fn fetch_coin_index_signatures(
        &self,
        epoch_id: EpochId,
    ) -> Result<AggregatedCoinIndicesSignatures, CredentialFetcherError> {
        self.bounded(
            "coin-index signatures",
            self.inner.fetch_coin_index_signatures(epoch_id),
        )
        .await
    }

    async fn fetch_expiration_date_signatures(
        &self,
        expiration_date: Date,
        epoch_id: EpochId,
    ) -> Result<AggregatedExpirationDateSignatures, CredentialFetcherError> {
        self.bounded(
            "expiration-date signatures",
            self.inner
                .fetch_expiration_date_signatures(expiration_date, epoch_id),
        )
        .await
    }
}

#[async_trait]
impl<F: CredentialFetcher> CredentialFetcher for TimeoutFetcher<F> {
    /// NOT timed — issuance deposits funds on-chain; see module docs.
    async fn fetch_ticketbooks(
        &self,
        ticketbook_type: TicketType,
    ) -> Result<Vec<NymCredential>, CredentialFetcherError> {
        self.inner.fetch_ticketbooks(ticketbook_type).await
    }

    async fn cleanup(&self) {
        self.inner.cleanup().await
    }

    async fn reset(self) -> Result<(), CredentialFetcherError> {
        self.inner.reset().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// How a [`StubFetcher`] call behaves. `Ok` outcomes are irrelevant to the
    /// timeout semantics (the decorator wraps the future, not its value), so
    /// completion is proven by a distinguishable *inner* error surfacing —
    /// avoiding the need to fabricate ecash values here.
    #[derive(Clone, Copy)]
    enum Mode {
        /// Accept the call and never complete (an unresponsive signer).
        Hang,
        /// Complete with the inner error after this long.
        ErrAfter(Duration),
        /// Complete with the inner error immediately.
        Err,
    }

    #[derive(Debug, thiserror::Error)]
    #[error("stub inner error")]
    struct StubError;

    impl FetcherError for StubError {
        fn kind(&self) -> FetcherErrorKind {
            FetcherErrorKind::Other
        }
    }

    struct StubFetcher {
        mode: Mode,
    }

    impl StubFetcher {
        async fn act<T>(&self) -> Result<T, CredentialFetcherError> {
            match self.mode {
                Mode::Hang => std::future::pending().await,
                Mode::ErrAfter(d) => {
                    tokio::time::sleep(d).await;
                    Err(StubError.into())
                }
                Mode::Err => Err(StubError.into()),
            }
        }
    }

    #[async_trait]
    impl CredentialPublicDataFetcher for StubFetcher {
        async fn fetch_master_verification_key(
            &self,
            _epoch_id: EpochId,
        ) -> Result<EpochVerificationKey, CredentialFetcherError> {
            self.act().await
        }

        async fn fetch_coin_index_signatures(
            &self,
            _epoch_id: EpochId,
        ) -> Result<AggregatedCoinIndicesSignatures, CredentialFetcherError> {
            self.act().await
        }

        async fn fetch_expiration_date_signatures(
            &self,
            _expiration_date: Date,
            _epoch_id: EpochId,
        ) -> Result<AggregatedExpirationDateSignatures, CredentialFetcherError> {
            self.act().await
        }
    }

    #[async_trait]
    impl CredentialFetcher for StubFetcher {
        async fn fetch_ticketbooks(
            &self,
            _ticketbook_type: TicketType,
        ) -> Result<Vec<NymCredential>, CredentialFetcherError> {
            self.act().await
        }

        async fn cleanup(&self) {}

        async fn reset(self) -> Result<(), CredentialFetcherError> {
            Ok(())
        }
    }

    fn today() -> Date {
        nym_ecash_time::ecash_today_date()
    }

    fn is_signer_timeout(err: &CredentialFetcherError) -> bool {
        err.to_string().contains("ecash signers unresponsive")
    }

    fn is_stub_error(err: &CredentialFetcherError) -> bool {
        err.to_string().contains("stub inner error")
    }

    const PER_CALL: Duration = Duration::from_secs(15);

    fn fetcher(mode: Mode) -> TimeoutFetcher<StubFetcher> {
        TimeoutFetcher::with_timeout(StubFetcher { mode }, PER_CALL)
    }

    /// 3.2: a hanging public-data fetch yields a bounded `SignerTimeout`
    /// instead of hanging — for each of the three fetches. The paused clock
    /// auto-advances, so an actual hang would fail the test harness, not CI.
    #[tokio::test(start_paused = true)]
    async fn hanging_public_data_fetch_times_out() {
        let f = fetcher(Mode::Hang);

        let err = f
            .fetch_expiration_date_signatures(today(), 0)
            .await
            .expect_err("must not hang");
        assert!(is_signer_timeout(&err), "got: {err}");

        let err = f
            .fetch_master_verification_key(0)
            .await
            .expect_err("must not hang");
        assert!(is_signer_timeout(&err), "got: {err}");

        let err = f
            .fetch_coin_index_signatures(0)
            .await
            .expect_err("must not hang");
        assert!(is_signer_timeout(&err), "got: {err}");
    }

    /// 3.3 (under threshold): an inner call that completes just before the
    /// bound surfaces its own outcome — proving the decorator waited.
    #[tokio::test(start_paused = true)]
    async fn slow_fetch_under_threshold_completes() {
        let f = fetcher(Mode::ErrAfter(PER_CALL - Duration::from_secs(1)));
        let err = f
            .fetch_expiration_date_signatures(today(), 0)
            .await
            .expect_err("stub errors after delay");
        assert!(
            is_stub_error(&err),
            "inner outcome must pass through: {err}"
        );
    }

    /// 3.3 (over threshold): an inner call that would complete just after the
    /// bound is cut off by `SignerTimeout` instead.
    #[tokio::test(start_paused = true)]
    async fn slow_fetch_over_threshold_times_out() {
        let f = fetcher(Mode::ErrAfter(PER_CALL + Duration::from_secs(1)));
        let err = f
            .fetch_expiration_date_signatures(today(), 0)
            .await
            .expect_err("must time out");
        assert!(is_signer_timeout(&err), "got: {err}");
    }

    /// 3.4: a genuine inner error passes through unaltered (no timeout wrapping).
    #[tokio::test(start_paused = true)]
    async fn immediate_inner_error_passes_through() {
        let f = fetcher(Mode::Err);
        let err = f
            .fetch_expiration_date_signatures(today(), 0)
            .await
            .expect_err("stub errors");
        assert!(is_stub_error(&err), "got: {err}");
    }

    /// 3.5: `fetch_ticketbooks` (deposit + issuance) is NOT timed. If the
    /// decorator (wrongly) applied the per-call bound, the hanging inner call
    /// would resolve to `SignerTimeout` at 15s — well inside the 1h outer
    /// probe. The outer probe elapsing therefore proves issuance is unbounded
    /// by the decorator. (Virtual clock: the hour passes instantly.)
    #[tokio::test(start_paused = true)]
    async fn fetch_ticketbooks_is_not_timed() {
        let f = fetcher(Mode::Hang);
        let probe = tokio::time::timeout(
            Duration::from_secs(3600),
            f.fetch_ticketbooks(TicketType::V1WireguardEntry),
        )
        .await;
        assert!(
            probe.is_err(),
            "issuance must not be bounded by the public-data timeout"
        );
    }
}
