// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::node::client_handling::websocket::message_receiver::MixMessageSender;
use dashmap::DashMap;
use nym_sphinx::DestinationAddressBytes;
use futures::channel::{mpsc, oneshot};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecentlyActive {
    Yes,
    YesButReset,
    No,
}

#[derive(Clone)]
pub(crate) struct ActiveClientsStore(
    Arc<DashMap<DestinationAddressBytes, (MixMessageSender, RecentlyActive, mpsc::UnboundedSender<oneshot::Sender<bool>>)>>,
);

impl ActiveClientsStore {
    /// Creates new instance of `ActiveClientsStore` to store in-memory handles to all currently connected clients.
    pub(crate) fn new() -> Self {
        ActiveClientsStore(Arc::new(DashMap::new()))
    }

    /// Tries to obtain sending channel to specified client. Note that if stale entry existed, it is
    /// removed and a `None` is returned instead.
    ///
    /// # Arguments
    ///
    /// * `client`: address of the client for which to obtain the handle.
    pub(crate) fn get(&self, client: DestinationAddressBytes) -> Option<MixMessageSender> {
        let entry = self.0.get(&client)?;
        let handle = &entry.value().0;

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

    pub(crate) fn get_is_recently_active(&self, client: DestinationAddressBytes) -> Option<RecentlyActive> {
        let entry = self.0.get(&client)?;
        let is_recently_active = &entry.value().1;

        // if the entry is stale, remove it from the map
        // if handle.is_valid() {
        if !entry.value().0.is_closed() {
            Some(is_recently_active.clone())
        } else {
            // drop the reference to the map to prevent deadlocks
            drop(entry);
            self.0.remove(&client);
            None
        }
    }

    pub(crate) fn get_is_active_sender(&self, client: DestinationAddressBytes) -> Option<mpsc::UnboundedSender<oneshot::Sender<bool>>> {
        let entry = self.0.get(&client)?;
        let is_active_sender = &entry.value().2;

        // if the entry is stale, remove it from the map
        // if handle.is_valid() {
        if !entry.value().0.is_closed() {
            Some(is_active_sender.clone())
        } else {
            // drop the reference to the map to prevent deadlocks
            drop(entry);
            self.0.remove(&client);
            None
        }
    }

    pub(crate) fn register_activity(&self, client: DestinationAddressBytes) {
        if let Some(mut entry) = self.0.get_mut(&client) {
            entry.value_mut().1 = RecentlyActive::Yes;
        }
    }

    pub(crate) fn reset_activity(&self, client: DestinationAddressBytes) {
        if let Some(mut entry) = self.0.get_mut(&client) {
            if entry.value_mut().1 == RecentlyActive::Yes {
                entry.value_mut().1 = RecentlyActive::YesButReset;
            } else if entry.value_mut().1 == RecentlyActive::YesButReset {
                entry.value_mut().1 = RecentlyActive::No;
            }
        }
    }

    /// Indicates particular client has disconnected from the gateway and its handle should get removed.
    ///
    /// # Arguments
    ///
    /// * `client`: address of the client for which to remove the handle.
    pub(crate) fn disconnect(&self, client: DestinationAddressBytes) {
        self.0.remove(&client);
    }

    /// Insert new client handle into the store.
    ///
    /// # Arguments
    ///
    /// * `client`: address of the client for which to insert the handle.
    /// * `handle`: the sender channel for all mix packets to be pushed back onto the websocket
    pub(crate) fn insert(&self, client: DestinationAddressBytes, handle: MixMessageSender, is_active_sender: mpsc::UnboundedSender<oneshot::Sender<bool>>) {
        self.0.insert(client, (handle, RecentlyActive::Yes, is_active_sender));
    }

    /// Get number of active clients in store
    pub(crate) fn size(&self) -> usize {
        self.0.len()
    }

}
