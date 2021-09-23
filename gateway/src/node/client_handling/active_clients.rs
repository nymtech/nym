// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::client_handling::websocket::message_receiver::MixMessageSender;
use dashmap::DashMap;
use nymsphinx::DestinationAddressBytes;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct ActiveClientsStore(Arc<DashMap<DestinationAddressBytes, MixMessageSender>>);

impl ActiveClientsStore {
    pub(crate) fn new() -> Self {
        ActiveClientsStore(Arc::new(DashMap::new()))
    }

    pub(crate) fn get(&self, client: DestinationAddressBytes) -> Option<MixMessageSender> {
        let entry = self.0.get(&client)?;
        let handle = entry.value();

        // if the entry is stale, remove it from the map
        // if handle.is_valid() {
        if !handle.is_closed() {
            Some(handle.clone())
        } else {
            // drop the reference to the map to prevent deadlocks
            drop(entry);
            self.0.remove(&client);
            None
        }
    }

    pub(crate) fn disconnect(&self, client: DestinationAddressBytes) {
        self.0.remove(&client);
    }

    pub(crate) fn insert(&self, client: DestinationAddressBytes, handle: MixMessageSender) {
        self.0.insert(client, handle);
    }
}
