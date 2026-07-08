// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::future::{poll_fn, Future};
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;

use nym_credentials_interface::TicketType;
use nym_task::ShutdownToken;
use tokio::sync::oneshot;

use crate::traits::{CredentialFetcher, CredentialFetcherError};
use crate::NymCredential;

/// Outcome of the cancellable fetching task.
/// Some(result) if it ran to completion, `None` if it was cancelled
type CredentialResult = Option<Result<Vec<NymCredential>, CredentialFetcherError>>;

/// Outcome of awaiting a fetch task's result channel: the fetched credentials (or a fetcher
/// error), `Ok(None)` if the task was cancelled, `Err(RecvError)` if the task dropped its sender (panic).
pub(crate) type FetchResult = Result<CredentialResult, oneshot::error::RecvError>;

/// A single background ticketbook fetch.
///
/// The result travels back over the `oneshot`: a task that finishes normally sends `Some(result)`,
/// a task cancelled via `cancel` sends `None`, and a task that panics drops the sender without
/// sending, so the receiver resolves to `Err(RecvError)`.
struct InFlightFetch {
    // cancels the in-flight fetch (reset / shutdown); the cancellation still drains as `Ok(None)`
    cancel: ShutdownToken,
    result: oneshot::Receiver<CredentialResult>,
}

/// Tracks background ticketbook fetches keyed per ticket type, so the same type is never requested
/// twice while a fetch is pending. Owns each fetch's cancel handle and result receiver; the
/// controller drains completions via [`Self::next_result`] and persists them.
///
/// results are reported back over a channel
#[derive(Default)]
pub(crate) struct InFlightFetches {
    fetches: HashMap<TicketType, InFlightFetch>,
}

