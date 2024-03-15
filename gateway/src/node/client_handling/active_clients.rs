// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::websocket::message_receiver::{IsActiveRequestSender, MixMessageSender};
use crate::node::client_handling::embedded_clients::LocalEmbeddedClientHandle;
use dashmap::DashMap;
use log::warn;
use nym_sphinx::DestinationAddressBytes;
use std::sync::Arc;

enum ActiveClient {
    /// Handle to a remote client connected via a network socket.
    Remote(ClientIncomingChannels),

    /// Handle to a locally (inside the same process) running network requester client.
    Embedded(LocalEmbeddedClientHandle),
}

impl ActiveClient {
    fn get_sender_ref(&self) -> &MixMessageSender {
        match self {
            ActiveClient::Remote(remote) => &remote.mix_message_sender,
            ActiveClient::Embedded(embedded) => &embedded.mix_message_sender,
        }
    }

    fn get_sender(&self) -> MixMessageSender {
        match self {
            ActiveClient::Remote(remote) => remote.mix_message_sender.clone(),
            ActiveClient::Embedded(embedded) => embedded.mix_message_sender.clone(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct ActiveClientsStore {
    inner: Arc<DashMap<DestinationAddressBytes, ActiveClient>>,
}

#[derive(Clone)]
pub(crate) struct ClientIncomingChannels {
    // Mix messages coming from the mixnet to the handler of a client.
    pub mix_message_sender: MixMessageSender,

    // Requests sent from the handler of one client to the handler of other clients.
    pub is_active_request_sender: IsActiveRequestSender,
}

impl ActiveClientsStore {
    /// Creates new instance of `ActiveClientsStore` to store in-memory handles to all currently connected clients.
    pub(crate) fn new() -> Self {
        ActiveClientsStore {
            inner: Arc::new(DashMap::new()),
        }
    }

    /// Tries to obtain sending channel to specified client. Note that if stale entry existed, it is
    /// removed and a `None` is returned instead.
    ///
    /// # Arguments
    ///
    /// * `client`: address of the client for which to obtain the handle.
    pub(crate) fn get_sender(&self, client: DestinationAddressBytes) -> Option<MixMessageSender> {
        let entry = self.inner.get(&client)?;
        let handle = entry.value().get_sender();

        // if the entry is stale, remove it from the map
        // if handle.is_valid() {
        if !handle.is_closed() {
            Some(handle)
        } else {
            // drop the reference to the map to prevent deadlocks
            drop(entry);
            self.inner.remove(&client);
            None
        }
    }

    /// Attempts to get full handle to a remotely connected client
    pub(crate) fn get_remote_client(
        &self,
        address: DestinationAddressBytes,
    ) -> Option<ClientIncomingChannels> {
        let entry = self.inner.get(&address)?;
        let handle = entry.value();

        let ActiveClient::Remote(channels) = handle else {
            warn!("attempted to get a remote handle to a embedded network requester");
            return None;
        };

        // if the entry is stale, remove it from the map
        if !channels.mix_message_sender.is_closed() {
            Some(channels.clone())
        } else {
            // drop the reference to the map to prevent deadlocks
            drop(entry);
            self.inner.remove(&address);
            None
        }
    }

    /// Checks whether there's already an active connection to this client.
    /// It will also remove the entry from the map if its stale.
    pub(crate) fn is_active(&self, client: DestinationAddressBytes) -> bool {
        let Some(entry) = self.inner.get(&client) else {
            return false;
        };
        let handle = entry.value().get_sender_ref();

        // if the entry is stale, remove it from the map
        if !handle.is_closed() {
            true
        } else {
            // drop the reference to the map to prevent deadlocks
            drop(entry);
            self.inner.remove(&client);
            false
        }
    }

    /// Indicates particular client has disconnected from the gateway and its handle should get removed.
    ///
    /// # Arguments
    ///
    /// * `client`: address of the client for which to remove the handle.
    pub(crate) fn disconnect(&self, client: DestinationAddressBytes) {
        self.inner.remove(&client);
    }

    /// Insert new client handle into the store.
    ///
    /// # Arguments
    ///
    /// * `client`: address of the client for which to insert the handle.
    /// * `handle`: the sender channel for all mix packets to be pushed back onto the websocket
    pub(crate) fn insert_remote(
        &self,
        client: DestinationAddressBytes,
        handle: MixMessageSender,
        is_active_request_sender: IsActiveRequestSender,
    ) {
        let entry = ActiveClient::Remote(ClientIncomingChannels {
            mix_message_sender: handle,
            is_active_request_sender,
        });
        if self.inner.insert(client, entry).is_some() {
            panic!("inserted a duplicate remote client")
        }
    }

    /// Inserts a handle to the embedded network requester
    pub(crate) fn insert_embedded(&self, local_nr_handle: LocalEmbeddedClientHandle) {
        let key = local_nr_handle.client_destination();
        let entry = ActiveClient::Embedded(local_nr_handle);
        if self.inner.insert(key, entry).is_some() {
            // this is literally impossible since we're starting local NR before even spawning the websocket listener task
            panic!("somehow we already had a client with the same address as our local NR!")
        }
    }

    /// Get number of active clients in store
    pub(crate) fn size(&self) -> usize {
        self.inner.len()
    }
}
