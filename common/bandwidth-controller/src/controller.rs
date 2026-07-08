// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::BandwidthControllerConfig;
use crate::error::BandwidthControllerError;
use crate::readiness::{FetchFailure, ReadinessRequest, ReadinessSnapshot, ReadinessStatus};
use crate::requests::{BandwidthControllerRequest, BandwidthControllerRequestSender};
use crate::ticketbooks::AvailableTicketbooks;
use crate::traits::{CredentialFetcher, CredentialPublicDataFetcher};
use crate::NymCredential;
use crate::{
    BandwidthTicketProvider, PreparedCredential, PreparedCredentialMetadata, UPGRADE_MODE_JWT_TYPE,
};

use nym_credential_storage::models::EmergencyCredentialContent;
use nym_credential_storage::models::RetrievedTicketbook;
use nym_credential_storage::storage::Storage;
use nym_credentials::ecash::bandwidth::CredentialSpendingData;
use nym_credentials::IssuedTicketBook;
use nym_credentials_interface::{
    AnnotatedCoinIndexSignature, AnnotatedExpirationDateSignature, TicketType, VerificationKeyAuth,
};
use nym_crypto::asymmetric::ed25519;
use nym_ecash_time::{Date, OffsetDateTime};
use nym_task::ShutdownToken;
use nym_validator_client::nym_api::EpochId;

use async_trait::async_trait;
use log::error;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

#[cfg(not(target_arch = "wasm32"))]
use crate::traits::CredentialFetcherError;

/// Result of a background ticketbook fetch, drained from `fetch_tasks` in the `run` loop.
#[cfg(not(target_arch = "wasm32"))]
struct FetchOutcome {
    ticket_type: TicketType,
    result: Result<Vec<NymCredential>, CredentialFetcherError>,
}

/// Owns all ecash credential state and is the **single writer** to the credential [`Storage`].
///
/// It serves ticket spending, credential acquisition, and global signing-data retrieval while
/// keeping storage consistent by being the only component that writes to it. The acquisition and
/// retrieval work is delegated to two pluggable, **storage-free** fetchers: they only do the
/// network/cryptographic work and hand the results back for the controller to persist.
///
/// - [`CredentialPublicDataFetcher`] (`public_data_fetcher`): lazily retrieves the global ecash
///   signing materials - master verification key, coin-index and expiration-date signatures - when
///   they're missing locally.
/// - [`CredentialFetcher`] (`credential_fetcher`): provisions usable ticketbooks (making deposits
///   and aggregating wallet signatures). It is a superset of the public-data fetcher, so installing
///   one via [`Self::with_credential_fetcher`] also registers it as the public-data fetcher.
///
/// It can be driven two ways:
/// - [`Self::run`] — the event loop: handles requests from [`BandwidthControllerRequestSender`] and,
///   on native targets, proactively restocks low ticket types in the background, keeps the global
///   data topped up, and resolves `wait_for_ticketbooks` readiness waiters as fetches complete.
/// - directly on a non-running instance — e.g. [`Self::fetch_ticketbook`] or
///   [`Self::prepare_ecash_ticket`] — for one-shot use without spawning the loop.
pub struct BandwidthController<St> {
    storage: St,

    // Channels used to receive commands from the outside.
    request_channel: (
        UnboundedSender<BandwidthControllerRequest>,
        UnboundedReceiver<BandwidthControllerRequest>,
    ),

    config: BandwidthControllerConfig,

    // fetches the global ecash signing materials when they are missing locally
    public_data_fetcher: Option<Arc<dyn CredentialPublicDataFetcher>>,

    // provisions usable ticketbooks. Available on all targets for manual (inline) fetching;
    // `Arc` so the native auto path can clone it into spawned fetch tasks.
    credential_fetcher: Option<Arc<dyn CredentialFetcher>>,

    // ticket types with an automatic fetch currently in flight, so we don't request the same
    // type twice (native auto path only)
    #[cfg(not(target_arch = "wasm32"))]
    in_flight: HashSet<TicketType>,

    // background ticketbook fetches; completions are drained in the `run` loop
    #[cfg(not(target_arch = "wasm32"))]
    fetch_tasks: tokio::task::JoinSet<FetchOutcome>,

