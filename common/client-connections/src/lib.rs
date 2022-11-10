// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::channel::mpsc;

pub type ConnectionId = u64;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub enum TransmissionLane {
    General,
    Reply,
    Retransmission,
    Control,
    ConnectionId(ConnectionId),
}

/// Announce connections that are closed, for whoever is interested.
/// One usecase is that the network-requester and socks5-client wants to know about this, so that
/// they can forward this to the `OutQueueControl` (via `ClientRequest` for the network-requester)
pub type ClosedConnectionSender = mpsc::UnboundedSender<ConnectionId>;
pub type ClosedConnectionReceiver = mpsc::UnboundedReceiver<ConnectionId>;
