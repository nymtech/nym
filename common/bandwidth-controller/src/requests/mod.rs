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
    EcashTicket(
        ReturnSender<Option<PreparedCredential>, BandwidthControllerError>,
        EcashTicketRequest,
    ),

    UpgradeModeToken(ReturnSender<Option<String>, BandwidthControllerError>),

    AttemptRevertSpending(
        ReturnSender<bool, BandwidthControllerError>,
        PreparedCredentialMetadata,
    ),
}

#[derive(Debug)]
pub struct ReturnSender<T, E> {
    sender: oneshot::Sender<Result<T, E>>,
}

impl<T, E> ReturnSender<T, E>
where
    T: std::fmt::Debug,
    E: std::fmt::Debug,
{
    pub fn new() -> (Self, oneshot::Receiver<Result<T, E>>) {
        let (sender, receiver) = oneshot::channel();
        (Self { sender }, receiver)
    }

    pub fn send(self, response: Result<T, E>)
    where
        T: Send,
        E: Send,
    {
        self.sender
            .send(response)
            .inspect_err(|err| {
                tracing::error!("Failed to send response: {:#?}", err);
            })
            .ok();
    }
}