    // callers parked on `wait_for_ticketbooks`, re-evaluated whenever a fetch completes
    pending_readiness: Vec<ReadinessRequest>,
}

impl<St: Storage> BandwidthController<St> {
    // ---------------------------------------------------------------------
    // Construction & configuration
    // ---------------------------------------------------------------------

    pub fn new(storage: St) -> Self {
        let request_channel = mpsc::unbounded_channel();
        BandwidthController {
            storage,
            request_channel,
            config: Default::default(),
            public_data_fetcher: None,
            credential_fetcher: None,
            #[cfg(not(target_arch = "wasm32"))]
            in_flight: HashSet::new(),
            #[cfg(not(target_arch = "wasm32"))]
            fetch_tasks: tokio::task::JoinSet::new(),
            pending_readiness: Vec::new(),
        }
    }
    #[must_use]
    pub fn with_config(mut self, config: BandwidthControllerConfig) -> Self {
        self.config = config;
        self
    }

    #[must_use]
    pub fn with_credential_fetcher(mut self, fetcher: impl CredentialFetcher + 'static) -> Self {
        let fetcher = Arc::new(fetcher);
        self.credential_fetcher = Some(fetcher.clone());
        self.public_data_fetcher = Some(fetcher);
        self
    }

    #[must_use]
    pub fn with_credential_public_data_fetcher(
        mut self,
        fetcher: impl CredentialPublicDataFetcher + 'static,
    ) -> Self {
        self.public_data_fetcher = Some(Arc::new(fetcher));
        self
    }

    /// Get the request channel used to send request to the controller.
    /// Request are handled only if the `BandwidthController` is running using `run`
    pub fn get_request_sender(&self) -> BandwidthControllerRequestSender {
        BandwidthControllerRequestSender::new(self.request_channel.0.clone())
    }

    // ---------------------------------------------------------------------
    // Event loop
    // ---------------------------------------------------------------------

    /// Runs the controller event loop, handling incoming requests until the
    /// request channel is closed or cancellation is requested. Additionally drives
    /// the proactive restock timer and drains completed background fetches.
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn run(mut self, shutdown_token: ShutdownToken) {
        tracing::info!("BandwidthController started successfully");
        let mut topup_interval = {
            let mut interval = tokio::time::interval(self.config.topup_interval);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            interval
        };

        loop {
            tokio::select! {
                biased;
                _ = shutdown_token.cancelled() => {
                    log::debug!("bandwidth controller received cancellation request; shutting down");
                    break;
                }
                _ = topup_interval.tick() => {
                    let _ = self.print_info().await;
                    self.ensure_global_data().await;
                    self.check_and_restock(AvailableTicketbooks::ticketbook_types()).await;
                }
                Some(done) = self.fetch_tasks.join_next() => {
                    self.on_fetch_complete(done).await;
                }
                request = self.request_channel.1.recv() => match request {
                    Some(request) => self.handle_request(request).await,
                    None => {
                        log::warn!("bandwidth controller request channel closed; this should never happen as we own a sender; shutting down");
                        break;
                    }
                }
            }
        }

        if let Some(fetcher) = self.credential_fetcher {
            fetcher.cleanup().await;
        }
        self.storage.close().await;
    }

    /// Runs the controller event loop, handling incoming requests until the
    /// request channel is closed or cancellation is requested
    #[cfg(target_arch = "wasm32")]
    pub async fn run(mut self, shutdown_token: ShutdownToken) {
        let mut topup_interval = {
            let mut interval = wasmtimer::tokio::interval(self.config.topup_interval);
            interval.set_missed_tick_behavior(wasmtimer::tokio::MissedTickBehavior::Delay);
            interval
        };
        loop {
            tokio::select! {
                biased;
                _ = shutdown_token.cancelled() => {
                    log::debug!("bandwidth controller received cancellation request; shutting down");
                    break;
                }
                _ = topup_interval.tick() => {
                    let _ = self.print_info().await;
                    self.ensure_global_data().await;
                }
                request = self.request_channel.1.recv() => match request {
                    Some(request) => self.handle_request(request).await,
                    None => {
                        log::warn!("bandwidth controller request channel closed; this should never happen as we own a sender; shutting down");
                        break;
                    }
                }
            }
        }
        self.storage.close().await;
    }

