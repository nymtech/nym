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

use crate::in_flight::global_data::{GlobalData, GlobalDataRequest};
use crate::traits::{CredentialFetcher, CredentialFetcherError, CredentialPublicDataFetcher};
use crate::NymCredential;

pub(crate) mod global_data;

/// Identifies (and thereby de-duplicates) a background fetch: a single [`InFlightFetches`] map holds
/// both ticketbook and global-signing-data fetches, keyed by this.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum FetchKey {
    Ticketbook(TicketType),
    GlobalData(GlobalDataRequest),
}

impl From<TicketType> for FetchKey {
    fn from(ticket_type: TicketType) -> Self {
        Self::Ticketbook(ticket_type)
    }
}

impl From<GlobalDataRequest> for FetchKey {
    fn from(request: GlobalDataRequest) -> Self {
        Self::GlobalData(request)
    }
}

/// Everything needed to run one background fetch: what to fetch, plus the fetcher to run it with.
///
/// Kept separate from [`FetchKey`] (the map key) because a fetcher isn't hashable - the key is
/// derived from the job via [`FetchJob::key`].
pub(crate) enum FetchJob {
    Ticketbook {
        ticket_type: TicketType,
        fetcher: Arc<dyn CredentialFetcher>,
    },
    GlobalData {
        request: GlobalDataRequest,
        fetcher: Arc<dyn CredentialPublicDataFetcher>,
    },
}

impl FetchJob {
    fn key(&self) -> FetchKey {
        match self {
            FetchJob::Ticketbook { ticket_type, .. } => (*ticket_type).into(),
            FetchJob::GlobalData { request, .. } => (*request).into(),
        }
    }

    /// Runs the fetch and tags the result with what kind it was, so completion handling doesn't
    /// have to correlate it back to the key.
    async fn run(self) -> Result<FetchedData, CredentialFetcherError> {
        match self {
            FetchJob::Ticketbook {
                ticket_type,
                fetcher,
            } => fetcher
                .fetch_ticketbooks(ticket_type)
                .await
                .map(FetchedData::Ticketbooks),
            FetchJob::GlobalData { request, fetcher } => {
                request.fetch(&*fetcher).await.map(FetchedData::GlobalData)
            }
        }
    }
}

/// What a completed fetch produced, for the controller to persist. The variant always matches the
/// [`FetchKey`] the fetch was spawned under.
pub(crate) enum FetchedData {
    Ticketbooks(Vec<NymCredential>),
    GlobalData(GlobalData),
}

/// What a finished fetch task hands back: `Some(result)` if it ran to completion (success or fetcher
/// error), or `None` if it was cancelled before finishing.
type FinishedFetch = Option<Result<FetchedData, CredentialFetcherError>>;

/// What draining a completed fetch yields: the [`FinishedFetch`], or `Err(RecvError)` if the task
/// dropped its result sender without sending - i.e. it panicked.
pub(crate) type FetchResult = Result<FinishedFetch, oneshot::error::RecvError>;

/// A single tracked fetch: the handle to cancel it, and the channel its result comes back on.
struct TrackedFetch {
    // cancels the in-flight fetch (reset / shutdown); the cancellation still drains as `Ok(None)`
    cancel: ShutdownToken,
    result: oneshot::Receiver<FinishedFetch>,
}

/// Tracks background fetches keyed by [`FetchKey`], so the same key is never requested twice while a
/// fetch is pending. Owns each fetch's cancel handle and result receiver; the controller drains
/// completions via [`Self::next_result`] and persists them.
#[derive(Default)]
pub(crate) struct InFlightFetches {
    fetches: HashMap<FetchKey, TrackedFetch>,
}

