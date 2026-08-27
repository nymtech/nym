// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::BandwidthControllerError, ticketbooks::AvailableTicketbooks, traits::CredentialFetcher,
    CredentialPublicDataFetcher, EcashTicketRequest, PreparedCredential,
    PreparedCredentialMetadata,
};
use nym_credentials_interface::TicketType;
use std::sync::Arc;

pub use sender::BandwidthControllerRequestSender;
use tokio::sync::oneshot;

mod sender;

#[derive(strum::Display)]
pub enum BandwidthControllerRequest {
    EcashTicket(ReturnSender<Option<PreparedCredential>>, EcashTicketRequest),

    UpgradeModeToken(ReturnSender<Option<String>>),

    AttemptRevertSpending(ReturnSender<bool>, PreparedCredentialMetadata),

    /// Sets (or clears, with `None`) the credential fetcher; a new one triggers an immediate restock.
    SetCredentialFetcher(ReturnSender<()>, Option<Arc<dyn CredentialFetcher>>),
    /// Sets (or clears, with `None`) the global-data fetcher.
    SetPublicDataFetcher(
        ReturnSender<()>,
        Option<Arc<dyn CredentialPublicDataFetcher>>,
    ),
    /// Cancels in-flight fetches, drops the fetcher, clears all stored credentials, and fails every
    /// parked readiness waiter.
    Reset(ReturnSender<()>),
    /// Removes the stored emergency (upgrade-mode) credentials only, leaving ticketbooks intact.
    ClearEmergencyCredentials(ReturnSender<()>),
    /// Returns the currently stored ticketbooks (also logs a stock summary).
    GetAvailableTicketbooks(ReturnSender<AvailableTicketbooks>),

    /// Checks the given ticket types and kicks off a background restock for any running low or
    /// about to expire. Resolves once the check has been scheduled, not once the fetches finish.
    ///
    /// Not to be used lightly: the automatic triggers (ticket handout, timer, fetcher install)
    /// already keep every type stocked. This is a manual safety valve, not a routine call.
    RestockTicketbooks(ReturnSender<()>, Vec<TicketType>),

    /// Resolves once every required ticket type is usable (stocked or covered by upgrade mode),
    /// or fails if a required type is neither stocked nor being fetched.
    WaitForTicketbooks(ReturnSender<()>, Vec<TicketType>),
}

#[derive(Debug)]
pub struct ReturnSender<T> {
    sender: oneshot::Sender<Result<T, BandwidthControllerError>>,
}

impl<T> ReturnSender<T>
where
    T: std::fmt::Debug,
{
    pub fn new() -> (Self, oneshot::Receiver<Result<T, BandwidthControllerError>>) {
        let (sender, receiver) = oneshot::channel();
        (Self { sender }, receiver)
    }

    pub fn send(self, response: Result<T, BandwidthControllerError>)
    where
        T: Send,
    {
        self.sender
            .send(response)
            .inspect_err(|_| {
                tracing::warn!("Failed to send response, receiver is gone already");
            })
            .ok();
    }
}
