// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use nym_credentials_interface::TicketType;

use crate::{
    error::BandwidthControllerError, requests::ReturnSender, traits::CredentialFetcherError,
};

/// Per-type readiness used to build a snapshot; variants are ordered by severity (see `severity`).
#[derive(Clone, Debug)]
pub(crate) enum ReadinessStatus {
    Ready,
    InFlight,
    Unavailable,
    /// unavailable because the last fetch failed; carries the reason for the waiter
    FetchFailed(String),
}

impl ReadinessStatus {
    // higher = worse; `evaluate_readiness` reports the most severe status across required types
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
#[derive(Debug)]
pub(crate) struct ReadinessSnapshot {
    pub(crate) upgrade_mode: bool,
    pub(crate) tickets_readiness: HashMap<TicketType, ReadinessStatus>,
}

impl ReadinessSnapshot {
    // Return the most severe ReadinessStatus across `required` (upgrade mode short-circuits to Ready)
    fn evaluate_readiness(&self, required: &[TicketType]) -> ReadinessStatus {
        if self.upgrade_mode {
            return ReadinessStatus::Ready;
        }
        required
            .iter()
            .map(|typ| {
                self.tickets_readiness
                    .get(typ)
                    .cloned()
                    .unwrap_or(ReadinessStatus::Unavailable)
            })
            .max_by_key(|status| status.severity())
            .unwrap_or(ReadinessStatus::Ready)
    }
}

/// A parked `wait_for_ticketbooks` caller: its reply channel plus the types it needs usable.
#[derive(Debug)]
pub(crate) struct ReadinessRequest {
    pub(crate) return_sender: ReturnSender<()>,
    pub(crate) ticket_types: Vec<TicketType>,
}

impl ReadinessRequest {
    /// Resolves against the snapshot: `Ok` once ready, an error once a required type is unavailable
    /// (carrying the fetch failure reason when the snapshot recorded one), `Some(self)` while still
    /// waiting on an in-flight fetch.
    pub(crate) fn try_resolve(self, readiness_snapshot: &ReadinessSnapshot) -> Option<Self> {
        match readiness_snapshot.evaluate_readiness(&self.ticket_types) {
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
                    .send(Err(BandwidthControllerError::TicketbookFetchFailed { reason }));
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
