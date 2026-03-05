// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::lp::control::egress::connection::NestedNodeConnectionSender;

pub(crate) type NestedClientConnectionSender = ();
pub(crate) type NestedClientConnectionReceiver = ();

pub(crate) struct NestedClientConnection {
    // handle for sending into `NestedNodeConnectionHandler`
    sender: NestedNodeConnectionSender,

    // handle for receiving from `NestedNodeConnectionHandler`
    receiver: NestedClientConnectionReceiver,
}