    // ---------------------------------------------------------------------
    // Request dispatch & handlers
    // ---------------------------------------------------------------------

    async fn handle_request(&mut self, request: BandwidthControllerRequest) {
        match request {
            BandwidthControllerRequest::EcashTicket(return_sender, request) => {
                let ticket_type = request.ticket_type;
                let credential_result = self
                    .prepare_ecash_ticket(
                        ticket_type,
                        request.gateway_id.to_bytes(),
                        request.tickets_to_spend,
                        request.spend_time,
                    )
                    .await;
                return_sender.send(credential_result);
                // a ticket was just requested for this type - top it up if it's now running low
                #[cfg(not(target_arch = "wasm32"))]
                self.check_and_restock(vec![ticket_type]).await;
            }
            BandwidthControllerRequest::UpgradeModeToken(return_sender) => {
                return_sender.send(self.get_upgrade_mode_token().await)
            }
            BandwidthControllerRequest::AttemptRevertSpending(return_sender, metadata) => {
                return_sender.send(self.attempt_revert_ticket_usage(metadata).await)
            }
            BandwidthControllerRequest::SetCredentialFetcher(return_sender, fetcher) => {
                self.handle_set_credential_fetcher(fetcher).await;
                return_sender.send(Ok(()))
            }

            BandwidthControllerRequest::SetPublicDataFetcher(return_sender, fetcher) => {
                self.public_data_fetcher = fetcher;
                return_sender.send(Ok(()))
            }
            BandwidthControllerRequest::Reset(return_sender) => {
                return_sender.send(self.handle_reset().await)
            }
            BandwidthControllerRequest::ClearEmergencyCredentials(return_sender) => return_sender
                .send(
                    self.storage
                        .clear_emergency_credentials()
                        .await
                        .map_err(BandwidthControllerError::credential_storage_error),
                ),
            BandwidthControllerRequest::GetAvailableTicketbooks(return_sender) => {
                return_sender.send(self.handle_get_available_ticketbooks().await)
            }
            BandwidthControllerRequest::WaitForTicketbooks(return_sender, ticket_types) => {
                self.handle_wait_for_ticketbooks(ReadinessRequest {
                    return_sender,
                    ticket_types,
                })
                .await
            }
        }
    }

    async fn handle_set_credential_fetcher(&mut self, fetcher: Option<Arc<dyn CredentialFetcher>>) {
        if let Some(old_fetcher) = self.credential_fetcher.take() {
            old_fetcher.cleanup().await;
        }
        // Explicit upcast required because of the option
        self.public_data_fetcher = fetcher
            .clone()
            .map(|f| f as Arc<dyn CredentialPublicDataFetcher>);
        self.credential_fetcher = fetcher;
        #[cfg(not(target_arch = "wasm32"))]
        self.check_and_restock(AvailableTicketbooks::ticketbook_types())
            .await;
    }

    // Removes fetcher, stops in flight requests, clear credentials, answer all readiness request with unavailable
    async fn handle_reset(&mut self) -> Result<(), BandwidthControllerError> {
        // Cancel fetching
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.fetch_tasks.shutdown().await;
            self.in_flight = HashSet::new();
        }

        if let Some(fetcher) = &self.credential_fetcher {
            fetcher.cleanup().await;
        }
        self.credential_fetcher = None;

        let requests = std::mem::take(&mut self.pending_readiness);
        requests.into_iter().for_each(|r| r.cancel());

