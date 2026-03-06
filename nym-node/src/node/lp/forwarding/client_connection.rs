// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::lp::control::egress::connection::NestedNodeConnectionSender;
use futures::channel::mpsc::UnboundedReceiver;

pub(crate) type NestedClientConnectionSender = ();
pub(crate) type NestedClientConnectionReceiver = UnboundedReceiver<Vec<u8>>;

pub(crate) struct NestedClientConnection {
    // handle for sending into `NestedNodeConnectionHandler`
    sender: NestedNodeConnectionSender,

    // handle for receiving from `NestedNodeConnectionHandler`
    receiver: NestedClientConnectionReceiver,
}