impl InFlightFetches {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.fetches.is_empty()
    }

    pub(crate) fn is_in_flight(&self, key: impl Into<FetchKey>) -> bool {
        self.fetches.contains_key(&key.into())
    }

    /// The ticket types whose fetch is currently in flight.
    pub(crate) fn in_flight_ticketbooks(&self) -> Vec<TicketType> {
        self.fetches
            .keys()
            .filter_map(|key| match key {
                FetchKey::Ticketbook(ticket_type) => Some(*ticket_type),
                FetchKey::GlobalData(_) => None,
            })
            .collect()
    }

    /// The global-data pieces whose fetch is currently in flight.
    pub(crate) fn in_flight_global_data(&self) -> Vec<GlobalDataRequest> {
        self.fetches
            .keys()
            .filter_map(|key| match key {
                FetchKey::GlobalData(request) => Some(*request),
                FetchKey::Ticketbook(_) => None,
            })
            .collect()
    }

    /// Spawns a background fetch and tracks it, unless one is already in flight for the same key.
    pub(crate) fn spawn(&mut self, job: FetchJob) {
        let key = job.key();
        if self.is_in_flight(key) {
            tracing::warn!("a {key:?} fetch is already in flight; not spawning a duplicate");
            return;
        }
        let cancel = ShutdownToken::new();
        let (tx, result) = oneshot::channel();
        self.fetches.insert(
            key,
            TrackedFetch {
                cancel: cancel.clone(),
                result,
            },
        );
        nym_task::spawn_future(async move {
            let finished = cancel.run_until_cancelled(job.run()).await;
            let _ = tx.send(finished);
        });
    }

    /// Awaits the first tracked fetch to complete, then forgets it, yielding its key and result.
    ///
    /// Cancel-safe, so it can sit in a `select!`: the completed fetch is removed only after its
    /// result is ready, with no `await` in between. With no fetches tracked this stays pending
    /// forever - guard the call site with [`Self::is_empty`].
    pub(crate) async fn next_result(&mut self) -> (FetchKey, FetchResult) {
        let (key, result) = poll_fn(|cx| {
            for (key, fetch) in self.fetches.iter_mut() {
                if let Poll::Ready(res) = Pin::new(&mut fetch.result).poll(cx) {
                    return Poll::Ready((*key, res));
                }
            }
            Poll::Pending
        })
        .await;

        self.fetches.remove(&key);
        (key, result)
    }

    /// Cancels every in-flight fetch, stopping the underlying work. Entries are left in place so
    /// each cancellation drains as `Ok(None)` through [`Self::next_result`] and is removed then,
    /// rather than being dropped silently.
    pub(crate) fn cancel_all(&self) {
        for fetch in self.fetches.values() {
            fetch.cancel.cancel();
        }
    }

    /// Cancels every in-flight fetch and waits for each task to observe the cancellation and
    /// finish, discarding their results, leaving the map empty.
    pub(crate) async fn cancel_and_join(&mut self) {
        self.cancel_all();
        while !self.is_empty() {
            let _ = self.next_result().await;
        }
    }
}

impl Drop for InFlightFetches {
    /// Cancel all tasks
    fn drop(&mut self) {
        self.cancel_all();
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

    fn key() -> FetchKey {
        FetchKey::Ticketbook(TYPE)
    }

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

    fn job(ticket_type: TicketType, behaviour: Behaviour) -> FetchJob {
        FetchJob::Ticketbook {
            ticket_type,
            fetcher: fetcher(behaviour),
        }
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

        fetches.spawn(job(TYPE, Behaviour::Hang));

        assert!(fetches.is_in_flight(TYPE));
        assert!(!fetches.is_empty());
    }

    #[tokio::test]
    async fn completed_fetch_bubbles_up_and_frees_slot() {
        let mut fetches = InFlightFetches::new();
        fetches.spawn(job(TYPE, Behaviour::Succeed));

        let (fetched_key, result) = fetches.next_result().await;

        assert_eq!(fetched_key, key());
        assert!(matches!(result, Ok(Some(Ok(FetchedData::Ticketbooks(_))))));
        assert!(fetches.is_empty());
    }

    #[tokio::test]
    async fn failed_fetch_bubbles_up_as_ok_some_err() {
        let mut fetches = InFlightFetches::new();
        fetches.spawn(job(TYPE, Behaviour::Fail));

        let (fetched_key, result) = fetches.next_result().await;

        assert_eq!(fetched_key, key());
        assert!(matches!(result, Ok(Some(Err(_)))));
        assert!(fetches.is_empty());
    }

    #[tokio::test]
    async fn cancelled_fetch_bubbles_up_as_ok_none() {
        let mut fetches = InFlightFetches::new();
        fetches.spawn(job(TYPE, Behaviour::Hang));

        fetches.cancel_all();
        let (fetched_key, result) = fetches.next_result().await;

        assert_eq!(fetched_key, key());
        assert!(matches!(result, Ok(None)));
        assert!(fetches.is_empty());
    }

    #[tokio::test]
    async fn spawning_a_duplicate_type_is_refused_and_leaves_the_first_fetch_intact() {
        let mut fetches = InFlightFetches::new();
        fetches.spawn(job(TYPE, Behaviour::Hang));
        // second spawn for the same type must not overwrite/orphan the first
        fetches.spawn(job(TYPE, Behaviour::Succeed));

        assert_eq!(fetches.fetches.len(), 1);
        // the surviving fetch is the original hanging one: it only drains once cancelled
        fetches.cancel_all();
        let (fetched_key, result) = fetches.next_result().await;
        assert_eq!(fetched_key, key());
        assert!(matches!(result, Ok(None)));
    }

    #[tokio::test]
    async fn cancel_and_join_drains_everything_and_empties_the_map() {
        let mut fetches = InFlightFetches::new();
        fetches.spawn(job(TicketType::V1MixnetEntry, Behaviour::Hang));
        fetches.spawn(job(TicketType::V1MixnetExit, Behaviour::Hang));

        fetches.cancel_and_join().await;

        assert!(fetches.is_empty());
    }

    #[tokio::test]
    async fn panicked_fetch_bubbles_up_as_recv_error_and_frees_slot() {
        let mut fetches = InFlightFetches::new();
        fetches.spawn(job(TYPE, Behaviour::Panic));

        // the spawned task panics; tokio isolates it and the dropped sender surfaces as RecvError,
        // freeing the slot so the type can be retried (a panic backtrace is printed - expected).
        let (fetched_key, result) = fetches.next_result().await;

        assert_eq!(fetched_key, key());
        assert!(result.is_err());
        assert!(fetches.is_empty());
    }
}