impl InFlightFetches {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.fetches.is_empty()
    }

    pub(crate) fn contains(&self, ticket_type: TicketType) -> bool {
        self.fetches.contains_key(&ticket_type)
    }

    /// Spawns a background fetch for `ticketbook_type` and tracks it. The caller is expected to
    /// skip types already in flight (see [`Self::contains`]).
    pub(crate) fn spawn(&mut self, ticket_type: TicketType, fetcher: Arc<dyn CredentialFetcher>) {
        let cancel = ShutdownToken::new();
        let (tx, result) = oneshot::channel();
        let task_cancel = cancel.clone();
        nym_task::spawn_future(async move {
            // If the task succeeds, we return Some(result)
            // If it gets cancelled it will be `None`
            // If it panics, tx will be dropped and `rx` will observe a `RecvError`
            // In all cases, we have a result
            let res = task_cancel
                .run_until_cancelled(fetcher.fetch_ticketbooks(ticket_type))
                .await;
            let _ = tx.send(res);
        });
        self.fetches
            .insert(ticket_type, InFlightFetch { cancel, result });
    }

    /// Awaits the first tracked fetch to complete, then forgets it, yielding its ticket type and
    /// the received value.
    ///
    /// Cancel-safe, so it can sit in a `select!`: the completed fetch is removed only after its
    /// result is ready, with no `await` in between. With no fetches tracked this stays pending
    /// forever - guard the call site with [`Self::is_empty`].
    pub(crate) async fn next_result(&mut self) -> (TicketType, FetchResult) {
        let (ticket_type, result) = poll_fn(|cx| {
            for (typ, fetch) in self.fetches.iter_mut() {
                if let Poll::Ready(res) = Pin::new(&mut fetch.result).poll(cx) {
                    return Poll::Ready((*typ, res));
                }
            }
            Poll::Pending
        })
        .await;

        self.fetches.remove(&ticket_type);
        (ticket_type, result)
    }

    /// Cancels every in-flight fetch, stopping the underlying network work. Entries are left in
    /// place so each cancellation drains as `Ok(None)` through [`Self::next_result`] and is removed
    /// then, rather than being dropped silently.
    pub(crate) fn cancel_all(&self) {
        for fetch in self.fetches.values() {
            fetch.cancel.cancel();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::FetcherErrorKind;
    use crate::traits::{CredentialPublicDataFetcher, FetcherError};
    use async_trait::async_trait;
    use nym_credentials::ecash::bandwidth::serialiser::keys::EpochVerificationKey;
    use nym_credentials::ecash::bandwidth::serialiser::signatures::{
        AggregatedCoinIndicesSignatures, AggregatedExpirationDateSignatures,
    };
    use nym_ecash_time::Date;
    use nym_validator_client::nym_api::EpochId;

    const TYPE: TicketType = TicketType::V1MixnetEntry;

    #[derive(Debug, thiserror::Error)]
    #[error("mock fetch failure")]
    struct MockFetchError;

    impl FetcherError for MockFetchError {
        fn kind(&self) -> FetcherErrorKind {
            FetcherErrorKind::Other
        }
    }

    /// What the mock's `fetch_ticketbooks` should do when driven.
    enum Behaviour {
        Succeed,
        Fail,
        /// never resolves - lets us drive the cancellation path
        Hang,
        Panic,
    }

    struct MockFetcher {
        behaviour: Behaviour,
    }

    fn fetcher(behaviour: Behaviour) -> Arc<dyn CredentialFetcher> {
        Arc::new(MockFetcher { behaviour })
    }

    #[async_trait]
    impl CredentialFetcher for MockFetcher {
        async fn fetch_ticketbooks(
            &self,
            _ticketbook_type: TicketType,
        ) -> Result<Vec<NymCredential>, CredentialFetcherError> {
            match self.behaviour {
                Behaviour::Succeed => Ok(Vec::new()),
                Behaviour::Fail => Err(MockFetchError.into()),
                Behaviour::Hang => std::future::pending().await,
                Behaviour::Panic => panic!("mock fetch panic"),
            }
        }

        async fn cleanup(&self) {}

        async fn reset(self) -> Result<(), CredentialFetcherError> {
            Ok(())
        }
    }

    // the controller only calls `fetch_ticketbooks` in these tests; the public-data methods are
    // never driven, so they just error out.
    #[async_trait]
    impl CredentialPublicDataFetcher for MockFetcher {
        async fn fetch_master_verification_key(
            &self,
            _epoch_id: EpochId,
        ) -> Result<EpochVerificationKey, CredentialFetcherError> {
            Err(MockFetchError.into())
        }

        async fn fetch_coin_index_signatures(
            &self,
            _epoch_id: EpochId,
        ) -> Result<AggregatedCoinIndicesSignatures, CredentialFetcherError> {
            Err(MockFetchError.into())
        }

        async fn fetch_expiration_date_signatures(
            &self,
            _expiration_date: Date,
            _epoch_id: EpochId,
        ) -> Result<AggregatedExpirationDateSignatures, CredentialFetcherError> {
            Err(MockFetchError.into())
        }
    }

    #[tokio::test]
    async fn tracks_a_spawned_fetch() {
        let mut fetches = InFlightFetches::new();
        assert!(fetches.is_empty());

        fetches.spawn(TYPE, fetcher(Behaviour::Hang));

        assert!(fetches.contains(TYPE));
        assert!(!fetches.is_empty());
    }

    #[tokio::test]
    async fn completed_fetch_bubbles_up_and_frees_slot() {
        let mut fetches = InFlightFetches::new();
        fetches.spawn(TYPE, fetcher(Behaviour::Succeed));

        let (typ, result) = fetches.next_result().await;

        assert_eq!(typ, TYPE);
        assert!(matches!(result, Ok(Some(Ok(_)))));
        assert!(fetches.is_empty());
    }

    #[tokio::test]
    async fn failed_fetch_bubbles_up_as_ok_some_err() {
        let mut fetches = InFlightFetches::new();
        fetches.spawn(TYPE, fetcher(Behaviour::Fail));

        let (typ, result) = fetches.next_result().await;

        assert_eq!(typ, TYPE);
        assert!(matches!(result, Ok(Some(Err(_)))));
        assert!(fetches.is_empty());
    }

    #[tokio::test]
    async fn cancelled_fetch_bubbles_up_as_ok_none() {
        let mut fetches = InFlightFetches::new();
        fetches.spawn(TYPE, fetcher(Behaviour::Hang));

        fetches.cancel_all();
        let (typ, result) = fetches.next_result().await;

        assert_eq!(typ, TYPE);
        assert!(matches!(result, Ok(None)));
        assert!(fetches.is_empty());
    }

    #[tokio::test]
    async fn panicked_fetch_bubbles_up_as_recv_error_and_frees_slot() {
        let mut fetches = InFlightFetches::new();
        fetches.spawn(TYPE, fetcher(Behaviour::Panic));

        // the spawned task panics; tokio isolates it and the dropped sender surfaces as RecvError,
        // freeing the slot so the type can be retried (a panic backtrace is printed - expected).
        let (typ, result) = fetches.next_result().await;

        assert_eq!(typ, TYPE);
        assert!(result.is_err());
        assert!(fetches.is_empty());
    }
}
