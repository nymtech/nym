// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::config::BandwidthControllerConfig;
use crate::error::BandwidthControllerError;
use crate::readiness::{FetchFailure, ReadinessRequest, ReadinessSnapshot};
use crate::requests::{BandwidthControllerRequest, BandwidthControllerRequestSender, ReturnSender};
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
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

#[cfg(not(target_arch = "wasm32"))]
use tokio::time::{interval, MissedTickBehavior};

#[cfg(target_arch = "wasm32")]
use wasmtimer::tokio::{interval, MissedTickBehavior};

use crate::in_flight::{
    global_data::{GlobalData, GlobalDataRequest},
    FetchJob, FetchKey, FetchResult, FetchedData, InFlightFetches,
};

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

    // background fetches currently in flight (both ticketbooks and global signing data); skips
    // duplicate requests per key, drives completions, and cancels on reset/shutdown.
    in_flight: InFlightFetches,

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
            in_flight: InFlightFetches::new(),
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
    pub async fn run(mut self, shutdown_token: ShutdownToken) {
        tracing::info!("BandwidthController started successfully");

        let mut topup_interval = interval(self.config.topup_interval);
        topup_interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                biased;
                _ = shutdown_token.cancelled() => {
                    log::debug!("bandwidth controller received cancellation request; shutting down");
                    break;
                }
                _ = topup_interval.tick() => {
                    let _ = self.print_info().await;
                    self.check_and_restock(self.config.managed_ticket_types.clone()).await;
                }
                (key, res) = self.in_flight.next_result(), if !self.in_flight.is_empty() => {
                    self.on_fetch_complete(key, res).await;
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

        // wait for in-flight fetches to stop before cleaning up the fetcher they still hold
        self.in_flight.cancel_and_join().await;
        if let Some(fetcher) = self.credential_fetcher {
            fetcher.cleanup().await;
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
                // a ticket was just requested for this type - top it up if it's now running low, but
                // only if it's a managed type (don't start depositing for a type this controller
                // isn't configured to proactively restock, e.g. a leftover ticketbook of another kind)
                if self.config.managed_ticket_types.contains(&ticket_type) {
                    self.check_and_restock(vec![ticket_type]).await;
                }
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
                self.handle_set_public_data_fetcher(fetcher).await;
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
            BandwidthControllerRequest::RestockTicketbooks(return_sender, ticket_types) => {
                self.check_and_restock(ticket_types).await;
                return_sender.send(Ok(()))
            }
            BandwidthControllerRequest::WaitForTicketbooks(return_sender, ticket_types) => {
                self.handle_wait_for_ticketbooks(return_sender, ticket_types)
                    .await
            }
        }
    }

    async fn handle_set_credential_fetcher(&mut self, fetcher: Option<Arc<dyn CredentialFetcher>>) {
        self.in_flight.cancel_and_join().await;
        if let Some(old_fetcher) = self.credential_fetcher.take() {
            old_fetcher.cleanup().await;
        }
        // a credential fetcher is also a public-data fetcher (explicit upcast through the Option)
        let public_data_fetcher = fetcher
            .clone()
            .map(|f| f as Arc<dyn CredentialPublicDataFetcher>);
        self.public_data_fetcher = public_data_fetcher;
        // No need to run a global data fetch here, check_and_restock does it

        self.credential_fetcher = fetcher;
        self.check_and_restock(self.config.managed_ticket_types.clone())
            .await;
    }

    async fn handle_set_public_data_fetcher(
        &mut self,
        fetcher: Option<Arc<dyn CredentialPublicDataFetcher>>,
    ) {
        self.public_data_fetcher = fetcher;
        // the public-data fetcher is what provisions global data, so installing one lets us fetch
        // anything the stored ticketbooks are still missing
        self.prefetch_global_data().await;
    }

    // Removes fetcher, stops in flight requests, clear credentials, answer all readiness request with unavailable
    async fn handle_reset(&mut self) -> Result<(), BandwidthControllerError> {
        // Cancel fetching and wait for the tasks to stop before touching the fetcher or storage:
        // this leaves no stale in-flight entries (so a following restock can spawn), and drains any
        // fetch that completed just before cancellation so it can't resurrect a ticketbook into
        // storage we're about to clear.
        self.in_flight.cancel_and_join().await;

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

    async fn handle_wait_for_ticketbooks(
        &mut self,
        return_sender: ReturnSender<()>,
        ticket_types: Vec<TicketType>,
    ) {
        // resolve the concrete global data those types need to be spendable (epoch + date of the
        // ticketbook that would be taken); empty for a type that isn't stocked yet - it's refreshed
        // once a ticketbook lands (see `resolve_pending_waiters`)
        let global_data = self
            .required_global_data(&ticket_types)
            .await
            .unwrap_or_default();
        let request = ReadinessRequest {
            return_sender,
            ticket_types,
            global_data,
        };

        let snapshot = match self.build_readiness_snapshot(None).await {
            Ok(snapshot) => snapshot,
            Err(err) => {
                // transient storage failure - leave the waiter parked for the next state change
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
    /// This is spending a ticket
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
    // Automatic restocking & background fetches
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

        // Ensure that data inline (fetch + persist if missing). `fetch_ticketbook` only runs on a
        // non-running controller, so blocking is fine
        self.ensure_global_data().await;

        Ok(())
    }

    /// Restocks the given ticket types that are running low or about to expire.
    /// Also checks the global_data
    async fn check_and_restock(&mut self, ticketbook_types: Vec<TicketType>) {
        let available = match self.get_available_ticketbooks().await {
            Ok(available) => available,
            Err(err) => {
                tracing::warn!("could not assess ticket stock for restocking: {err}");
                return;
            }
        };

        for typ in ticketbook_types {
            if self.config.managed_ticket_types.contains(&typ) {
                tracing::debug!("Checking credential stock for {typ} ticket");
                if available.needs_restock(typ, &self.config) {
                    tracing::debug!("{typ} tickets need a restock");
                    self.ensure_stocked(typ);
                }
            } else {
                tracing::debug!("Credential {typ} is not managed, check and restock skipped");
            }
        }
        self.prefetch_global_data().await;
    }

    /// Spawns a background fetch for `ticket_type` unless one is already in flight for it.
    /// Non-blocking: the result is drained later in the `run` loop via `on_fetch_complete`.
    fn ensure_stocked(&mut self, ticket_type: TicketType) {
        let Some(fetcher) = &self.credential_fetcher else {
            tracing::debug!("No credential fetcher set. No restock possible");
            return;
        };
        if self.in_flight.is_in_flight(ticket_type) {
            // already fetching this type; don't ask again while we're still waiting
            tracing::debug!("{ticket_type} ticket restock already in flight");
            return;
        }
        tracing::debug!("requesting more {ticket_type} ticketbooks");
        self.in_flight.spawn(FetchJob::Ticketbook {
            ticket_type,
            fetcher: Arc::clone(fetcher),
        });
    }

    /// Routes a completed background fetch to the right handler based on what was requested.
    async fn on_fetch_complete(&mut self, key: FetchKey, received: FetchResult) {
        match key {
            FetchKey::Ticketbook(ticket_type) => {
                self.on_ticketbook_fetch_complete(ticket_type, received)
                    .await
            }
            FetchKey::GlobalData(request) => {
                self.on_global_data_fetch_complete(request, received).await
            }
        }
    }

    /// Persists a completed ticketbook fetch and re-evaluates readiness waiters. The in-flight slot
    /// was already freed by [`InFlightFetches::next_result`].
    async fn on_ticketbook_fetch_complete(
        &mut self,
        ticket_type: TicketType,
        received: FetchResult,
    ) {
        // a failed fetch is surfaced to any readiness waiter that required the failed type
        let failure = match received {
            Ok(Some(Ok(FetchedData::Ticketbooks(credentials)))) => {
                self.store_fetched(credentials).await;
                tracing::info!("fetched and stored a {ticket_type} ticketbook");
                // provision the global data those ticketbooks need in the background, off the loop
                self.prefetch_global_data().await;
                None
            }
            Ok(Some(Err(err))) => {
                tracing::warn!("failed to fetch {ticket_type} ticketbooks: {err}");
                Some(FetchFailure {
                    ticket_type,
                    error: err,
                })
            }
            Ok(None) => {
                // fetch was cancelled (reset / shutdown); next_result already freed the slot
                tracing::debug!("fetch for {ticket_type} ticketbooks was cancelled");
                None
            }
            // the task panicked (or, impossibly, returned a non-ticketbook payload); the slot was
            // already freed, so a later restock can retry the type
            Ok(Some(Ok(_))) | Err(_) => {
                tracing::error!("a credential fetch task for {ticket_type} terminated abnormally");
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

    /// Persists a fetched ticketbook
    async fn store_ticketbook(&self, ticketbook: IssuedTicketBook) {
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

    /// Gathers the current stock, stored global data, in-flight fetches and any fetch failure, and
    /// hands them to [`ReadinessSnapshot::build`] to assemble - the controller just collects, the
    /// snapshot does the bookkeeping.
    async fn build_readiness_snapshot(
        &self,
        failure: Option<FetchFailure>,
    ) -> Result<ReadinessSnapshot, BandwidthControllerError> {
        let upgrade_mode = self.get_upgrade_mode_token().await?.is_some();

        let available = self.get_available_ticketbooks().await?;
        let stocked_types = AvailableTicketbooks::ticketbook_types()
            .into_iter()
            .filter(|typ| available.contains_minimal_tickets(*typ, &self.config))
            .collect();

        let available_global_data = self
            .storage
            .get_available_global_data()
            .await
            .map_err(BandwidthControllerError::credential_storage_error)?;

        Ok(ReadinessSnapshot::build(
            upgrade_mode,
            stocked_types,
            self.in_flight.in_flight_ticketbooks(),
            failure,
            available_global_data,
            self.in_flight.in_flight_global_data(),
        ))
    }

    /// The `(epoch, date)` global data needed to spend the given types: for each stocked type, that
    /// of the ticketbook that would be taken next. A type with no usable ticketbook contributes
    /// nothing (its requirement is the ticketbook itself, tracked separately).
    ///
    /// Read-only - it peeks at the next-usable ticketbook's metadata without withdrawing (unlike
    /// [`Self::get_next_usable_ticketbook`], which is the actual spend path).
    async fn required_global_data(
        &self,
        ticket_types: &[TicketType],
    ) -> Result<Vec<(EpochId, Date)>, BandwidthControllerError> {
        let available = self.get_available_ticketbooks().await?;
        Ok(ticket_types
            .iter()
            .filter_map(|&typ| {
                available
                    .next_usable(typ)
                    .map(|ticketbook| (ticketbook.epoch_id, ticketbook.expiration))
            })
            .collect())
    }

    /// Re-evaluates parked `wait_for_ticketbooks` callers after stock/in-flight state changed,
    /// answering and dropping the ones that resolved.
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

        let mut still_waiting = Vec::new();
        for mut request in std::mem::take(&mut self.pending_readiness) {
            // a newly-stored ticketbook can introduce global-data requirements a still-unstocked
            // type didn't have when it was parked, so refresh them before evaluating
            request.global_data = self
                .required_global_data(&request.ticket_types)
                .await
                .unwrap_or_default();
            if let Some(request) = request.try_resolve(&snapshot) {
                still_waiting.push(request);
            }
        }

        self.pending_readiness = still_waiting;
    }

    // ---------------------------------------------------------------------
    // Global signing data (fetched & cached on demand)
    // ---------------------------------------------------------------------

    /// Returns the master verification key for the epoch, fetching and persisting it via the
    /// public-data fetcher if it isn't already in local storage.
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
        self.fetch_and_store_global_data(GlobalDataRequest::MasterVerificationKey(epoch_id))
            .await?;
        self.storage
            .get_master_verification_key(epoch_id)
            .await
            .map_err(BandwidthControllerError::credential_storage_error)?
            .ok_or(BandwidthControllerError::MissingVerificationKey { epoch_id })
    }

    /// Returns the coin index signatures for the epoch, fetching and persisting them via the
    /// public-data fetcher if they aren't already in local storage.
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
        self.fetch_and_store_global_data(GlobalDataRequest::CoinIndexSignatures(epoch_id))
            .await?;
        self.storage
            .get_coin_index_signatures(epoch_id)
            .await
            .map_err(BandwidthControllerError::credential_storage_error)?
            .ok_or(BandwidthControllerError::MissingCoinIndexSignatures { epoch_id })
    }

    /// Returns the expiration date signatures for the epoch and expiration date, fetching and
    /// persisting them via the public-data fetcher if they aren't already in local storage.
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
        self.fetch_and_store_global_data(GlobalDataRequest::ExpirationDateSignatures {
            epoch_id,
            expiration_date,
        })
        .await?;
        self.storage
            .get_expiration_date_signatures(expiration_date, epoch_id)
            .await
            .map_err(BandwidthControllerError::credential_storage_error)?
            .ok_or(BandwidthControllerError::MissingExpirationDateSignatures { epoch_id })
    }

    /// The global signing data referenced by a stored ticketbook whose fetch (master key, coin
    /// index or expiration signatures) isn't in local storage yet.
    async fn missing_global_data(
        &self,
    ) -> Result<Vec<GlobalDataRequest>, BandwidthControllerError> {
        let available_global_data = self
            .storage
            .get_available_global_data()
            .await
            .map_err(BandwidthControllerError::credential_storage_error)?;

        let ticketbooks = self
            .storage
            .get_ticketbooks_info()
            .await
            .map_err(BandwidthControllerError::credential_storage_error)?;

        let required = AvailableTicketbooks::try_from(ticketbooks)?
            .filter(|ticketbook| !ticketbook.has_expired())
            .flat_map(|ticketbook| {
                GlobalDataRequest::for_ticketbook(ticketbook.epoch_id, ticketbook.expiration)
            })
            .collect::<HashSet<_>>();

        let missing = required
            .into_iter()
            .filter(|request| {
                let present = match *request {
                    GlobalDataRequest::MasterVerificationKey(epoch_id) => available_global_data
                        .master_verification_key_epochs
                        .contains(&epoch_id),
                    GlobalDataRequest::CoinIndexSignatures(epoch_id) => available_global_data
                        .coin_index_signature_epochs
                        .contains(&epoch_id),
                    GlobalDataRequest::ExpirationDateSignatures {
                        epoch_id,
                        expiration_date,
                    } => available_global_data
                        .expiration_date_signatures
                        .contains(&(epoch_id, expiration_date)),
                };
                !present
            })
            .collect();

        Ok(missing)
    }

    /// Kicks off background fetches for any missing global signing data. Only local storage reads
    /// happen inline; the (slow) network fetches run off the event loop and are persisted by
    /// `on_global_data_fetch_complete`, so the loop stays responsive instead of blocking.
    async fn prefetch_global_data(&mut self) {
        let missing = match self.missing_global_data().await {
            Ok(missing) => missing,
            Err(err) => {
                tracing::warn!("could not assess global data for prefetch: {err}");
                return;
            }
        };
        for request in missing {
            self.spawn_global_data_fetch(request);
        }
    }

    /// Fetches and persists any missing global signing data inline. Blocking - only used on a
    /// non-running controller, where there's no event loop to drain a background fetch.
    async fn ensure_global_data(&self) {
        let missing = match self.missing_global_data().await {
            Ok(missing) => missing,
            Err(err) => {
                tracing::warn!("could not assess global data: {err}");
                return;
            }
        };
        for request in missing {
            if let Err(err) = self.fetch_and_store_global_data(request).await {
                tracing::warn!("failed to ensure global data ({request:?}): {err}");
            }
        }
    }

    /// Spawns a background fetch for one piece of global data, unless it's already in flight or no
    /// public-data fetcher is set.
    fn spawn_global_data_fetch(&mut self, request: GlobalDataRequest) {
        let Some(fetcher) = &self.public_data_fetcher else {
            tracing::debug!("no public data fetcher set; cannot prefetch {request:?}");
            return;
        };
        if self.in_flight.is_in_flight(request) {
            return;
        }
        tracing::debug!("prefetching global data: {request:?}");
        self.in_flight.spawn(FetchJob::GlobalData {
            request,
            fetcher: Arc::clone(fetcher),
        });
    }

    /// Fetches one piece of global data and persists it, inline. Does nothing (`Ok`) when no
    /// public-data fetcher is configured - callers that need the value surface their own "missing"
    /// error on the subsequent read.
    async fn fetch_and_store_global_data(
        &self,
        request: GlobalDataRequest,
    ) -> Result<(), BandwidthControllerError> {
        let Some(fetcher) = &self.public_data_fetcher else {
            return Ok(());
        };
        let data = request
            .fetch(&**fetcher)
            .await
            .map_err(BandwidthControllerError::fetcher_error)?;
        self.persist_global_data(data).await
    }

    /// Persists fetched global signing data.
    async fn persist_global_data(&self, data: GlobalData) -> Result<(), BandwidthControllerError> {
        match data {
            GlobalData::MasterVerificationKey(key) => {
                self.storage.insert_master_verification_key(&key).await
            }
            GlobalData::CoinIndexSignatures(signatures) => {
                self.storage.insert_coin_index_signatures(&signatures).await
            }
            GlobalData::ExpirationDateSignatures(signatures) => {
                self.storage
                    .insert_expiration_date_signatures(&signatures)
                    .await
            }
        }
        .map_err(BandwidthControllerError::credential_storage_error)
    }

    /// Persists a completed background global-data fetch. The in-flight slot was already freed by
    /// [`InFlightFetches::next_result`].
    async fn on_global_data_fetch_complete(
        &mut self,
        request: GlobalDataRequest,
        received: FetchResult,
    ) {
        match received {
            Ok(Some(Ok(FetchedData::GlobalData(data)))) => {
                if let Err(err) = self.persist_global_data(data).await {
                    tracing::warn!("failed to persist prefetched global data ({request:?}): {err}");
                }
            }
            Ok(Some(Err(err))) => {
                tracing::warn!("failed to prefetch global data ({request:?}): {err}")
            }
            Ok(None) => tracing::debug!("global data prefetch cancelled: {request:?}"),
            // the task panicked (or, impossibly, returned a non-global-data payload)
            Ok(Some(Ok(_))) | Err(_) => {
                tracing::error!("a global data prefetch task ({request:?}) terminated abnormally")
            }
        }

        // global data settling (landed or failed) can flip a stocked type's readiness, so waiters
        // blocked on it need re-evaluating
        self.resolve_pending_waiters(None).await;
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
            } else if ticketbook.expired_soon(OffsetDateTime::now_utc(), &self.config) {
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
