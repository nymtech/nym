// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::BandwidthControllerError, EcashTicketRequest, PreparedCredential,
    PreparedCredentialMetadata,
};

pub use sender::BandwidthControllerRequestSender;
use tokio::sync::oneshot;

mod sender;

#[derive(strum::Display)]
pub enum BandwidthControllerRequest {
    EcashTicket(ReturnSender<Option<PreparedCredential>>, EcashTicketRequest),

    UpgradeModeToken(ReturnSender<Option<String>>),

    AttemptRevertSpending(ReturnSender<bool>, PreparedCredentialMetadata),
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
            .inspect_err(|err| {
                tracing::error!("Failed to send response: {:#?}", err);
            })
            .ok();
    }
}
