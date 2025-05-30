// Copyright 2021-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use super::websocket::message_receiver::{IsActiveRequestSender, MixMessageSender};
use crate::node::client_handling::embedded_clients::LocalEmbeddedClientHandle;
use dashmap::DashMap;
use nym_sphinx::DestinationAddressBytes;
use std::sync::Arc;
use time::OffsetDateTime;
use tracing::warn;

#[derive(Clone)]
pub(crate) struct RemoteClientData {
    // note, this does **NOT** indicate timestamp of when client connected
    // it is (for v2 auth) timestamp the client **signed** when it created the request
    pub(crate) session_request_timestamp: OffsetDateTime,
    pub(crate) channels: ClientIncomingChannels,
}

enum ActiveClient {
    /// Handle to a remote client connected via a network socket.
    Remote(RemoteClientData),

    /// Handle to a locally (inside the same process) running client.
    Embedded(Box<LocalEmbeddedClientHandle>),
}

impl ActiveClient {
    fn get_sender_ref(&self) -> &MixMessageSender {
        match self {
            ActiveClient::Remote(remote) => &remote.channels.mix_message_sender,
            ActiveClient::Embedded(embedded) => &embedded.mix_message_sender,
        }
    }

    fn get_sender(&self) -> MixMessageSender {
        match self {
            ActiveClient::Remote(remote) => remote.channels.mix_message_sender.clone(),
            ActiveClient::Embedded(embedded) => embedded.mix_message_sender.clone(),
        }
    }
}

#[derive(Clone, Default)]
pub struct ActiveClientsStore {
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
    pub fn new() -> Self {
        Self::default()
    }

    /// Tries to obtain sending channel to specified client. Note that if stale entry existed, it is
    /// removed and a `None` is returned instead.
    ///
    /// # Arguments
    ///
    /// * `client`: address of the client for which to obtain the handle.
    pub fn get_sender(&self, client: DestinationAddressBytes) -> Option<MixMessageSender> {
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
    ) -> Option<RemoteClientData> {
        let entry = self.inner.get(&address)?;
        let handle = entry.value();

        let ActiveClient::Remote(remote) = handle else {
            warn!("attempted to get a remote handle to a embedded network requester");
            return None;
        };

        // if the entry is stale, remove it from the map
        if !remote.channels.mix_message_sender.is_closed() {
            Some(remote.clone())
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
        session_request_timestamp: OffsetDateTime,
    ) {
        let entry = ActiveClient::Remote(RemoteClientData {
            session_request_timestamp,
            channels: ClientIncomingChannels {
                mix_message_sender: handle,
                is_active_request_sender,
            },
        });
        if self.inner.insert(client, entry).is_some() {
            panic!("inserted a duplicate remote client")
        }
    }

    /// Inserts a handle to the embedded client
    pub fn insert_embedded(&self, local_client_handle: LocalEmbeddedClientHandle) {
        let key = local_client_handle.client_destination();
        let entry = ActiveClient::Embedded(Box::new(local_client_handle));
        if self.inner.insert(key, entry).is_some() {
            // this is literally impossible since we're starting the local embedded client before
            // even spawning the websocket listener task
            panic!("somehow we already had a client with the same address as our local embedded client!")
        }
    }

    /// Get number of active clients in store
    #[allow(unused)]
    pub(crate) fn size(&self) -> usize {
        self.inner.len()
    }

    pub fn pending_packets(&self) -> usize {
        self.inner
            .iter()
            .map(|client| client.get_sender_ref().len())
            .sum()
    }
}
