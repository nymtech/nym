// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use nym_credential_storage::models::AvailableGlobalData;
use nym_credentials_interface::TicketType;
use nym_ecash_time::Date;
use nym_validator_client::nym_api::EpochId;

use crate::in_flight::global_data::GlobalDataRequest;
use crate::{
    error::BandwidthControllerError, requests::ReturnSender, traits::CredentialFetcherError,
};

/// Per-item readiness; variants are ordered by severity (see `severity`).
#[derive(Clone, Debug)]
pub(crate) enum ReadinessStatus {
    Ready,
    InFlight,
    Unavailable,
    /// unavailable because the last fetch failed; carries the reason for the waiter
    FetchFailed(String),
}

impl ReadinessStatus {
    // higher = worse; `evaluate_readiness` reports the most severe status across a request's needs
    fn severity(&self) -> u8 {
        match self {
            ReadinessStatus::Ready => 0,
            ReadinessStatus::InFlight => 1,
            ReadinessStatus::Unavailable => 2,
            ReadinessStatus::FetchFailed(_) => 3,
        }
    }
}

/// A ticketbook fetch that just failed; folded into a snapshot so a waiting caller learns why.
#[derive(Debug)]
pub(crate) struct FetchFailure {
    pub(crate) ticket_type: TicketType,
    pub(crate) error: CredentialFetcherError,
}

/// The state readiness is judged against - built once and reused across a batch of waiters.
///
/// Each map holds only what's known to be `Ready` or `InFlight`; anything absent is `Unavailable`
/// at resolution time. Global data is tracked per piece, keyed as it's stored: master key and
/// coin-index signatures by epoch, expiration-date signatures by `(epoch, date)`.
#[derive(Debug)]
pub(crate) struct ReadinessSnapshot {
    upgrade_mode: bool,
    tickets_readiness: HashMap<TicketType, ReadinessStatus>,
    master_key_readiness: HashMap<EpochId, ReadinessStatus>,
    coin_index_readiness: HashMap<EpochId, ReadinessStatus>,
    expiration_readiness: HashMap<(EpochId, Date), ReadinessStatus>,
}

impl ReadinessSnapshot {
    /// Assembles the snapshot from what's currently stocked/stored (`Ready`), what's being fetched
    /// (`InFlight`), and any recorded ticket fetch failure. Same recipe for every map: present is
    /// `Ready`, in flight is `InFlight` (without overriding a `Ready`), everything else is left out
    /// and resolves to `Unavailable`.
    pub(crate) fn build(
        upgrade_mode: bool,
        stocked_types: Vec<TicketType>,
        tickets_in_flight: Vec<TicketType>,
        ticket_failure: Option<FetchFailure>,
        available_global_data: AvailableGlobalData,
        global_data_in_flight: Vec<GlobalDataRequest>,
    ) -> Self {
        let mut tickets_readiness = HashMap::new();
        for ticket_type in stocked_types {
            tickets_readiness.insert(ticket_type, ReadinessStatus::Ready);
        }
        for ticket_type in tickets_in_flight {
            tickets_readiness
                .entry(ticket_type)
                .or_insert(ReadinessStatus::InFlight);
        }
        if let Some(failure) = ticket_failure {
            tickets_readiness
                .entry(failure.ticket_type)
                .or_insert(ReadinessStatus::FetchFailed(failure.error.to_string()));
        }

        let mut master_key_readiness =
            epochs_ready(available_global_data.master_verification_key_epochs);
        let mut coin_index_readiness =
            epochs_ready(available_global_data.coin_index_signature_epochs);
        let mut expiration_readiness: HashMap<(EpochId, Date), ReadinessStatus> =
            available_global_data
                .expiration_date_signatures
                .into_iter()
                .map(|(epoch_id, date)| ((EpochId::from(epoch_id), date), ReadinessStatus::Ready))
                .collect();

        for request in global_data_in_flight {
            match request {
                GlobalDataRequest::MasterVerificationKey(epoch_id) => {
                    master_key_readiness
                        .entry(epoch_id)
                        .or_insert(ReadinessStatus::InFlight);
                }
                GlobalDataRequest::CoinIndexSignatures(epoch_id) => {
                    coin_index_readiness
                        .entry(epoch_id)
                        .or_insert(ReadinessStatus::InFlight);
                }
                GlobalDataRequest::ExpirationDateSignatures {
                    epoch_id,
                    expiration_date,
                } => {
                    expiration_readiness
                        .entry((epoch_id, expiration_date))
                        .or_insert(ReadinessStatus::InFlight);
                }
            }
        }

        ReadinessSnapshot {
            upgrade_mode,
            tickets_readiness,
            master_key_readiness,
            coin_index_readiness,
            expiration_readiness,
        }
    }

