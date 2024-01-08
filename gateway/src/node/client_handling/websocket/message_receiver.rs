// Copyright 2020 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use futures::channel::{mpsc, oneshot};

pub(crate) type MixMessageSender = mpsc::UnboundedSender<Vec<Vec<u8>>>;
pub(crate) type MixMessageReceiver = mpsc::UnboundedReceiver<Vec<Vec<u8>>>;

// Channels used for one handler to requester another handler to check that the client is still
// active. The result is then passed back to the requesting handler in the oneshot channel.
pub(crate) type IsActiveRequestSender = mpsc::UnboundedSender<IsActiveResultSender>;
pub(crate) type IsActiveRequestReceiver = mpsc::UnboundedReceiver<IsActiveResultSender>;

#[derive(Debug)]
pub(crate) enum IsActive {
    Active,
    NotActive,
    BusyPinging,
}
pub(crate) type IsActiveResultSender = oneshot::Sender<IsActive>;
