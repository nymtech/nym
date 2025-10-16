// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::StreamExt;

use crate::client::base_client::{EventReceiver, EventSender, MixnetClientEvent};

/// Launches and manages task events, propagating upwards what is not strictly internal.
pub(crate) struct EventControl {
    parent_event_tx: Option<EventSender>,
    children_event_rx: EventReceiver,
}

impl EventControl {
    pub(crate) fn new(
        parent_event_tx: Option<EventSender>,
        children_event_rx: EventReceiver,
    ) -> Self {
        EventControl {
            parent_event_tx,
            children_event_rx,
        }
    }

    fn is_internal(event: MixnetClientEvent) -> bool {
        match event {
            MixnetClientEvent::Traffic(_) => false,
        }
    }

    pub(crate) async fn run(mut self) {
        while let Some(event) = self.children_event_rx.next().await {
            if let Some(parent_event_tx) = &self.parent_event_tx {
                if !Self::is_internal(event) {
                    parent_event_tx.send(event);
                }
            }
        }
    }
}