    // Return the most severe ReadinessStatus across the request's needs (upgrade mode -> Ready).
    fn evaluate_readiness(&self, request: &ReadinessRequest) -> ReadinessStatus {
        if self.upgrade_mode {
            return ReadinessStatus::Ready;
        }

        let tickets = request.ticket_types.iter().map(|ticket_type| {
            self.tickets_readiness
                .get(ticket_type)
                .cloned()
                .unwrap_or(ReadinessStatus::Unavailable)
        });
        // spending a `(epoch, date)` ticketbook needs all three of its global-data pieces present
        let global_data = request
            .global_data
            .iter()
            .flat_map(|&(epoch_id, expiration_date)| {
                [
                    self.master_key_readiness.get(&epoch_id),
                    self.coin_index_readiness.get(&epoch_id),
                    self.expiration_readiness.get(&(epoch_id, expiration_date)),
                ]
                .map(|status| status.cloned().unwrap_or(ReadinessStatus::Unavailable))
            });

        tickets
            .chain(global_data)
            .max_by_key(|status| status.severity())
            .unwrap_or(ReadinessStatus::Ready)
    }
}

/// Builds a per-epoch `Ready` map from the epochs whose data is present in storage.
fn epochs_ready(epochs: Vec<u64>) -> HashMap<EpochId, ReadinessStatus> {
    epochs
        .into_iter()
        .map(|epoch_id| (EpochId::from(epoch_id), ReadinessStatus::Ready))
        .collect()
}

/// A parked `wait_for_ticketbooks` caller: its reply channel plus what it needs usable - the ticket
/// types and the `(epoch, date)` global data for the ticketbook each would spend next.
#[derive(Debug)]
pub(crate) struct ReadinessRequest {
    pub(crate) return_sender: ReturnSender<()>,
    pub(crate) ticket_types: Vec<TicketType>,
    pub(crate) global_data: Vec<(EpochId, Date)>,
}

impl ReadinessRequest {
    /// Resolves against the snapshot: `Ok` once ready, an error once a requirement is unavailable
    /// (carrying the fetch failure reason when the snapshot recorded one), `Some(self)` while still
    /// waiting on an in-flight fetch.
    pub(crate) fn try_resolve(self, snapshot: &ReadinessSnapshot) -> Option<Self> {
        match snapshot.evaluate_readiness(&self) {
            ReadinessStatus::Ready => {
                self.return_sender.send(Ok(()));
                None
            }
            ReadinessStatus::InFlight => Some(self),
            ReadinessStatus::Unavailable => {
                self.return_sender
                    .send(Err(BandwidthControllerError::TicketbooksUnavailable));
                None
            }
            ReadinessStatus::FetchFailed(reason) => {
                self.return_sender
                    .send(Err(BandwidthControllerError::TicketbookFetchFailed {
                        reason,
                    }));
                None
            }
        }
    }

    /// Fails the waiter with `TicketbooksUnavailable`; used when the controller is reset.
    pub(crate) fn cancel(self) {
        self.return_sender
            .send(Err(BandwidthControllerError::TicketbooksUnavailable));
    }
}