        // Clear credentials
        self.storage
            .clear_ticketbooks()
            .await
            .map_err(BandwidthControllerError::credential_storage_error)?;
        self.storage
            .clear_emergency_credentials()
            .await
            .map_err(BandwidthControllerError::credential_storage_error)
    }

    async fn handle_get_available_ticketbooks(
        &self,
    ) -> Result<AvailableTicketbooks, BandwidthControllerError> {
        self.print_info().await?;
        self.get_available_ticketbooks().await
    }

    async fn handle_wait_for_ticketbooks(&mut self, request: ReadinessRequest) {
        let snapshot = match self.build_readiness_snapshot(None).await {
            Ok(snapshot) => snapshot,
            Err(err) => {
                // transient storage failure - leave waiters parked for the next state change
                tracing::warn!("could not assess ticketbook readiness: {err}");
                self.pending_readiness.push(request);
                return;
            }
        };
        tracing::debug!("Readiness snapshot : {:#?}", snapshot);
        if let Some(request) = request.try_resolve(&snapshot) {
            self.pending_readiness.push(request);
        }
    }

    // ---------------------------------------------------------------------
    // Ticket spending
    // ---------------------------------------------------------------------

    pub async fn prepare_ecash_ticket(
        &self,
        ticketbook_type: TicketType,
        provider_pk: [u8; 32],
        tickets_to_spend: u32,
        spend_time: OffsetDateTime,
    ) -> Result<Option<PreparedCredential>, BandwidthControllerError> {
        let Some(retrieved_ticketbook) = self
            .get_next_usable_ticketbook(ticketbook_type, tickets_to_spend)
            .await?
        else {
            return Ok(None);
        };

        let ticketbook_id = retrieved_ticketbook.ticketbook_id;
        let epoch_id = retrieved_ticketbook.ticketbook.epoch_id();

        let used_tickets =
            retrieved_ticketbook.ticketbook.spent_tickets() as u32 + tickets_to_spend;
        let metadata = PreparedCredentialMetadata {
            ticketbook_id,
            tickets_withdrawn: tickets_to_spend,
            used_tickets,
        };

        match self
            .prepare_ecash_ticket_inner(
                provider_pk,
                spend_time,
                tickets_to_spend,
                retrieved_ticketbook,
            )
            .await
        {
            Ok(data) => Ok(Some(PreparedCredential {
                data,
                epoch_id,
                metadata,
            })),
            Err(err) => {
                error!("failed to prepare credential spending request. attempting to revert withdrawal...");
                self.attempt_revert_ticket_usage(metadata).await?;
                Err(err)
            }
        }
    }

    async fn prepare_ecash_ticket_inner(
        &self,
        provider_pk: [u8; 32],
        spend_time: OffsetDateTime,
        tickets_to_spend: u32,
        mut retrieved_ticketbook: RetrievedTicketbook,
    ) -> Result<CredentialSpendingData, BandwidthControllerError> {
        let epoch_id = retrieved_ticketbook.ticketbook.epoch_id();
        let expiration_date = retrieved_ticketbook.ticketbook.expiration_date();

        let verification_key = self.ensure_master_verification_key(epoch_id).await?;
        let expiration_signatures = self
            .ensure_expiration_date_signatures(epoch_id, expiration_date)
            .await?;
        let coin_indices_signatures = self.ensure_coin_index_signatures(epoch_id).await?;

        let pay_info = retrieved_ticketbook
            .ticketbook
            .generate_pay_info(provider_pk, spend_time);

        let spend_request = retrieved_ticketbook.ticketbook.prepare_for_spending(
            &verification_key,
            pay_info.into(),
            &coin_indices_signatures,
            &expiration_signatures,
            tickets_to_spend as u64,
        )?;
        Ok(spend_request)
    }

    /// Tries to retrieve one of the stored, unused credentials for the given type that hasn't yet expired.
    pub async fn get_next_usable_ticketbook(
        &self,
        ticketbook_type: TicketType,
        tickets: u32,
    ) -> Result<Option<RetrievedTicketbook>, BandwidthControllerError> {
        self.storage
            .get_next_unspent_usable_ticketbook(ticketbook_type.to_string(), tickets)
            .await
            .map_err(BandwidthControllerError::credential_storage_error)
    }

    async fn attempt_revert_ticket_usage(
        &self,
        info: PreparedCredentialMetadata,
    ) -> Result<bool, BandwidthControllerError> {
        self.storage
            .attempt_revert_ticketbook_withdrawal(
                info.ticketbook_id,
                info.tickets_withdrawn,
                info.used_tickets,
            )
            .await
            .map_err(BandwidthControllerError::credential_storage_error)
    }

    // ---------------------------------------------------------------------
    // Automatic restocking & background fetches (native only)
    // ---------------------------------------------------------------------

    /// Fetches a ticketbook of `ticketbook_type` via the configured credential fetcher and persists
    /// it (together with the global signing materials it needs). The fetch runs inline, so this
    /// works on a controller that isn't running its event loop - suitable for one-shot issuance.
    pub async fn fetch_ticketbook(
        &self,
        ticketbook_type: TicketType,
    ) -> Result<(), BandwidthControllerError> {
        let Some(fetcher) = &self.credential_fetcher else {
            return Err(BandwidthControllerError::MissingCredentialFetcher);
        };
        let credentials = fetcher
            .fetch_ticketbooks(ticketbook_type)
            .await
            .map_err(BandwidthControllerError::fetcher_error)?;
        self.store_fetched(credentials).await;
        Ok(())
    }

    /// Restocks the given ticket types that are running low or about to expire.
    #[cfg(not(target_arch = "wasm32"))]
    async fn check_and_restock(&mut self, ticketbook_types: Vec<TicketType>) {
        let available = match self.get_available_ticketbooks().await {
            Ok(available) => available,
            Err(err) => {
                tracing::warn!("could not assess ticket stock for restocking: {err}");
                return;
            }
        };

        for typ in ticketbook_types {
            tracing::debug!("Checking credential stock for {typ} ticket");
            if available.needs_restock(typ, self.config) {
                tracing::debug!("{typ} tickets need a restock");
                self.ensure_stocked(typ);
            }
        }
    }

    /// Spawns a background fetch for `ticketbook_type` unless one is already in flight for it.
    /// Non-blocking: the result is drained later in the `run` loop via `on_fetch_complete`.
    #[cfg(not(target_arch = "wasm32"))]
    fn ensure_stocked(&mut self, ticketbook_type: TicketType) {
        if self.in_flight.contains(&ticketbook_type) {
            // already fetching this type; don't ask again while we're still waiting
            tracing::debug!("{ticketbook_type} ticket restock already in flight");
            return;
        }
        let Some(fetcher) = &self.credential_fetcher else {
            tracing::debug!("No credential fetcher set. No restock possible");
            return;
        };
        let fetcher = Arc::clone(fetcher);
        self.in_flight.insert(ticketbook_type);
        tracing::debug!("requesting more {ticketbook_type} ticketbooks");
        self.fetch_tasks.spawn(async move {
            let result = fetcher.fetch_ticketbooks(ticketbook_type).await;
            FetchOutcome {
                ticket_type: ticketbook_type,
                result,
            }
        });
    }

    /// Persists the credentials returned by a completed fetch and clears its in-flight marker.
    #[cfg(not(target_arch = "wasm32"))]
    async fn on_fetch_complete(&mut self, done: Result<FetchOutcome, tokio::task::JoinError>) {
        // a failed fetch is surfaced to any readiness waiter that required the failed type
        let failure = match done {
            Ok(FetchOutcome {
                ticket_type,
                result: Ok(credentials),
            }) => {
                self.in_flight.remove(&ticket_type);
                self.store_fetched(credentials).await;
                tracing::info!("fetched and stored a {ticket_type} ticketbook");
                None
            }
            Ok(FetchOutcome {
                ticket_type,
                result: Err(err),
            }) => {
                self.in_flight.remove(&ticket_type);
                tracing::warn!("failed to fetch {ticket_type} ticketbooks: {err}");
                Some(FetchFailure {
                    ticket_type,
                    error: err,
                })
            }
            Err(join_err) => {
                // a panicking fetch loses its ticket type, so it stays marked in-flight until the
                // controller is restarted - acceptable as our fetcher isn't expected to panic.
                tracing::error!("a credential fetch task terminated abnormally: {join_err}");
                None
            }
        };
        self.resolve_pending_waiters(failure).await;
    }

    /// Persists fetched credential
    async fn store_fetched(&self, credentials: Vec<NymCredential>) {
        for credential in credentials {
            match credential {
                NymCredential::Ticketbook(ticketbook) => self.store_ticketbook(*ticketbook).await,
                NymCredential::UpgradeModeToken { jwt, expiration } => {
                    self.store_upgrade_token(jwt, expiration).await
                }
            }
        }
    }

    /// Persists fetched ticketbooks together with the global materials they need to be spent.
    ///
    /// Best-effort: each step (storing a material that came with the credential, or ensuring a
    /// missing one is fetched, or storing the ticketbook itself) is logged on failure and the rest
    /// still proceed - a single failure never aborts the batch.
    async fn store_ticketbook(&self, ticketbook: IssuedTicketBook) {
        // This path is used when a CredentialFetcher is there, so there will be a way to get the public data as well
        let epoch_id = ticketbook.epoch_id();

        if let Err(err) = self.ensure_master_verification_key(epoch_id).await {
            tracing::warn!("failed to ensure master verification key for epoch {epoch_id}: {err}");
        }
        if let Err(err) = self.ensure_coin_index_signatures(epoch_id).await {
            tracing::warn!("failed to ensure coin index signatures for epoch {epoch_id}: {err}");
        }

        if let Err(err) = self
            .ensure_expiration_date_signatures(epoch_id, ticketbook.expiration_date())
            .await
        {
            tracing::warn!(
                "failed to ensure expiration date signatures for epoch {epoch_id}: {err}"
            );
        }

        if let Err(err) = self.storage.insert_issued_ticketbook(&ticketbook).await {
            tracing::warn!("failed to store ticketbook: {err}");
        }
    }

    async fn store_upgrade_token(&self, jwt: String, expiration: OffsetDateTime) {
        let credential_content = EmergencyCredentialContent {
            typ: UPGRADE_MODE_JWT_TYPE.into(),
            content: jwt.into_bytes(),
            expiration: Some(expiration),
        };
        if let Err(e) = self
            .storage
            .insert_emergency_credential(&credential_content)
            .await
        {
            tracing::warn!("failed to store emergency credential: {e}");
        }
    }

    // ---------------------------------------------------------------------
    // Ticketbook readiness
    // ---------------------------------------------------------------------

    #[cfg(not(target_arch = "wasm32"))]
    fn is_in_flight(&self, typ: TicketType) -> bool {
        self.in_flight.contains(&typ)
    }

    #[cfg(target_arch = "wasm32")]
    fn is_in_flight(&self, _typ: TicketType) -> bool {
        false
    }

    async fn build_readiness_snapshot(
        &self,
        failure: Option<FetchFailure>,
    ) -> Result<ReadinessSnapshot, BandwidthControllerError> {
        let upgrade_mode = self.get_upgrade_mode_token().await?.is_some();
        let available = self.get_available_ticketbooks().await?;

        let mut tickets_readiness = HashMap::new();
        for typ in AvailableTicketbooks::ticketbook_types() {
            let status = if available.contains_minimal_tickets(typ, self.config) {
                ReadinessStatus::Ready
            } else if self.is_in_flight(typ) {
                ReadinessStatus::InFlight
            } else {
                match failure.as_ref() {
                    Some(failure) if failure.ticket_type == typ => {
                        ReadinessStatus::FetchFailed(failure.error.to_string())
                    }
                    _ => ReadinessStatus::Unavailable,
                }
            };
            tickets_readiness.insert(typ, status);
        }

        Ok(ReadinessSnapshot {
            upgrade_mode,
            tickets_readiness,
        })
    }

    /// Re-evaluates parked `wait_for_ticketbooks` callers after stock/in-flight state changed,
    /// answering and dropping the ones that resolved.
    #[cfg(not(target_arch = "wasm32"))]
    async fn resolve_pending_waiters(&mut self, failure: Option<FetchFailure>) {
        if self.pending_readiness.is_empty() {
            return;
        }
        let snapshot = match self.build_readiness_snapshot(failure).await {
            Ok(snapshot) => snapshot,
            Err(err) => {
                // transient storage failure - leave waiters parked for the next state change
                tracing::warn!("could not assess ticketbook readiness: {err}");
                return;
            }
        };

        tracing::debug!("Readiness snapshot : {:#?}", snapshot);

        let requests = std::mem::take(&mut self.pending_readiness);

        let still_waiting = requests
            .into_iter()
            .filter_map(|request| request.try_resolve(&snapshot))
            .collect();

        self.pending_readiness = still_waiting;
    }

    // ---------------------------------------------------------------------
    // Global signing data (fetched & cached on demand)
    // ---------------------------------------------------------------------

    /// Returns the master verification key for the epoch, fetching it via the configured public
    /// data fetcher and persisting it if it isn't already in local storage.
    async fn ensure_master_verification_key(
        &self,
        epoch_id: EpochId,
    ) -> Result<VerificationKeyAuth, BandwidthControllerError> {
        if let Some(key) = self
            .storage
            .get_master_verification_key(epoch_id)
            .await
            .map_err(BandwidthControllerError::credential_storage_error)?
        {
            return Ok(key);
        }
        let Some(fetcher) = &self.public_data_fetcher else {
            return Err(BandwidthControllerError::MissingVerificationKey { epoch_id });
        };
        let key = fetcher
            .fetch_master_verification_key(epoch_id)
            .await
            .map_err(BandwidthControllerError::fetcher_error)?;
        self.storage
            .insert_master_verification_key(&key)
            .await
            .map_err(BandwidthControllerError::credential_storage_error)?;
        Ok(key.key)
    }

    /// Returns the coin index signatures for the epoch, fetching them via the configured public
    /// data fetcher and persisting them if they aren't already in local storage.
    async fn ensure_coin_index_signatures(
        &self,
        epoch_id: EpochId,
    ) -> Result<Vec<AnnotatedCoinIndexSignature>, BandwidthControllerError> {
        if let Some(signatures) = self
            .storage
            .get_coin_index_signatures(epoch_id)
            .await
            .map_err(BandwidthControllerError::credential_storage_error)?
        {
            return Ok(signatures);
        }
        let Some(fetcher) = &self.public_data_fetcher else {
            return Err(BandwidthControllerError::MissingCoinIndexSignatures { epoch_id });
        };
        let signatures = fetcher
            .fetch_coin_index_signatures(epoch_id)
            .await
            .map_err(BandwidthControllerError::fetcher_error)?;
        self.storage
            .insert_coin_index_signatures(&signatures)
            .await
            .map_err(BandwidthControllerError::credential_storage_error)?;
        Ok(signatures.signatures)
    }

    /// Returns the expiration date signatures for the epoch and expiration date, fetching them via
    /// the configured public data fetcher and persisting them if they aren't already in local storage.
    async fn ensure_expiration_date_signatures(
        &self,
        epoch_id: EpochId,
        expiration_date: Date,
    ) -> Result<Vec<AnnotatedExpirationDateSignature>, BandwidthControllerError> {
        if let Some(signatures) = self
            .storage
            .get_expiration_date_signatures(expiration_date, epoch_id)
            .await
            .map_err(BandwidthControllerError::credential_storage_error)?
        {
            return Ok(signatures);
        }
        let Some(fetcher) = &self.public_data_fetcher else {
            return Err(BandwidthControllerError::MissingExpirationDateSignatures { epoch_id });
        };
        let signatures = fetcher
            .fetch_expiration_date_signatures(expiration_date, epoch_id)
            .await
            .map_err(BandwidthControllerError::fetcher_error)?;
        self.storage
            .insert_expiration_date_signatures(&signatures)
            .await
            .map_err(BandwidthControllerError::credential_storage_error)?;
        Ok(signatures.signatures)
    }

    /// Ensures the global signing data (master key, coin-index and expiration-date signatures) is
    /// present locally for every epoch/expiration referenced by a stored ticketbook, fetching
    /// whatever is missing.
    ///
    /// Best-effort: each missing piece is fetched independently and failures are logged, so one
    /// failure never blocks the rest.
    async fn ensure_global_data(&self) {
        let ticketbooks = match self.storage.get_ticketbooks_info().await {
            Ok(ticketbooks) => ticketbooks,
            Err(err) => {
                tracing::warn!("could not read ticketbooks to ensure global data: {err}");
                return;
            }
        };

        // master key and coin-index signatures are per-epoch
        let epochs = ticketbooks
            .iter()
            .map(|ticketbook| EpochId::from(ticketbook.epoch_id))
            .collect::<HashSet<_>>();
        for epoch_id in epochs {
            if let Err(err) = self.ensure_master_verification_key(epoch_id).await {
                tracing::warn!(
                    "failed to ensure master verification key for epoch {epoch_id}: {err}"
                );
            }
            if let Err(err) = self.ensure_coin_index_signatures(epoch_id).await {
                tracing::warn!(
                    "failed to ensure coin index signatures for epoch {epoch_id}: {err}"
                );
            }
        }

        // expiration-date signatures are keyed by (epoch, expiration date)
        let expirations = ticketbooks
            .iter()
            .map(|ticketbook| {
                (
                    EpochId::from(ticketbook.epoch_id),
                    ticketbook.expiration_date,
                )
            })
            .collect::<HashSet<_>>();
        for (epoch_id, expiration_date) in expirations {
            if let Err(err) = self
                .ensure_expiration_date_signatures(epoch_id, expiration_date)
                .await
            {
                tracing::warn!(
                    "failed to ensure expiration date signatures for epoch {epoch_id}: {err}"
                );
            }
        }
    }

    // ---------------------------------------------------------------------
    // Storage queries & diagnostics
    // ---------------------------------------------------------------------

    async fn get_upgrade_mode_token(&self) -> Result<Option<String>, BandwidthControllerError> {
        let Some(emergency_credential) = self
            .storage
            .get_emergency_credential(UPGRADE_MODE_JWT_TYPE)
            .await
            .map_err(BandwidthControllerError::credential_storage_error)?
        else {
            return Ok(None);
        };
        // upgrade mode credential is just a simple stringified JWT
        let token = String::from_utf8(emergency_credential.data.content)
            .map_err(|_| BandwidthControllerError::MalformedUpgradeModeToken)?;
        Ok(Some(token))
    }

    async fn get_available_ticketbooks(
        &self,
    ) -> Result<AvailableTicketbooks, BandwidthControllerError> {
        let ticketbooks_info = self
            .storage
            .get_ticketbooks_info()
            .await
            .map_err(BandwidthControllerError::credential_storage_error)?;
        AvailableTicketbooks::try_from(ticketbooks_info)
    }

    async fn print_info(&self) -> Result<(), BandwidthControllerError> {
        let ticketbooks_info = self.get_available_ticketbooks().await?;
        let num_ticketbooks = ticketbooks_info.len_not_expired();
        let num_total_ticketbooks = ticketbooks_info.len();
        tracing::info!("Ticketbooks stored: {num_ticketbooks}");
        tracing::debug!("Total ticketbooks stored: {num_total_ticketbooks}");
        for ticketbook in ticketbooks_info {
            if ticketbook.has_expired() {
                tracing::debug!("Expired ticketbook: {ticketbook}");
            } else if ticketbook.expired_soon(OffsetDateTime::now_utc(), self.config) {
                tracing::info!("Soon expired ticketbook: {ticketbook}");
            } else {
                tracing::info!("Ticketbook: {ticketbook}");
            }
        }

        Ok(())
    }
}

// So we can use the BC without making it run on its own if we don't need that
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<St: Storage> BandwidthTicketProvider for BandwidthController<St> {
    async fn get_ecash_ticket(
        &self,
        ticket_type: TicketType,
        gateway_id: ed25519::PublicKey,
        tickets_to_spend: u32,
        spend_time: OffsetDateTime,
    ) -> Result<Option<PreparedCredential>, BandwidthControllerError> {
        self.prepare_ecash_ticket(
            ticket_type,
            gateway_id.to_bytes(),
            tickets_to_spend,
            spend_time,
        )
        .await
    }

    async fn get_upgrade_mode_token(&self) -> Result<Option<String>, BandwidthControllerError> {
        self.get_upgrade_mode_token().await
    }

    async fn attempt_revert_spending(
        &self,
        metadata: PreparedCredentialMetadata,
    ) -> Result<bool, BandwidthControllerError> {
        self.attempt_revert_ticket_usage(metadata).await
    }

    async fn close(&self) {
        self.storage.close().await
    }
}
